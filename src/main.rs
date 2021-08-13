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
  sync::{
    broadcast::{self, error::RecvError},
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
      let abort = abort.clone();
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
                  match displace(&opts, p).await {
                    Ok(displaced) => {
                      if tx.send(displaced).await.is_err() {
                        let _ = abort.send(Fail::Join);
                        break;
                      }
                    },
                    Err(err) => {
                      let _ = abort.send(err);
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
      if !err.is_cancelled() {
        let _ = abort.send(Fail::Join);
      }
    }
  });
  (handle, rx)
}

async fn run(abort: &Abort) -> Result<(), Fail> {
  let args = parse_args()?;
  let (h_1, input_stream) = stream_input(abort, &args);
  let opts = parse_opts(args)?;
  let (h_2, trans_stream) = stream_trans(abort, &opts, input_stream);
  let h_3 = stream_output(abort, opts, trans_stream);
  match try_join3(h_1, h_2, h_3).await {
    Err(err) if err.is_cancelled() => Ok(()),
    Err(_) => Err(Fail::Join),
    _ => Ok(()),
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
        Err(RecvError::Lagged(_)) => None,
        _ => None
      },
      maybe = run(&abort) => match maybe {
        Ok(_) => None,
        Err(err) => Some(err)
      }
    }
  });
  drop(rt);

  match status {
    Some(Fail::Interrupt) => exit(130),
    Some(err) => {
      eprintln!("{}", Colour::Red.paint(format!("{}", err)));
      exit(1)
    }
    None => exit(0),
  }
}
