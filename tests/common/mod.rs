use taskee::{Task, TaskStatus};

#[derive(TaskStatus, Default, Clone, Debug, PartialEq)]
pub enum JobStatus {
  #[default]
  Pending,
  Running,
  #[terminal]
  Failed(String),
  #[terminal]
  Cancelled,
  #[terminal]
  Done,
}

#[derive(Task, Default, Clone, Debug)]
#[taskee(progress, status)]
#[allow(dead_code)]
pub struct Job {
  pub status: JobStatus,
  pub current: u64,
  pub total: u64,
  pub progress: u8,
  pub message: String,
}
