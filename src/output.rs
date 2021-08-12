use super::argparse::{Action, Options, Printer};
use super::fzf::run_fzf;
use super::types::{Abort, Task};
use async_channel::Receiver;
use std::sync::Arc;
use tokio::{
  io::{self, AsyncWriteExt, BufWriter},
  select, task,
};

fn stream_stdout(abort: &Arc<Abort>, stream: Receiver<String>) -> Task {
  let mut stdout = BufWriter::new(io::stdout());
  task::spawn(async move {
    loop {
      select! {
        _ = abort.rx.changed() => {
          break
        },
        print = stream.recv() => {
        match print {
          Ok(val) => match stdout.write(val.as_bytes()).await {
            Err(err) => {
              abort.tx.send(Box::new(err)).expect("<CHANNEL>");
              break;
            }
            _ => {}
          },
          Err(err) => {
            abort.tx.send(Box::new(err)).expect("<CHANNEL>");
            break;
          }
        }
        }
      }
    }
  })
}

pub fn stream_output(abort: &Arc<Abort>, opts: Options, stream: Receiver<String>) -> Task {
  match (&opts.action, &opts.printer) {
    (Action::Fzf(fzf_p, fzf_a), _) => run_fzf(abort, fzf_p.to_owned(), fzf_a.to_owned(), stream),
    (_, Printer::Pager(cmd)) => cmd.stream(abort, stream),
    (_, Printer::Stdout) => stream_stdout(abort, stream),
  }
}
