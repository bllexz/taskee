use std::sync::atomic::{AtomicBool, Ordering};
use std::{sync::Arc, time::SystemTime};
use tokio::sync::watch::{Receiver, Sender};
use tokio::task::JoinHandle;
use tokio::time::{Duration, error::Elapsed};

#[derive(Default, Clone)]
pub struct TaskData<P: Default + Clone> {
  pub id: String,
  pub payload: P,
  pub finished_at: Option<SystemTime>,
  pub started_at: Option<SystemTime>,
}

impl<P: Default + Clone> TaskData<P> {
  pub(super) fn new(id: String) -> Self {
    Self { id, ..Self::default() }
  }

  pub(super) fn start(&mut self, tx: &Sender<Self>) {
    self.started_at = Some(SystemTime::now());
    self.send_update(tx);
  }

  pub(super) fn finish(&mut self, tx: &Sender<Self>) {
    self.finished_at = Some(SystemTime::now());
    self.send_update(tx);
  }

  pub(super) fn send_update(&self, tx: &Sender<Self>) {
    tx.send(self.clone()).ok();
  }

  pub fn duration(&self) -> Option<Duration> {
    match (self.started_at, self.finished_at) {
      (Some(start), Some(end)) => end.duration_since(start).ok(),
      (Some(start), None) => SystemTime::now().duration_since(start).ok(),
      _ => None,
    }
  }
}

//

#[derive(Clone)]
pub struct TaskHandle<P: Default + Clone> {
  pub id: String,
  pub rx: Receiver<TaskData<P>>,
  pub(super) abort: Arc<AtomicBool>,
}
impl<P: Default + Clone + Send + Sync + 'static> TaskHandle<P> {
  pub fn abort(&self) {
    self.abort.store(true, Ordering::Relaxed);
  }

  pub fn was_aborted(&self) -> bool {
    self.abort.load(Ordering::Relaxed)
  }

  pub fn on_update<F: Fn(TaskData<P>) + Send + 'static>(&self, f: F) -> JoinHandle<()> {
    let mut rx = self.rx.clone();
    tokio::spawn(async move {
      while rx.changed().await.is_ok() {
        let data = rx.borrow().clone();
        f(data);
      }
    })
  }

  pub async fn wait(&self) -> TaskData<P> {
    let mut rx = self.rx.clone();
    loop {
      let is_finished = {
        let state = rx.borrow_and_update();
        state.finished_at.is_some()
      };
      if is_finished || self.was_aborted() {
        return rx.borrow().clone();
      }
      if rx.changed().await.is_err() {
        break;
      }
    }
    self.rx.borrow().clone()
  }

  pub async fn wait_map<T, F: FnOnce(TaskData<P>) -> T>(&self, f: F) -> T {
    let result = self.wait().await;
    f(result)
  }

  pub async fn wait_timeout(&self, dur: Duration) -> Result<TaskData<P>, Elapsed> {
    tokio::time::timeout(dur, self.wait()).await
  }

  pub fn on_finish<F: FnOnce(TaskData<P>) + Send + 'static>(&self, f: F) -> JoinHandle<()> {
    let handle = self.clone();
    tokio::spawn(async move {
      let result = handle.wait().await;
      f(result);
    })
  }
}
