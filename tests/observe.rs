mod common;

use common::{JobStatus, JobStatusExt, JobTask};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[tokio::test]
async fn on_update_stops_once_the_task_settles() {
  let (task, handle) = JobTask::new("job");
  task.start();

  let watcher = handle.on_update(|_| {});
  task.done();

  tokio::time::timeout(Duration::from_secs(1), watcher).await.expect("the watcher must stop on its own").unwrap();
}

#[tokio::test]
async fn next_drains_pending_changes_before_reporting_the_end() {
  let (task, mut handle) = JobTask::new("job");
  task.start();
  task.done();

  let last = handle.next().await.expect("the final state must not be skipped");
  assert_eq!(last.payload.status, JobStatus::Done);
  assert!(handle.next().await.is_none());
}

#[tokio::test(flavor = "current_thread")]
async fn only_a_percentage_change_notifies_observers() {
  let (task, handle) = JobTask::new("job");
  task.set_total(1000);

  let seen = Arc::new(Mutex::new(Vec::new()));
  let recorder = Arc::clone(&seen);
  let watcher = handle.on_update(move |state| recorder.lock().unwrap().push(state.payload.current));

  for step in [0, 1, 1, 8, 990] {
    task.advance(step);
    tokio::task::yield_now().await;
  }
  task.done();
  watcher.await.unwrap();

  let seen = seen.lock().unwrap().clone();
  assert!(!seen.contains(&1) && !seen.contains(&2), "sub-percentage steps must not be broadcast: {seen:?}");
  assert!(seen.contains(&10), "the step that crossed 1% must be broadcast: {seen:?}");
  assert_eq!(seen.last(), Some(&1000));
}
