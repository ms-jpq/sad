use argparse::{Arguments, Options};
use async_std::sync::{channel, Receiver, Sender};
use errors::*;
use futures::future::{try_join3, try_join_all, TryJoinAll};
use input::Payload;
use std::sync::Arc;
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
        while let Ok(path) = stream.recv().await {
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
    .collect::<Vec<_>>();
  let handle = try_join_all(handles);
  (handle, rx)
}

async fn run() -> SadResult<()> {
  let args = Arguments::new()?;
  let (reader, receiver) = args.stream();
  let opts = Options::new(args)?;
  let (steps, rx) = stream_process(opts.clone(), receiver);
  let writer = output::stream_output(opts, rx);
  try_join3(reader, steps, writer)
    .await
    .map(|_| ())
    .into_sadness()
}

fn main() {
  let mut rt = runtime::Builder::new()
    .threaded_scheduler()
    .enable_io()
    .build()
    .unwrap();
  rt.block_on(async {
    if let Err(err) = run().await {
      output::err_exit(err.into()).await
    }
  })
}

