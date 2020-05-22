use tokio::task::JoinHandle;

pub type Task = JoinHandle<()>;

