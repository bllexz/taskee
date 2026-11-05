mod common;

use common::{JobStatus, JobStatusExt, JobTask};
use std::time::Duration;

#[tokio::test]
async fn an_abort_is_visible_to_the_producer_and_to_observers() {
  let (task, handle) = JobTask::new("job");
  task.start();

  assert!(!task.is_aborted());
  handle.abort();

  assert!(task.is_aborted());
  assert!(handle.snapshot().is_abort_requested());
}

#[tokio::test]
async fn until_aborted_reacts_without_polling() {
  let (task, handle) = JobTask::new("job");
  task.start();

  let working = tokio::spawn(async move {
    if task.until_aborted(std::future::pending::<()>()).await.is_none() {
      task.cancelled();
    }
  });

  tokio::time::sleep(Duration::from_millis(10)).await;
  handle.abort();

  working.await.unwrap();
  assert_eq!(handle.wait().await.payload.status, JobStatus::Cancelled);
}
