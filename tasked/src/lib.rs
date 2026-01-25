mod task;
mod traits;
pub use task::{TaskData, TaskHandle};
pub use traits::*;

use std::sync::atomic::{AtomicBool, Ordering};
use std::{sync::Arc, time::SystemTime};
use tokio::sync::{RwLock, watch};

pub use tasked_derive::Task;
pub use tasked_derive::TaskStatus;

#[derive(Clone)]
pub struct Task<P: Default + Clone> {
  state: Arc<RwLock<TaskData<P>>>,
  tx: watch::Sender<TaskData<P>>,
  abort: Arc<AtomicBool>,
}

impl<P: Default + Clone> Task<P> {
  pub async fn new(id: impl Into<String>) -> (Self, TaskHandle<P>) {
    let id = id.into();
    let state = Arc::new(RwLock::new(TaskData::new(id.clone())));
    let (tx, rx) = watch::channel(state.read().await.clone());
    let abort = Arc::new(AtomicBool::new(false));

    let handle = TaskHandle {
      id,
      rx,
      abort: abort.clone(),
    };
    let task = Self { state, tx, abort };

    // task.state.write().await.send_update(&task.tx);
    (task, handle)
  }

  pub fn was_aborted(&self) -> bool {
    self.abort.load(Ordering::Relaxed)
  }

  pub async fn start(&self) {
    self.state.write().await.start(&self.tx);
  }

  pub async fn close(self) {
    self.state.write().await.finish(&self.tx);
  }

  pub async fn update(&self, update_fn: impl FnOnce(&mut P) -> bool) {
    let mut state = self.state.write().await;
    if update_fn(&mut state.payload) {
      state.send_update(&self.tx);
    }
  }

  pub async fn duration(&self) -> Option<tokio::time::Duration> {
    let state = self.state.write().await;
    match (state.started_at, state.finished_at) {
      (Some(start), Some(end)) => end.duration_since(start).ok(),
      (Some(start), None) => SystemTime::now().duration_since(start).ok(),
      _ => None,
    }
  }
}

// wrapper for tasked-derive
#[doc(hidden)]
pub trait WrapTask<P: Default + Clone> {
  fn from_task(task: Task<P>) -> Self;
}
impl<P: Default + Clone> Task<P> {
  pub async fn new_wrapped<W: WrapTask<P>>(id: impl Into<String>) -> (W, TaskHandle<P>) {
    let (task, handle) = Self::new(id).await;
    (W::from_task(task), handle)
  }
}
