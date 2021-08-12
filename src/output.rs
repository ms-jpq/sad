use super::argparse::{Action, Options, Printer};
use super::errors::{Failure, SadResult};
use super::fzf::run_fzf;
use super::types::Task;
use ansi_term::Colour;
use async_channel::Receiver;
use futures::future::try_join;
use std::process;
use tokio::{
  io::{self, AsyncWriteExt, BufWriter},
  task,
};

fn stream_stdout(stream: Receiver<SadResult<String>>) -> Task {
  let mut stdout = BufWriter::new(io::stdout());
  task::spawn(async move {
    while let Ok(print) = stream.recv().await {
      match print {
        Ok(val) => match stdout.write(val.as_bytes()).await {
          Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => {
            err_exit(Failure::Interrupt).await;
            break;
          }
          Err(e) => {
            err_exit(e.into()).await;
            break;
          }
          _ => {}
        },
        Err(e) => {
          err_exit(e).await;
          break;
        }
      }
    }
    stdout.shutdown().await.expect("shutdown failure")
  })
}

pub fn stream_output(opts: Options, stream: Receiver<SadResult<String>>) -> Task {
  match (&opts.action, &opts.printer) {
    (Action::Fzf(fzf_p, fzf_a), _) => {
      let (child, rx) = run_fzf(fzf_p.to_owned(), fzf_a.to_owned(), stream);
      let recv = stream_stdout(rx);
      task::spawn(async {
        if let Err(e) = try_join(child, recv).await {
          err_exit(e.into()).await
        }
      })
    }
    (_, Printer::Pager(cmd)) => {
      let (child, rx) = cmd.stream(stream);
      let recv = stream_stdout(rx);
      task::spawn(async {
        if let Err(e) = try_join(child, recv).await {
          err_exit(e.into()).await
        }
      })
    }
    (_, Printer::Stdout) => stream_stdout(stream),
  }
}

pub async fn err_exit(err: Failure) -> ! {
  if let Some(msg) = err.exit_message() {
    eprintln!("{}", Colour::Red.paint(msg));
  }
  process::exit(err.exit_code())
}
