use super::subprocess::SubprocessCommand;
use super::types::{Abort, Fail};
use futures::future::try_join;
use std::{collections::HashMap, env, path::PathBuf, process::Stdio};
use tokio::{
  io::{AsyncWriteExt, BufWriter, ErrorKind},
  process::Command,
  select,
  sync::mpsc::Receiver,
  task::{spawn, JoinHandle},
};
use which::which;

async fn reset_term() -> Result<(), Fail> {
  if let Ok(path) = which("tput") {
    let status = Command::new(&path)
      .kill_on_drop(true)
      .stdin(Stdio::null())
      .arg("reset")
      .status()
      .await
      .map_err(|e| Fail::IO(path, e.kind()))?;

    if status.success() {
      return Ok(());
    }
  }
  if let Ok(path) = which("reset") {
    let status = Command::new(&path)
      .kill_on_drop(true)
      .stdin(Stdio::null())
      .status()
      .await
      .map_err(|e| Fail::IO(path, e.kind()))?;
    if status.success() {
      return Ok(());
    }
  }
  Err(Fail::IO(PathBuf::from("reset"), ErrorKind::NotFound))
}

fn run_fzf(abort: &Abort, cmd: SubprocessCommand, mut stream: Receiver<String>) -> JoinHandle<()> {
  let abort = abort.clone();

  spawn(async move {
    let subprocess = Command::new(&cmd.prog)
      .kill_on_drop(true)
      .args(&cmd.args)
      .envs(&cmd.env)
      .stdin(Stdio::piped())
      .spawn();

    match subprocess {
      Err(err) => {
        abort.send(Fail::IO(cmd.prog, err.kind())).expect("<ABORT CH OPEN>");
      }
      Ok(mut child) => {
        let mut stdin = child.stdin.take().map(BufWriter::new).expect("nil stdin");

        let abort_1 = abort.clone();
        let p1 = cmd.prog.clone();
        let handle_in = spawn(async move {
          let mut on_abort = abort_1.subscribe();
          loop {
            select! {
              _ = on_abort.recv() => break,
              print = stream.recv() => {
                match print {
                  Some(val) => {
                    if let Err(err) = stdin.write(val.as_bytes()).await {
                      let _ = abort_1.send(Fail::IO(p1.clone(),err.kind()));
                      break;
                    }
                  }
                  _ => break
                }
              }
            }
          }
          if let Err(err) = stdin.shutdown().await {
            let _ = abort_1.send(Fail::IO(p1, err.kind()));
          }
        });

        let abort_2 = abort.clone();
        let p2 = cmd.prog.clone();
        let handle_child = spawn(async move {
          let mut on_abort = abort_2.subscribe();
          select! {
            lhs = child.wait() => {
              match lhs {
                Ok(status) => {
                  match status.code() {
                    Some(0) | Some(1) | None => (),
                    Some(130) => {
                      let _ = abort_2.send(Fail::Interrupt);
                    }
                    Some(c) => {
                      let _ = abort_2.send(Fail::BadExit(p2, c));
                      if let Err(err) = reset_term().await {
                        let _ = abort_2.send(err);
                      }
                    }
                  }
                }
                Err(err) => {
                  let _ = abort_2.send(Fail::IO(p2, err.kind()));
                }
              }
            },
            _ = on_abort.recv() => {
              match child.kill().await {
                Err(err) => {
                  let _ = abort_2.send(Fail::IO(p2, err.kind()));
                },
                _ => {
                  if let Err(err) = reset_term().await {
                    let _ = abort_2.send(err);
                  }
                }
              }
            }
          }
        });

        if let Err(err) = try_join(handle_child, handle_in).await {
          abort.send(err.into()).expect("<ABORT CH OPEN>");
        }
      }
    }
  })
}

pub fn stream_fzf(
  abort: &Abort,
  bin: PathBuf,
  args: Vec<String>,
  stream: Receiver<String>,
) -> JoinHandle<()> {
  let sad = env::current_exe()
    .or_else(|_| which("sad".to_owned()))
    .map(|p| format!("{}", p.display()))
    .unwrap_or_else(|_| "sad".to_owned());

  let preview_args = env::args().skip(1).collect::<Vec<_>>().join("\x04");
  let execute = format!(
    "abort+execute:{}\x04--internal-patch\x04{{+f}}\x04{}",
    sad, preview_args
  );
  let mut arguments = vec![
    "--read0".to_owned(),
    "--print0".to_owned(),
    "-m".to_owned(),
    "--ansi".to_owned(),
    format!("--bind=enter:{}", execute),
    format!("--bind=double-click:{}", execute),
    format!(
      "--preview={}\x04--internal-preview\x04{{f}}\x04{}",
      sad, preview_args
    ),
    "--preview-window=70%:wrap".to_owned(),
  ];
  arguments.extend(args);
  let mut env = HashMap::new();
  env.insert("SHELL".to_owned(), sad);
  let cmd = SubprocessCommand {
    prog: bin,
    args: arguments,
    env,
  };
  run_fzf(abort, cmd, stream)
}
