use super::errors::SadResult;
use tokio::task::JoinHandle;

pub type Task = JoinHandle<SadResult<()>>;
