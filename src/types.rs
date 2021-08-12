use std::{any::Any, error::Error};
use tokio::{
  sync::watch::{Receiver, Sender},
  task::JoinHandle,
};

pub type Task = JoinHandle<()>;

pub struct Abort {
  tx: Sender<Error>,
  rx: Receiver<Error>,
}
