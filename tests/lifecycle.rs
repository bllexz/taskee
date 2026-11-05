mod common;

use common::{JobStatus, JobStatusExt, JobTask};
use std::time::Duration;

const WRITERS: u64 = 32;
const STEPS: u64 = 250;

#[tokio::test]
async fn a_terminal_status_finishes_the_task() {
  let (task, handle) = JobTask::new("job");
  task.start();
  task.set_running();
  assert!(!handle.is_finished());

  task.done();

  let state = handle.wait().await;
  assert_eq!(state.payload.status, JobStatus::Done);
  assert!(!state.is_abandoned());
  assert!(state.duration().is_some());
}

#[tokio::test]
async fn a_terminal_status_carries_its_fields() {
  let (task, handle) = JobTask::new("job");
  task.start();
  task.failed("disk full".into());

  let state = handle.wait().await;
  assert_eq!(state.payload.status, JobStatus::Failed("disk full".into()));
}

#[tokio::test]
async fn dropping_the_producer_settles_the_task_as_abandoned() {
  let (task, handle) = JobTask::new("job");
  task.start();
  task.set_running();
  drop(task);

  let state = tokio::time::timeout(Duration::from_secs(1), handle.wait())
    .await
    .expect("wait must resolve when the producer is dropped");

  assert!(state.is_abandoned());
  assert_eq!(state.payload.status, JobStatus::Running);
}

#[tokio::test]
async fn a_settled_task_ignores_further_writes() {
  let (task, handle) = JobTask::new("job");
  let survivor = task.clone();

  task.start();
  task.set(|payload| payload.message = "final".into());
  task.done();

  survivor.set(|payload| payload.message = "too late".into());
  survivor.set_running();

  let state = handle.wait().await;
  assert_eq!(state.payload.message, "final");
  assert_eq!(state.payload.status, JobStatus::Done);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn many_producers_never_lose_an_increment() {
  let (task, handle) = JobTask::new("race");
  task.start();
  task.set_total(WRITERS * STEPS);

  let mut writers = Vec::new();
  for _ in 0..WRITERS {
    let writer = task.clone();
    writers.push(tokio::spawn(async move {
      for _ in 0..STEPS {
        writer.advance(1);
      }
    }));
  }

  for writer in writers {
    writer.await.unwrap();
  }

  let payload = handle.snapshot().payload;
  assert_eq!(payload.current, WRITERS * STEPS);
  assert_eq!(payload.progress, 100);
}
