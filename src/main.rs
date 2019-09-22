mod spec;

use std::fs::File;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crossbeam_channel::{bounded, Receiver, Sender};
use serde_yaml;

use crate::spec::{AppInfo, Spec};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum StreamKind {
    Stdout,
    Stderr,
}

#[derive(Debug)]
enum EventKind {
    Exit(ExitStatus),
    SpawnError(io::Error),
    WaitError(io::Error),
    Line(StreamKind, Vec<u8>),
    Err(StreamKind, io::Error),
    EOF(StreamKind),
}

#[derive(Debug)]
struct Event {
    app: Arc<AppInfo>,
    kind: EventKind,
}

impl Event {
    pub fn new(app: &Arc<AppInfo>, kind: EventKind) -> Event {
        Event {
            app: app.clone(),
            kind: kind,
        }
    }
}

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

fn stream_handler<R>(app: Arc<AppInfo>, sender: Sender<Event>, stream: R, stream_kind: StreamKind)
where
    R: Read,
{
    let mut reader = BufReader::new(stream);
    loop {
        let mut line = Vec::with_capacity(512);
        match read_line(&mut reader, &mut line) {
            Ok(0) => {
                let _ = sender.send(Event::new(&app, EventKind::EOF(stream_kind)));
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

fn spawn(prog: Arc<AppInfo>, sender: Sender<Event>, delay: Duration) {
    thread::spawn(move || {
        thread::sleep(delay);

        let stdout = if prog.stdout { Stdio::piped() } else { Stdio::null() };
        let stderr = if prog.stderr { Stdio::piped() } else { Stdio::null() };

        let mut proc = match Command::new(&prog.exec)
            .args(&prog.args)
            .stdout(stdout)
            .stderr(stderr)
            .spawn()
        {
            Ok(proc) => proc,
            Err(err) => {
                let _ = sender.send(Event::new(&prog, EventKind::SpawnError(err)));
                return;
            }
        };

        if let Some(stdout) = proc.stdout.take() {
            let prog = prog.clone();
            let sender = sender.clone();
            thread::spawn(move || {
                stream_handler(prog, sender, stdout, StreamKind::Stdout);
            });
        }

        if let Some(stderr) = proc.stderr.take() {
            let prog = prog.clone();
            let sender = sender.clone();
            thread::spawn(move || {
                stream_handler(prog, sender, stderr, StreamKind::Stderr);
            });
        }

        let _ = match proc.wait() {
            Ok(status) => sender.send(Event::new(&prog, EventKind::Exit(status))),
            Err(err) => sender.send(Event::new(&prog, EventKind::WaitError(err))),
        };
    });
}

fn write_log_entry<W>(mut writer: W, app: &AppInfo, line: &[u8]) -> io::Result<()>
where
    W: Write,
{
    write!(writer, "[{}] ", app.name)?;
    writer.write_all(&line)?;
    writer.write_all(&b"\n"[..])?;
    writer.flush()?;
    Ok(())
}

fn main() {
    let spec_path = std::env::args_os().nth(1).expect("first argument must be spec path");
    let file = File::open(spec_path).expect("unable to open spec for reading");
    let reader = BufReader::new(file);
    let spec: Spec = serde_yaml::from_reader(reader).expect("invalid spec");

    println!("{:#?}", spec);

    let (sender, receiver): (Sender<Event>, Receiver<Event>) = bounded(128);

    let mut apps = Vec::new();

    for (name, app_spec) in spec.apps.into_iter() {
        apps.push(Arc::new(AppInfo::new(name, app_spec)));
    }

    for app in apps {
        if !app.disable {
            spawn(app, sender.clone(), Duration::from_secs(0));
        }
    }

    let stdout = std::io::stdout();
    let stderr = std::io::stderr();
    let mut stdout_lock = stdout.lock();
    let mut stderr_lock = stderr.lock();

    for msg in receiver {
        // println!("{:?}", msg);
        match msg.kind {
            EventKind::Line(stream, line) => match stream {
                StreamKind::Stdout => {
                    let _ = write_log_entry(&mut stdout_lock, &msg.app, &line);
                }
                StreamKind::Stderr => {
                    let _ = write_log_entry(&mut stderr_lock, &msg.app, &line);
                }
            },
            EventKind::Exit(status) => {
                match status.code() {
                    Some(code) => eprintln!("[supermon] {} has exited with code {}", msg.app.name, code),
                    None => eprintln!("[supermon] {} has exited from a signal", msg.app.name),
                };

                if msg.app.restart {
                    eprintln!("[supermon] restarting app {} in {} sec(s)", msg.app.name, msg.app.restart_delay);
                    spawn(
                        msg.app.clone(),
                        sender.clone(),
                        Duration::from_secs(msg.app.restart_delay as u64),
                    );
                }
            }
            EventKind::SpawnError(err) => {
                eprintln!("[supermon] Error spawning app {}: {}", msg.app.name, err);
            }
            _ => {}
        }
    }
}
