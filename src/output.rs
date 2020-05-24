use super::argparse::{Action, Options, Printer};
use super::errors::*;
use super::subprocess::SubprocessCommand;
use super::types::Task;
use ansi_term::Colour;
use async_std::sync::Receiver;
use futures::future::try_join;
use std::{collections::HashMap, env, process};
use tokio::{
  io::{self, AsyncWriteExt, BufWriter},
  task,
};

fn stream_stdout(stream: Receiver<SadResult<String>>) -> Task {
  let mut stdout = BufWriter::new(io::stdout());
  task::spawn(async move {
    while let Some(print) = stream.recv().await {
      match print {
        Ok(val) => match stdout.write(val.as_bytes()).await {
          Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => {
            err_exit(Failure::Interrupt).await
          }
          Err(e) => err_exit(e.into()).await,
          _ => {}
        },
        Err(e) => err_exit(e).await,
      }
    }
    stdout.shutdown().await.unwrap()
  })
}

pub fn stream_output(opts: Options, stream: Receiver<SadResult<String>>) -> Task {
  match (opts.action, opts.printer) {
    (Action::Fzf, _) => {
      let preview_args = env::args().collect::<Vec<_>>().join("\x00");
      let execute = format!("abort+execute:{}\x00--internal-patch={{+}}", preview_args);
      let mut arguments = vec![
        "--read0".to_string(),
        "-m".to_string(),
        "--ansi".to_string(),
        format!("--bind=enter:{}", execute),
        format!("--bind=double-click:{}", execute),
        format!("--preview={}\x00--internal-preview={{}}", preview_args),
        "--preview-window=70%:wrap".to_string(),
      ];
      arguments.extend(opts.fzf.unwrap_or_default());
      let mut env = HashMap::new();
      env.insert("SHELL".to_owned(), opts.name);
      let cmd = SubprocessCommand {
        program: "fzf".to_string(),
        arguments,
        env,
      };
      let (child, rx) = cmd.stream_connected(stream);
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
  if !err.silent_exit() {
    eprintln!("{}", Colour::Red.paint(format!("Error:\n{:#?}", err)));
  }
  process::exit(1)
}
