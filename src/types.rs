use std::error::Error;
use tokio::{
  sync::watch::{Receiver, Sender},
  task::JoinHandle,
};

pub type Task = JoinHandle<()>;

pub struct Abort {
  pub tx: Sender<Boxed<dyn Error>>,
  pub rx: Receiver<Boxed<dyn Error>>,
}
