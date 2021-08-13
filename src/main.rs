use ansi_term::Colour;
use argparse::{parse_args, parse_opts, Options};
use async_channel::Receiver as MPMCR;
use displace::displace;
use futures::future::{try_join3, try_join_all};
use input::{stream_input, Payload};
use output::stream_output;
use std::{process::exit, sync::Arc};
use tokio::{
  runtime::Builder,
  select,
  sync::mpsc::{self, Receiver},
  task::{spawn, JoinHandle},
};
use types::{Abort, Fail};

mod argparse;
mod displace;
mod fs_pipe;
mod fzf;
mod input;
mod output;
mod subprocess;
mod types;
mod udiff;

fn stream_trans(
  abort: &Arc<Abort>,
  cpus: usize,
  opts: &Options,
  stream: MPMCR<Payload>,
) -> (JoinHandle<()>, Receiver<String>) {
  let a_opts = Arc::new(opts.clone());
  let (tx, rx) = mpsc::channel::<String>(1);

  let handles = (1..=cpus * 2)
    .map(|_| {
      let abort = abort.clone();
      let stream = stream.clone();
      let opts = a_opts.clone();
      let tx = tx.clone();

      spawn(async move {
        loop {
          select! {
            _ = abort.notified() => break,
            payload = stream.recv() => {
              match payload {
                Ok(p) => {
                  match displace(&opts, p).await {
                    Ok(displaced) => {
                      if tx.send(displaced).await.is_err() {
                        break;
                      }
                    },
                    Err(err) => {
                      abort.send(err).await;
                    }
                  }
                },
                _ => break
              }
            }
          }
        }
      })
    })
    .collect::<Vec<_>>();

  let abort = abort.clone();
  let handle = spawn(async move {
    if let Err(err) = try_join_all(handles).await {
      abort.send(err.into()).await;
    }
  });
  (handle, rx)
}

async fn run(abort: &Arc<Abort>, cpus: usize) -> Result<(), Fail> {
  let args = parse_args()?;
  let (h_1, input_stream) = stream_input(abort, &args);
  let opts = parse_opts(args)?;
  let (h_2, trans_stream) = stream_trans(abort, cpus, &opts, input_stream);
  let h_3 = stream_output(abort, &opts, trans_stream);
  try_join3(h_1, h_2, h_3).await?;
  Ok(())
}

fn main() {
  let cpus = num_cpus::get();
  let rt = Builder::new_multi_thread()
    .enable_io()
    .max_blocking_threads(cpus)
    .build()
    .expect("runtime failure");

  let errors = rt.block_on(async {
    let abort = Abort::new();
    if let Err(err) = run(&abort, cpus).await {
      let mut errs = abort.fin().await;
      errs.push(err);
      errs
    } else {
      abort.fin().await
    }
  });

  match errors[..] {
    [] => exit(0),
    [Fail::Interrupt] => exit(130),
    _ => {
      for err in errors {
        eprintln!("{}", Colour::Red.paint(format!("{}", err)));
      }
      exit(1)
    }
  }
}
