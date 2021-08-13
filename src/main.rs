use ansi_term::Colour;
use argparse::{parse_args, parse_opts, Options};
use async_channel::Receiver as MPMCR;
use displace::displace;
use futures::future::{try_join3, try_join_all};
use input::{stream_input, Payload};
use output::stream_output;
use std::{error::Error, process::exit, sync::Arc, time::Duration};
use tokio::{
  runtime::Builder,
  select,
  sync::{
    broadcast,
    mpsc::{self, Receiver},
  },
  task::{spawn, JoinHandle},
};
use types::{Abort, Fail};

mod argparse;
mod displace;
mod errors;
mod fs_pipe;
mod fzf;
mod input;
mod output;
mod subprocess;
mod types;
mod udiff;

fn stream_trans(
  abort: &Abort,
  opts: &Options,
  stream: MPMCR<Payload>,
) -> (JoinHandle<()>, Receiver<String>) {
  let a_opts = Arc::new(opts.clone());
  let (tx, rx) = mpsc::channel::<String>(1);

  let handles = (1..=num_cpus::get() * 2)
    .map(|_| {
      let mut on_abort = abort.subscribe();
      let stream = stream.clone();
      let opts = a_opts.clone();
      let tx = tx.clone();

      spawn(async move {
        loop {
          select! {
            _ = on_abort.recv() => break,
            payload = stream.recv() => {
              match payload {
                Ok(p) => {
                  let displaced = displace(&opts, payload).await;
                  if let Err(err) = tx.send(displaced).await {
                    let _ = abort.send(Box::new(err));
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

  let handle = spawn(async move {
    match try_join_all(handles).await {
      Err(err) => {
        let _ = abort.send(Box::new(err));
      }
      _ => (),
    }
  });
  (handle, rx)
}

async fn run(abort: &Abort) -> Option<Fail> {
  let args = parse_args()?;
  let opts = parse_opts(args)?;
  let (h_1, input_stream) = stream_input(abort, &args);
  let (h_2, trans_stream) = stream_trans(abort, &opts, input_stream);
  let h_3 = stream_output(abort, opts, trans_stream);
  match try_join3(h_1, h_2, h_3).await {
    Err(err) => Some(err),
    _ => None,
  }
}

fn main() {
  let rt = Builder::new_multi_thread()
    .enable_io()
    .build()
    .expect("runtime failure");
  let status = rt.block_on(async {
    let (abort, mut rx) = broadcast::channel::<Fail>(1);
    select! {
      maybe = rx.recv() => match maybe {
        Ok(err) => Some(err),
        Err(RecvError::Lagged) => None,
        _ => None
      },
      maybe = run(&abort) => maybe
    }
  });
  rt.shutdown_timeout(Duration::from_secs(9001));

  if let Some(err) = status {
    eprintln!("{}", Colour::Red.paint(format!("{}", err)));
    exit(1)
  } else {
    exit(0)
  }
}
