use crate::{channel::Channel, data::TaskData, task::TaskPayload};
use std::sync::Arc;
use tokio::sync::watch;

/// A cheaply cloneable task handle.
///
/// It reads the state and can request an abort. It never writes the payload.
#[derive(Clone)]
pub struct TaskHandle<P> {
  /// The identifier the task was created with.
  pub id: Arc<str>,
  chan: Arc<Channel<P>>,
  rx: watch::Receiver<TaskData<P>>,
}

impl<P: TaskPayload> TaskHandle<P> {
  pub(crate) fn new(chan: Arc<Channel<P>>) -> Self {
    let rx = chan.subscribe();
    let id = Arc::clone(&chan.borrow().id);
    Self { id, chan, rx }
  }

  /// A copy of the current state.
  pub fn snapshot(&self) -> TaskData<P> {
    self.chan.borrow().clone()
  }

  /// Whether the task has ended.
  pub fn is_finished(&self) -> bool {
    self.chan.borrow().is_finished()
  }

  /// Requests an abort and notifies observers.
  pub fn abort(&self) {
    self.chan.request_abort();
  }

  /// Returns the next state this handle has not seen, waiting if there is none.
  /// Gives `None` once nothing is left to read (finished).
  pub async fn next(&mut self) -> Option<TaskData<P>> {
    if self.rx.has_changed().unwrap_or(false) {
      return Some(self.rx.borrow_and_update().clone());
    }
    if self.rx.borrow().is_finished() {
      return None;
    }
    self.rx.changed().await.ok()?;
    Some(self.rx.borrow_and_update().clone())
  }

  /// Waits for the task to end and returns its final state.
  pub async fn wait(&self) -> TaskData<P> {
    let _ = self.chan.subscribe().wait_for(TaskData::is_finished).await;
    self.chan.borrow().clone()
  }
}

#[cfg(feature = "rt")]
impl<P: TaskPayload> TaskHandle<P> {
  /// Spawns a task that calls `f` with the current state and on every change,
  /// until the task ends.
  ///
  /// `f` gets a borrow, it holds the channel read lock and blocks
  /// writes while it runs. Use [`next`](Self::next) for a heavy callback.
  /// One that writes to this same task deadlocks.
  pub fn on_update<F>(&self, mut f: F) -> tokio::task::JoinHandle<()>
  where
    F: FnMut(&TaskData<P>) + Send + 'static,
  {
    let mut rx = self.chan.subscribe();
    tokio::spawn(async move {
      loop {
        {
          let data = rx.borrow_and_update();
          f(&data);
          if data.is_finished() {
            break;
          }
        }
        if rx.changed().await.is_err() {
          break;
        }
      }
    })
  }

  /// Spawns a task that calls `f` once, with the final state.
  pub fn on_finish<F>(&self, f: F) -> tokio::task::JoinHandle<()>
  where
    F: FnOnce(TaskData<P>) + Send + 'static,
  {
    let handle = self.clone();
    tokio::spawn(async move { f(handle.wait().await) })
  }
}
