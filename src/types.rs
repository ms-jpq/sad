use std::error::Error;
use tokio::{
  sync::watch::{Receiver, Sender},
  task::JoinHandle,
};

pub type Task = JoinHandle<()>;

pub struct Abort {
  pub tx: Sender<Box<dyn Error>>,
  pub rx: Receiver<Box<dyn Error>>,
}
