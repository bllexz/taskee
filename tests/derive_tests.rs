use taskee::{Progress, Task, TaskStatus};

#[derive(Default, Clone, Task)]
#[taskee(progress)]
struct Download {
  current: u64,
  total: u64,
  progress: u8,
  filename: String,
}

#[tokio::test]
async fn derive_task_basic() {
  let (task, handle) = DownloadTask::new("download-1").await;

  assert_eq!(handle.id, "download-1");

  task.start().await;
  task.close().await;

  let result = handle.wait().await;
  assert!(result.finished_at.is_some());
}

#[tokio::test]
async fn derive_task_progress() {
  let (task, handle) = DownloadTask::new("download-progress").await;

  task.start().await;
  task.set_total(100).await;
  task.add_progress(25).await;

  let data = handle.rx.borrow().clone();
  assert_eq!(data.payload.total, 100);
  assert_eq!(data.payload.current, 25);
  assert_eq!(data.payload.progress, 25);

  task.add_progress(25).await;

  let data = handle.rx.borrow().clone();
  assert_eq!(data.payload.current, 50);
  assert_eq!(data.payload.progress, 50);

  task.close().await;
}

#[tokio::test]
async fn derive_task_set_progress() {
  let (task, handle) = DownloadTask::new("set-progress").await;

  task.start().await;
  task.set_total(200).await;
  task.set_progress(100).await;

  let data = handle.rx.borrow().clone();
  assert_eq!(data.payload.current, 100);
  assert_eq!(data.payload.progress, 50);

  task.close().await;
}

#[derive(Default, Clone, TaskStatus)]
pub enum DownloadStatus {
  #[default]
  Pending,
  Connecting,
  Downloading,
  #[terminal]
  Completed,
  #[terminal]
  Failed,
}

#[derive(Default, Clone, Task)]
#[taskee(status)]
pub struct DownloadWithStatus {
  status: DownloadStatus,
  bytes_downloaded: u64,
}

#[tokio::test]
async fn derive_task_status_methods() {
  let (task, handle) = DownloadWithStatusTask::new("status-test").await;

  task.start().await;

  let data = handle.rx.borrow().clone();
  assert!(!data.payload.status.is_terminal());

  task.set_connecting().await;
  let data = handle.rx.borrow().clone();
  matches!(data.payload.status, DownloadStatus::Connecting);

  task.set_downloading().await;
  let data = handle.rx.borrow().clone();
  matches!(data.payload.status, DownloadStatus::Downloading);

  task.completed().await;

  let result = handle.wait().await;
  assert!(result.payload.status.is_terminal());
  assert!(result.finished_at.is_some());
}

#[tokio::test]
async fn task_status_is_terminal() {
  let mut status = DownloadStatus::Pending;
  assert!(!status.is_terminal());

  status.set_connecting();
  assert!(!status.is_terminal());

  status.completed();
  assert!(status.is_terminal());

  let mut status2 = DownloadStatus::Downloading;
  status2.failed();
  assert!(status2.is_terminal());
}

#[derive(Default, Clone, TaskStatus)]
pub enum ProcessStatus {
  #[default]
  Waiting,
  Processing,
  #[terminal]
  Done,
  #[terminal]
  Error,
}

#[derive(Default, Clone, Task)]
#[taskee(status, progress)]
pub struct ProcessPayload {
  status: ProcessStatus,
  current: u64,
  total: u64,
  progress: u8,
}

#[tokio::test]
async fn derive_task_status_and_progress() {
  let (task, handle) = ProcessPayloadTask::new("combined-test").await;

  task.start().await;
  task.set_total(50).await;
  task.set_processing().await;

  for _ in 1..=50 {
    task.add_progress(1).await;
  }

  let data = handle.rx.borrow().clone();
  assert_eq!(data.payload.current, 50);
  assert_eq!(data.payload.progress, 100);

  task.done().await;

  let result = handle.wait().await;
  assert!(result.payload.status.is_terminal());
  assert!(result.finished_at.is_some());
}

#[test]
fn progress_trait_calc() {
  #[derive(Default)]
  struct TestProgress {
    current: u64,
    total: u64,
    progress: u8,
  }

  impl Progress for TestProgress {
    fn current(&self) -> u64 {
      self.current
    }
    fn total(&self) -> u64 {
      self.total
    }
    fn progress(&self) -> u8 {
      self.progress
    }
    fn set_current(&mut self, val: u64) {
      self.current = val;
    }
    fn set_total(&mut self, val: u64) {
      self.total = val;
    }
    fn set_progress(&mut self, val: u8) {
      self.progress = val;
    }
  }

  let mut p = TestProgress {
    current: 0,
    total: 100,
    progress: 0,
  };

  // 0/100 = 0%
  assert_eq!(p.calc_progress(), 0);

  p.current = 50;
  // 50/100 = 50%
  assert_eq!(p.calc_progress(), 50);

  p.current = 100;
  // 100/100 = 100%
  assert_eq!(p.calc_progress(), 100);

  // Edge case: total = 0
  p.total = 0;
  assert_eq!(p.calc_progress(), 0);

  p.total = 50;
  p.current = 100;
  assert_eq!(p.calc_progress(), 100); // min(200, 100) = 100
}
