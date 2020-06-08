use super::errors::*;
use super::types::Task;
use async_std::sync::{channel, Receiver, Sender};
use futures::future::{select, try_join, try_join4, Either};
use std::{collections::HashMap, process::Stdio};
use tokio::{
  io::{self, AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, BufWriter},
  process::Command,
  task,
};

#[derive(Clone, Debug)]
pub struct SubprocessCommand {
  pub program: String,
  pub arguments: Vec<String>,
  pub env: HashMap<String, String>,
}

impl SubprocessCommand {
  pub fn stream(&self, stream: Receiver<SadResult<String>>) -> (Task, Receiver<SadResult<String>>) {
    let (tx, rx) = channel::<SadResult<String>>(1);
    let to = Sender::clone(&tx);
    let te = Sender::clone(&tx);
    let tt = Sender::clone(&tx);
    let ta = Sender::clone(&tx);

    let subprocess = Command::new(&self.program)
      .kill_on_drop(true)
      .args(&self.arguments)
      .envs(&self.env)
      .kill_on_drop(true)
      .stdin(Stdio::piped())
      .stdout(Stdio::piped())
      .stderr(Stdio::piped())
      .spawn();

    let mut child = match subprocess.into_sadness() {
      Ok(child) => child,
      Err(err) => {
        let handle = task::spawn(async move { tx.send(Err(err)).await });
        return (handle, rx);
      }
    };

    let mut stdin = child.stdin.take().map(BufWriter::new).unwrap();
    let mut stdout = child.stdout.take().map(BufReader::new).unwrap();
    let mut stderr = child.stderr.take().map(BufReader::new).unwrap();

    let handle_in = task::spawn(async move {
      while let Ok(print) = stream.recv().await {
        match print {
          Ok(val) => {
            if let Err(err) = stdin.write(val.as_bytes()).await.into_sadness() {
              tx.send(Err(err)).await;
            }
          }
          Err(err) => tx.send(Err(err)).await,
        }
      }
      if let Err(err) = stdin.shutdown().await {
        tx.send(Err(err.into())).await;
      }
    });

    let handle_out = task::spawn(async move {
      loop {
        let mut buf = String::new();
        match stdout.read_line(&mut buf).await.into_sadness() {
          Ok(0) => return,
          Ok(_) => {
            to.send(Ok(buf)).await;
          }
          Err(err) => to.send(Err(err)).await,
        }
      }
    });

    let handle_err = task::spawn(async move {
      let mut buf = String::new();
      match stderr.read_to_string(&mut buf).await.into_sadness() {
        Err(err) => {
          te.send(Err(err)).await;
        }
        Ok(_) => {
          if !buf.is_empty() {
            te.send(Err(Failure::Pager(buf))).await
          }
        }
      }
    });

    let handle_child = task::spawn(async move {
      if let Err(err) = child.await {
        tt.send(Err(err.into())).await;
      }
    });

    let handle = task::spawn(async move {
      if let Err(err) = try_join4(handle_child, handle_in, handle_out, handle_err).await {
        ta.send(Err(err.into())).await;
      }
    });

    (handle, rx)
  }

  pub fn stream_connected(
    &self,
    stream: Receiver<SadResult<String>>,
  ) -> (Task, Receiver<SadResult<String>>) {
    let (tx, rx) = channel::<SadResult<String>>(1);
    let (tix, rix) = channel::<Failure>(1);
    let ta = Sender::clone(&tx);

    let subprocess = Command::new(&self.program)
      .args(&self.arguments)
      .envs(&self.env)
      .kill_on_drop(true)
      .stdin(Stdio::piped())
      .stdout(Stdio::inherit())
      .stderr(Stdio::inherit())
      .spawn();

    let mut child = match subprocess.into_sadness() {
      Ok(child) => child,
      Err(err) => {
        let handle = task::spawn(async move { tx.send(Err(err)).await });
        return (handle, rx);
      }
    };

    let mut stdin = child.stdin.take().map(BufWriter::new).unwrap();

    let handle_in = task::spawn(async move {
      while let Ok(print) = stream.recv().await {
        match print {
          Ok(val) => {
            if let Err(err) = stdin.write(val.as_bytes()).await.into_sadness() {
              tix.send(err).await;
            }
          }
          Err(err) => tix.send(err).await,
        }
      }
      if let Err(err) = stdin.shutdown().await {
        tix.send(err.into()).await;
      }
    });

    let handle_kill = task::spawn(async move {
      match rix.recv().await {
        Ok(err) => Some(err),
        Err(_) => None,
      }
    });

    let handle_child = task::spawn(async move {
      match select(child, handle_kill).await {
        Either::Left((Ok(status), _)) => process_status_code(status.code(), tx).await,
        Either::Left((Err(err), _)) => tx.send(Err(err.into())).await,
        Either::Right((handle, mut child)) => {
          let maybe_failure = match handle.into_sadness() {
            Ok(err) => err,
            Err(err) => Some(err),
          };
          match maybe_failure {
            Some(err) => {
              let err = combine_err(err, child.kill().into_sadness());
              let err = combine_err(err, child.await.into_sadness());
              let err = combine_err(err, reset_term().await);
              tx.send(Err(err)).await
            }
            None => match child.await.into_sadness() {
              Err(err) => tx.send(Err(err)).await,
              Ok(status) => process_status_code(status.code(), tx).await,
            },
          }
        }
      }
    });

    let handle = task::spawn(async move {
      if let Err(err) = try_join(handle_child, handle_in).await {
        ta.send(Err(err.into())).await;
      }
    });

    (handle, rx)
  }
}

fn combine_err<T>(err: Failure, res: SadResult<T>) -> Failure {
  match res {
    Ok(_) => err,
    Err(e) => Failure::Compound(Box::new(err), Box::new(e)),
  }
}

async fn process_status_code(code: Option<i32>, tx: Sender<SadResult<String>>) {
  match code {
    Some(0) | Some(1) | None => {}
    Some(130) => tx.send(Err(Failure::Interrupt)).await,
    Some(c) => {
      tx.send(Err(Failure::Fzf(format!("Error exit - {}", c))))
        .await
    }
  }
}

async fn reset_term() -> SadResult<()> {
  io::stdout().flush().await.into_sadness()?;
  io::stderr().flush().await.into_sadness()?;
  if which::which("tput").is_ok() {
    Command::new("tput")
      .arg("reset")
      .status()
      .await
      .into_sadness()?;
  } else if which::which("reset").is_ok() {
    Command::new("reset").status().await.into_sadness()?;
  } else {
    return Err(Failure::Fzf("Unable to clear screen".to_owned()));
  };
  Ok(())
}

