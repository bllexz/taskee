use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;
use taskee::{Task, TaskHandle};

#[tokio::test]
async fn task_create_and_close() {
  let (task, handle): (Task<()>, TaskHandle<()>) = Task::new("test-task").await;

  assert_eq!(handle.id, "test-task");
  assert!(!task.was_aborted());
  assert!(!handle.was_aborted());

  task.start().await;
  task.close().await;

  let result = handle.wait().await;
  assert!(result.finished_at.is_some());
  assert!(result.started_at.is_some());
}

#[tokio::test]
async fn task_abort() {
  let (task, handle): (Task<()>, TaskHandle<()>) = Task::new("abort-test").await;

  assert!(!handle.was_aborted());
  handle.abort();
  assert!(handle.was_aborted());
  assert!(task.was_aborted());
}

#[tokio::test]
async fn task_with_payload() {
  #[derive(Default, Clone)]
  struct MyPayload {
    counter: u32,
    message: String,
  }

  let (task, handle): (Task<MyPayload>, TaskHandle<MyPayload>) = Task::new("payload-test").await;

  task.start().await;

  task
    .update(|payload| {
      payload.counter = 42;
      payload.message = "Hello, World!".to_string();
      true
    })
    .await;

  task.close().await;

  let result = handle.wait().await;
  assert_eq!(result.payload.counter, 42);
  assert_eq!(result.payload.message, "Hello, World!");
}

#[tokio::test]
async fn task_duration() {
  let (task, handle): (Task<()>, TaskHandle<()>) = Task::new("duration-test").await;

  assert!(task.duration().await.is_none());

  task.start().await;

  tokio::time::sleep(Duration::from_millis(10)).await;
  let running_duration = task.duration().await;
  assert!(running_duration.is_some());
  assert!(running_duration.unwrap().as_millis() >= 10);

  task.close().await;

  let final_data = handle.wait().await;
  let final_duration = final_data.duration();
  assert!(final_duration.is_some());
}

#[tokio::test]
async fn task_handle_on_update() {
  #[derive(Default, Clone)]
  struct Counter {
    value: u32,
  }

  let (task, handle): (Task<Counter>, TaskHandle<Counter>) = Task::new("on-update-test").await;
  let update_count = Arc::new(AtomicU32::new(0));
  let update_count_clone = update_count.clone();

  let _join_handle = handle.on_update(move |_data| {
    update_count_clone.fetch_add(1, Ordering::SeqCst);
  });

  task.start().await;

  for i in 1..=5 {
    task
      .update(|payload| {
        payload.value = i;
        true
      })
      .await;
  }

  task.close().await;

  tokio::time::sleep(Duration::from_millis(50)).await;

  assert!(update_count.load(Ordering::SeqCst) > 0);
}

#[tokio::test]
async fn task_handle_wait_timeout() {
  let (task, handle): (Task<()>, TaskHandle<()>) = Task::new("timeout-test").await;

  task.start().await;

  let result = handle.wait_timeout(Duration::from_millis(50)).await;
  assert!(result.is_err());

  task.close().await;
}

#[tokio::test]
async fn task_handle_wait_map() {
  let (task, handle): (Task<()>, TaskHandle<()>) = Task::new("wait-map-test").await;

  task.start().await;
  task.close().await;

  let id = handle.wait_map(|data| data.id.clone()).await;
  assert_eq!(id, "wait-map-test");
}

#[tokio::test]
async fn task_handle_on_finish() {
  let (task, handle): (Task<()>, TaskHandle<()>) = Task::new("on-finish-test").await;
  let finished = Arc::new(AtomicU32::new(0));
  let finished_clone = finished.clone();

  let _join_handle = handle.on_finish(move |_data| {
    finished_clone.store(1, Ordering::SeqCst);
  });

  task.start().await;
  task.close().await;

  tokio::time::sleep(Duration::from_millis(50)).await;

  assert_eq!(finished.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn task_update_no_change() {
  #[derive(Default, Clone)]
  struct Data {
    value: u32,
  }

  let (task, _handle): (Task<Data>, TaskHandle<Data>) = Task::new("no-change-test").await;

  task.start().await;

  task.update(|_payload| false).await;

  task.close().await;
}
