use std::collections::BTreeMap;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crossbeam_utils::thread::scope;
use nix::sys::signal::{kill, Signal};
use nix::unistd::setpgid;

use crate::events::{Event, EventKind, EventReceiver, EventSender, StreamKind};
use crate::spec::{AppInfo, Spec};
use crate::utils::Pid;

fn read_line<R>(reader: &mut BufReader<R>, buf: &mut Vec<u8>) -> io::Result<usize>
where
    R: Read,
{
    buf.clear();
    let n = reader.read_until(b'\n', buf)?;

    if buf.ends_with(b"\r\n") {
        buf.truncate(buf.len() - 2);
    }

    if buf.ends_with(b"\n") {
        buf.truncate(buf.len() - 1);
    }

    Ok(n)
}

fn stream_handler<R>(app: Arc<AppInfo>, sender: EventSender, stream: R, stream_kind: StreamKind)
where
    R: Read,
{
    let mut reader = BufReader::new(stream);
    loop {
        let mut line = Vec::with_capacity(512);
        match read_line(&mut reader, &mut line) {
            Ok(0) => {
                let _ = sender.send(Event::new(&app, EventKind::Eof(stream_kind)));
                break;
            }
            Ok(_) => {
                if sender
                    .send(Event::new(&app, EventKind::Line(stream_kind, line)))
                    .is_err()
                {
                    break;
                }
            }
            Err(err) => {
                let _ = sender.send(Event::new(&app, EventKind::Err(stream_kind, err)));
                break;
            }
        }
    }
}

fn spawn_thread(app: Arc<AppInfo>, sender: EventSender, delay: Duration) {
    thread::sleep(delay);

    let stdout = if app.stdout { Stdio::piped() } else { Stdio::null() };
    let stderr = if app.stderr { Stdio::piped() } else { Stdio::null() };

    let mut proc = unsafe {
        match Command::new(&app.exec)
            .args(&app.args)
            .stdout(stdout)
            .stderr(stderr)
            .current_dir(&app.workdir)
            .pre_exec(|| {
                setpgid(Pid(0).to_nix(), Pid(0).to_nix()); // TODO handle error
                Ok(())
            })
            .spawn()
        {
            Ok(proc) => {
                let _ = sender.send(Event::new(&app, EventKind::Started(proc.id().into())));
                proc
            }
            Err(err) => {
                let _ = sender.send(Event::new(&app, EventKind::SpawnError(err)));
                return;
            }
        }
    };

    let _ = scope(|s| {
        if let Some(stdout) = proc.stdout.take() {
            s.spawn(|_| {
                stream_handler(app.clone(), sender.clone(), stdout, StreamKind::Stdout);
            });
        }

        if let Some(stderr) = proc.stderr.take() {
            s.spawn(|_| {
                stream_handler(app.clone(), sender.clone(), stderr, StreamKind::Stderr);
            });
        }
    });
}

fn spawn(app: Arc<AppInfo>, sender: EventSender, delay: Duration) {
    thread::spawn(move || {
        spawn_thread(app, sender, delay);
    });
}

fn write_app_line_to_stream<W>(mut writer: W, app: &AppInfo, line: &[u8]) -> io::Result<()>
where
    W: Write,
{
    write!(writer, "[{}] ", app.name)?;
    writer.write_all(&line)?;
    writer.write_all(&b"\n"[..])?;
    writer.flush()?;
    Ok(())
}

pub struct Reactor {
    stdout: io::Stdout,
    stderr: io::Stderr,
    sender: EventSender,
    receiver: Option<EventReceiver>,
    processes: BTreeMap<Pid, Arc<AppInfo>>,
    shutdown_requested: bool,
}

impl Reactor {
    pub fn new(sender: EventSender, receiver: EventReceiver) -> Reactor {
        Reactor {
            stdout: io::stdout(),
            stderr: io::stderr(),
            sender: sender,
            receiver: Some(receiver),
            processes: BTreeMap::new(),
            shutdown_requested: false,
        }
    }

    fn log<A>(&mut self, log: A)
    where
        A: AsRef<str>,
    {
        let _ = write!(self.stderr, "[supermon] {}\n", log.as_ref());
    }

    fn log_app_line(&mut self, app: &AppInfo, stream_kind: StreamKind, line: &[u8]) {
        let _ = match stream_kind {
            StreamKind::Stdout => write_app_line_to_stream(&mut self.stdout, &app, line),
            StreamKind::Stderr => write_app_line_to_stream(&mut self.stderr, &app, line),
        };
    }

    fn start_app<D>(&self, app: &Arc<AppInfo>, delay: D)
    where
        D: Into<Option<Duration>>,
    {
        if !app.disable && !self.shutdown_requested {
            spawn(
                app.clone(),
                self.sender.clone(),
                delay.into().unwrap_or(Duration::from_secs(0)),
            );
        }
    }

    fn restart_app(&mut self, app: &Arc<AppInfo>) {
        if app.restart && !self.shutdown_requested {
            self.log(format!("restarting app {} in {} sec(s)", app.name, app.restart_delay));
            self.start_app(app, Some(Duration::from_secs(app.restart_delay as u64)));
        }
    }

    fn initialize(&self, spec: Spec) {
        for (name, app_spec) in spec.apps.into_iter() {
            let app = Arc::new(AppInfo::new(name, app_spec));
            self.start_app(&app, None);
        }
    }

    fn shutdown(&mut self, signal: Signal) {
        self.shutdown_requested = true;

        for pid in self.processes.keys() {
            kill(pid.to_nix(), signal);
        }
    }

    fn can_exit(&self) -> bool {
        println!("can exit {:?} {}", self.shutdown_requested, self.processes.len());
        self.shutdown_requested && self.processes.len() == 0
    }

    fn handle_app_event(&mut self, app: &Arc<AppInfo>, kind: EventKind) {
        match kind {
            EventKind::Line(stream_kind, line) => self.log_app_line(&app, stream_kind, &line),
            EventKind::Started(pid) => {
                self.processes.insert(pid, app.clone());
                self.log(format!("{} spawned with pid {}", app.name, pid));
            }
            EventKind::SpawnError(err) => {
                self.log(format!("Error spawning app {}: {}", app.name, err));
            }
            _ => {}
        }
    }

    pub fn run(mut self, spec: Spec) {
        self.initialize(spec);

        for event in self.receiver.take().unwrap() {
            match event {
                Event::App { app, kind } => self.handle_app_event(&app, kind),
                Event::Signal(signal) => {
                    self.shutdown(signal);
                }
                Event::Exited(pid, code) => {
                    if let Some(app) = self.processes.get(&pid).map(|x| x.clone()) {
                        self.log(format!("{} has exited with code {}", app.name, code));
                        self.restart_app(&app);
                    } else {
                        self.log(format!("zombie {} has been reaped", pid));
                    }

                    // Try to remove the PID from the process table to prevent overgrowth.
                    self.processes.remove(&pid);
                }
                Event::Signaled(pid, _) => {
                    if let Some(app) = self.processes.get(&pid).map(|x| x.clone()) {
                        self.restart_app(&app);
                    }

                    // Try to remove the PID from the process table to prevent overgrowth.
                    self.processes.remove(&pid);
                }
            }

            if self.can_exit() {
                break;
            }
        }
    }

    // TODO on exit, reap zombies by calling wait, but do not block
}
