[![Crates.io](https://img.shields.io/crates/v/taskee)](https://crates.io/crates/taskee)
[![Docs.rs](https://img.shields.io/docsrs/taskee)](https://docs.rs/taskee)
[![CI](https://github.com/bllexz/taskee/actions/workflows/ci.yml/badge.svg)](https://github.com/bllexz/taskee/actions/workflows/ci.yml)
![License](https://img.shields.io/crates/l/taskee)

# taskee

Typed, observable state for long-running async tasks.

You define the task payload, whatever the work has to report. `taskee` gives you both cloneable producers to write it and handles to read it.

```toml
[dependencies]
taskee = "0.1"
```

## Example

```rust
use taskee::{Task, TaskStatus};

#[derive(TaskStatus, Default, Clone, Debug, PartialEq)]
enum DownloadStatus {
  #[default]
  Pending,
  Downloading,
  #[terminal] // reaching a terminal variant finishes the task
  Failed(String),
  #[terminal]
  Done,
}

#[derive(Task, Default, Clone)]
#[taskee(progress, status)]
struct Download {
  status: DownloadStatus,
  current: u64,
  total: u64,
  progress: u8,
}

#[tokio::main]
async fn main() {
  let (task, handle) = DownloadTask::new("assets");
  task.set_total(4000);
  task.set_downloading();
  task.start();

  let worker = tokio::spawn(async move {
    for _ in 0..4 {
      if task.is_aborted() {
        task.failed("cancelled".into());
        return;
      }
      task.advance(1000);
    }
    task.done();
  });

  let final_state = handle.wait().await;
  assert_eq!(final_state.payload.status, DownloadStatus::Done);
  assert_eq!(final_state.payload.progress, 100);
  worker.await.unwrap();
}
```
