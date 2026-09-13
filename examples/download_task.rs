//! Download task example

use taskee::{Task, TaskStatus};

// You can create a status enum for your task
// It must have one default status and optional terminals
#[derive(TaskStatus, Default, Clone)]
pub enum DownloadStatus {
  #[default] // this is the status the task will have once created
  Pending,
  Downloading,
  Installing,
  #[terminal] // the task will end if it reaches a terminal status
  Failed,
  #[terminal]
  Aborted,
  #[terminal]
  Success,
}

// Now you can create your custom payload
#[derive(Task, Default, Clone)]
#[taskee(progress, status)]
pub struct Download {
  pub id: String,
  pub status: DownloadStatus,

  pub current: u64, // bytes
  pub total: u64,   // bytes
  pub progress: u8, // 0-100

  pub message: String,
  pub error_message: String,
}

// private download manager
impl DownloadTask {
  pub async fn set_message(&self, message: impl Into<String>) {
    self
      .update(|payload| {
        payload.message = message.into();
        true
      })
      .await;
  }

  pub async fn set_error_message(&self, message: impl Into<String>) {
    self
      .update(|payload| {
        payload.error_message = message.into();
        true
      })
      .await;
  }

  pub async fn add_progress_msg(&self, amount: u64, message: impl Into<String>) {
    use taskee::Progress;
    self
      .update(|payload| {
        let prev_progress = payload.progress();

        let new_current = payload.current().saturating_add(amount).min(payload.total());
        payload.set_current(new_current);

        let new_progress = payload.calc_progress();
        payload.set_progress(new_progress);
        payload.message = message.into();

        prev_progress < new_progress
      })
      .await;
  }
}

#[tokio::main]
async fn main() {
  let (_t, _h) = DownloadTask::new("id").await;

  // let (_t, _h) = DownloadTask::new("some_id").await;
}
