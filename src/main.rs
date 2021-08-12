use argparse::{Arguments, Options};
use async_channel::{bounded, Receiver, Sender};
use displace::displace;
use errors::Failure;
use futures::future::{try_join3, try_join_all, TryJoinAll};
use input::Payload;
use output::stream_output;
use std::{sync::Arc, time::Duration};
use tokio::{runtime::Builder, select, sync::watch, task};
use types::{Abort, Task};

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

fn stream_process(
  abort: Abort,
  opts: Options,
  stream: Receiver<Payload>,
) -> (TryJoinAll<Task>, Receiver<String>) {
  let oo = Arc::new(opts);
  let (tx, rx) = bounded::<String>(1);

  let handles = (1..=num_cpus::get() * 2)
    .map(|_| {
      let stream = Receiver::clone(&stream);
      let opts = Arc::clone(&oo);
      let sender = Sender::clone(&tx);

      task::spawn(async move {
        loop {
          select! {
            _ = abort.rx.changed() => break,
            payload = stream.recv() => {
              match payload {
                Ok(p) => {

                let displaced = displace(&opts, payload).await;
                sender.send(displaced).await.expect("<CHANNEL>")
                },
                _ => break
              }
            }
          }
        }
      })
    })
    .collect::<Vec<_>>();
  let handle = try_join_all(handles);
  (handle, rx)
}

async fn run(abort: Abort) {
  let args = Arguments::new()?;
  let (reader, receiver) = args.stream(abort);
  let opts = Options::new(args)?;
  let (steps, rx) = stream_process(abort, opts.clone(), receiver);
  let writer = stream_output(abort, opts, rx);
  if let Err(err) = try_join3(reader, steps, writer).await {
    abort.tx.send(Box::new(err)).await.expect("<CHANNEL>")
  }
}

fn main() {
  let rt = Builder::new_multi_thread()
    .enable_io()
    .build()
    .expect("runtime failure");
  rt.block_on(async {
    let (tx, rx) = watch::channel(Box::new(Failure::Gucci));
    let abort = Abort { tx, rx };
    let exiting = select! {
      maybe = rx.changed() => maybe,
      handle = run(abort) => handle
    };
    //if let Some(msg) = err.exit_message() {
    //  eprintln!("{}", Colour::Red.paint(msg));
    //}
  });
  rt.shutdown_timeout(Duration::MAX)
  //exit(err.exit_code())
}
