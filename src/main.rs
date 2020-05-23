use argparse::{Arguments, Options};
use async_std::sync::{channel, Receiver, Sender};
use errors::*;
use futures::future::{try_join3, try_join_all, TryJoinAll};
use input::Payload;
use std::sync::Arc;
use structopt::StructOpt;
use tokio::{runtime, task};
use types::Task;

mod argparse;
mod displace;
mod errors;
mod input;
mod output;
mod subprocess;
mod types;
mod udiff;

fn stream_process(
  opts: Options,
  stream: Receiver<SadResult<Payload>>,
) -> (TryJoinAll<Task>, Receiver<SadResult<String>>) {
  let oo = Arc::new(opts);
  let (tx, rx) = channel::<SadResult<String>>(1);

  let handles = (1..=num_cpus::get() * 2)
    .map(|_| {
      let stream = Receiver::clone(&stream);
      let opts = Arc::clone(&oo);
      let sender = Sender::clone(&tx);

      task::spawn(async move {
        while let Some(path) = stream.recv().await {
          match path {
            Ok(val) => {
              let displaced = displace::displace(&opts, val).await;
              sender.send(displaced).await
            }
            Err(err) => sender.send(Err(err)).await,
          }
        }
      })
    })
    .collect::<Vec<Task>>();
  let handle = try_join_all(handles);
  (handle, rx)
}

fn main() {
  let mut rt = runtime::Builder::new()
    .threaded_scheduler()
    .enable_io()
    .build()
    .unwrap();
  rt.block_on(async {
    let args = Arguments::from_args();
    let (reader, receiver) = args.stream();
    let end = match Options::new(args) {
      Ok(opts) => {
        let pager = opts.pager.clone();
        let (steps, rx) = stream_process(opts, receiver);
        let writer = output::stream_output(pager, rx);
        try_join3(reader, steps, writer).await
      }
      Err(e) => output::err_exit(e),
    };
    if let Err(err) = end {
      output::err_exit(err.into())
    }
  })
}

/* Exit */
