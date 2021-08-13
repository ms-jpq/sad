use super::subprocess::SubprocessCommand;
use super::types::{Abort, Failure};
use futures::future::try_join;
use std::{
  collections::HashMap,
  env,
  error::Error,
  path::PathBuf,
  process:: Stdio,
};
use tokio::{
  io::{self, AsyncWriteExt, BufWriter},
  process::Command,
  select,
  sync::mpsc::Receiver,
  task::{spawn, JoinHandle},
};
use which::which;

async fn reset_term() -> Result<(), dyn Error> {
  try_join(io::stdout().flush(), io::stderr().flush()).map(|_| ()).await?;
  if let Ok(path) = which("tput") {
    Command::new("tput").arg("reset").status().await?
  } else if let Ok(path) = which("reset") {
    Command::new("reset").status().await?
  } else {
    Err(Failure::Sucks(String::new()))
  }
}

fn run_fzf(abort: &Abort, cmd: &SubprocessCommand, stream: Receiver<String>) -> JoinHandle<()> {
  let subprocess = Command::new(&cmd.program)
    .kill_on_drop(true)
    .args(&cmd.arguments)
    .envs(&cmd.env)
    .stdin(Stdio::piped())
    .spawn();

  spawn(async move {
    match subprocess {
      Err(err) => {
        let _ = abort.send(Box::new(err));
      }
      Ok(child) => {
        let mut stdin = child.stdin.take().map(BufWriter::new).expect("nil stdin");

        let handle_in = spawn(async move {
          let mut on_abort = abort.subscribe();
          loop {
            select! {
              _ = on_abort.recv() => break,
              print = stream.recv() => {
                match print {
                  Some(val) => {
                    if let Err(err) = stdin.write(val.as_bytes()).await {
                      let _ = abort.send(Box::new(err));
                      break;
                    }
                  }
                  _ => break
                }
              }
            }
          }
          if let Err(err) = stdin.shutdown().await {
            let _ = abort.send(Box::new(err));
          }
        });

        let handle_child = spawn(async move {
          let mut on_abort = abort.subscribe();
          select! {
            lhs = child.wait() => {
              match lhs {
                Ok(status) => {
                  match status.code() {
                    Some(0) | Some(1) | None => (),
                    Some(130) => {
                      let _ = abort.send(Box::new(Failure::Interrupt));
                    }
                    Some(c) => {
                      let _ = abort.send(Box::new(Failure::Sucks(format!("Error exit - {}", c))));
                      if let Err(err) = reset_term().await {
                        let _ = abort.send(err)
                      }
                    }
                  }
                }
                Err(err) => {
                  let _ = abort.send(Box::new(err));
                }
              }
            },
            _ = on_abort.recv() => {
              match child.kill().await {
                Err(err) => {
                  let _ = abort.send(err);
                },
                _ => {
                  if let Err(err) = reset_term().await {
                    let _ = abort.send(err)
                  }
                }
              }
            }
          }
        });

        spawn(async move {
          if let Err(err) = try_join(handle_child, handle_in).await {
            let _ = abort.send(Box::new(err));
          }
        })
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
    .unwrap_or("sad".to_owned());

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
    program: bin,
    arguments,
    env,
  };
  run_fzf(abort, &cmd, stream)
}
