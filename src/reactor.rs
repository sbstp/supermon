use std::io::{self, BufRead, BufReader, Read, StderrLock, StdoutLock, Write};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crossbeam_channel::bounded;
use crossbeam_utils::thread::scope;

use crate::events::{Event, EventKind, EventReceiver, EventSender, StreamKind};
use crate::spec::{AppInfo, Spec};

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

    let mut proc = match Command::new(&app.exec)
        .args(&app.args)
        .stdout(stdout)
        .stderr(stderr)
        .current_dir(&app.workdir)
        .spawn()
    {
        Ok(proc) => proc,
        Err(err) => {
            let _ = sender.send(Event::new(&app, EventKind::SpawnError(err)));
            return;
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

        let _ = match proc.wait() {
            Ok(status) => sender.send(Event::new(&app, EventKind::Exit(status))),
            Err(err) => sender.send(Event::new(&app, EventKind::WaitError(err))),
        };
    });
}

fn spawn(app: Arc<AppInfo>, sender: EventSender, delay: Duration) {
    thread::spawn(move || {
        spawn_thread(app, sender, delay);
    });
}

struct Logger<'o, 'e> {
    stdout: StdoutLock<'o>,
    stderr: StderrLock<'e>,
}

impl<'o, 'e> Logger<'o, 'e> {
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

    fn log_app_line(&mut self, app: &AppInfo, stream_kind: StreamKind, line: &[u8]) {
        let _ = match stream_kind {
            StreamKind::Stdout => Logger::write_app_line_to_stream(&mut self.stdout, &app, line),
            StreamKind::Stderr => Logger::write_app_line_to_stream(&mut self.stderr, &app, line),
        };
    }

    fn log_msg<A>(&mut self, msg: A)
    where
        A: AsRef<str>,
    {
        let _ = writeln!(self.stderr, "[supermon] {}", msg.as_ref());
    }
}

pub fn run(spec: Spec) {
    let (sender, receiver): (EventSender, EventReceiver) = bounded(128);

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

    let mut logger = Logger {
        stdout: stdout.lock(),
        stderr: stderr.lock(),
    };

    for event in receiver {
        let app = event.app;
        match event.kind {
            EventKind::Line(stream_kind, line) => logger.log_app_line(&app, stream_kind, &line),
            EventKind::Exit(status) => {
                let _ = match status.code() {
                    Some(code) => logger.log_msg(format!("[supermon] {} has exited with code {}", app.name, code)),
                    None => logger.log_msg(format!("[supermon] {} has exited from a signal", app.name)),
                };

                if app.restart {
                    logger.log_msg(format!(
                        "[supermon] restarting app {} in {} sec(s)",
                        app.name, app.restart_delay
                    ));
                    spawn(
                        app.clone(),
                        sender.clone(),
                        Duration::from_secs(app.restart_delay as u64),
                    );
                }
            }
            EventKind::SpawnError(err) => {
                logger.log_msg(format!("[supermon] Error spawning app {}: {}", app.name, err));
            }
            _ => {}
        }
    }
}
