//! A download reporting progress to a UI with a cancel button.
//!
//! Run with `cargo run --example download`. Add `-- cancel` to watch the task being aborted,
//! or `-- corrupt` to see a terminal status that carries a field.
//!
//! A task has two halves: the producer `DownloadTask` writes the state and the handle
//! `TaskHandle<Download>` reads it and can ask for an abort. Both clone, so a bigger job
//! can have many of each writing and watching the same state.

use std::time::Duration;
use taskee::{Task, TaskStatus};

// `TaskStatus` turns this enum into the task's state machine.
//
// Every non-terminal variant becomes `set_{variant}()` on the producer.
// Every terminal variant becomes `{variant}()`, which consumes the producer and finishes the task.
//
// Those methods arrive through `DownloadStatusExt`.
// It is already in scope here, from another module you must `use` it first.
#[derive(TaskStatus, Default, Clone, Debug, PartialEq)]
enum DownloadStatus {
  #[default]
  Pending,
  Downloading,
  Verifying,
  #[terminal]
  Failed(String),
  #[terminal]
  Cancelled,
  #[terminal]
  Done,
}

// `Task` generates the producer: struct `Download` produces `DownloadTask`.
//
// Flags in `#[taskee(..)]`:
// - `status` points at the `status` field, unlocking the setters the enum above describes.
// - `progress` points at `current`, `total` and `progress`, unlocking `set_total`, `advance` and `set_current`.
//
// `message` belongs to neither: ordinary payload, written by hand with `set`.
#[derive(Task, Default, Clone)]
#[taskee(progress, status)]
struct Download {
  status: DownloadStatus,

  current: u64,
  total: u64,
  progress: u8,

  message: String,
}

// Download simulation
const CHUNK: u64 = 64 * 1024;
const CHUNKS: u64 = 160;

#[tokio::main]
async fn main() {
  let args: Vec<String> = std::env::args().collect();
  let cancel = args.iter().any(|arg| arg == "cancel");
  let corrupt = args.iter().any(|arg| arg == "corrupt");
  let (task, handle) = DownloadTask::new("client");

  handle.on_update(|state| {
    let payload = &state.payload;
    println!("[{:>3}%] {:?} {}", payload.progress, payload.status, payload.message);
  });

  tokio::spawn(download(task, corrupt));

  if cancel {
    tokio::time::sleep(Duration::from_millis(1234)).await;
    println!("-- cancel requested");

    handle.abort(); // Nothing is killed here. This only raises a flag the producer will see.
  }

  let state = handle.wait().await;
  println!("-- {:?} after {:?}", state.payload.status, state.duration().unwrap_or_default());
}

async fn download(task: DownloadTask, corrupt: bool) {
  task.start();
  task.set_total(CHUNK * CHUNKS);
  task.set_downloading();

  for chunk in 0..CHUNKS {
    let sleep = tokio::time::sleep(Duration::from_millis(5));

    // `None` means the abort won the race, and the sleep was dropped mid job.
    if task.until_aborted(sleep).await.is_none() {
      task.cancelled();
      return;
    }

    if chunk == CHUNKS / 2 {
      task.set(|payload| payload.message = "halfway".into());
    }

    task.advance(CHUNK); // `advance` only notifies when the percentage actually moves
  }

  task.set_verifying();
  if task.until_aborted(tokio::time::sleep(Duration::from_millis(50))).await.is_none() {
    task.cancelled();
    return;
  }

  if corrupt {
    task.failed("checksum mismatch".into());
    return;
  }

  task.done();
}
