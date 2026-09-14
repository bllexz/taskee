use crate::{channel::Channel, data::TaskData, handle::TaskHandle};
use std::future::Future;
use std::sync::Arc;

pub trait TaskPayload: Default + Clone + Send + Sync + 'static {}
impl<P: Default + Clone + Send + Sync + 'static> TaskPayload for P {}

pub trait AsTask: Sized {
  type Payload: TaskPayload;

  fn as_task(&self) -> &Task<Self::Payload>;
  fn into_task(self) -> Task<Self::Payload>;
}

impl<P: TaskPayload> AsTask for Task<P> {
  type Payload = P;

  fn as_task(&self) -> &Task<P> {
    self
  }

  fn into_task(self) -> Task<P> {
    self
  }
}

struct Producer<P> {
  chan: Arc<Channel<P>>,
}

impl<P> Drop for Producer<P> {
  fn drop(&mut self) {
    self.chan.finish(true);
  }
}

/// A cheaply cloneable task producer.
///
/// Clone it to write from several places at once. The task ends as abandoned
/// when the last clone is dropped without [`finish`](Self::finish).
#[derive(Clone)]
pub struct Task<P> {
  producer: Arc<Producer<P>>,
  id: Arc<str>,
}

impl<P: TaskPayload> Task<P> {
  /// Creates a task and the first handle observing it.
  ///
  /// ```
  /// #[derive(Default, Clone)]
  /// struct Bytes { done: u64 }
  ///
  /// let (task, handle) = taskee::Task::<Bytes>::new("download");
  /// task.start();
  /// task.set(|p| p.done = 42);
  /// assert_eq!(handle.snapshot().payload.done, 42);
  /// ```
  pub fn new(id: impl Into<String>) -> (Self, TaskHandle<P>) {
    let id: Arc<str> = id.into().into();
    let chan = Arc::new(Channel::new(Arc::clone(&id)));
    let handle = TaskHandle::new(Arc::clone(&chan));

    let task = Self {
      producer: Arc::new(Producer { chan }),
      id,
    };
    (task, handle)
  }

  fn chan(&self) -> &Channel<P> {
    &self.producer.chan
  }

  /// The identifier this task was created with
  pub fn id(&self) -> &str {
    &self.id
  }

  /// Opens another handle
  pub fn handle(&self) -> TaskHandle<P> {
    TaskHandle::new(Arc::clone(&self.producer.chan))
  }

  /// Stamps the start time and notifies. Calling it again does nothing.
  pub fn start(&self) {
    self.chan().start();
  }

  /// Ends the task. Calling it again does nothing.
  pub fn finish(&self) {
    self.chan().finish(false);
  }

  /// Mutates the payload. Notifies only if `f` returns `true`
  pub fn update(&self, f: impl FnOnce(&mut P) -> bool) {
    self.chan().update(f);
  }

  /// Mutates the payload and always notifies
  pub fn set(&self, f: impl FnOnce(&mut P)) {
    self.chan().update(|payload| {
      f(payload);
      true
    });
  }

  /// A copy of the current state
  pub fn snapshot(&self) -> TaskData<P> {
    self.chan().borrow().clone()
  }

  /// Whether the task has ended
  pub fn is_finished(&self) -> bool {
    self.chan().borrow().is_finished()
  }

  /// Whether an abort was requested through a handle
  pub fn is_aborted(&self) -> bool {
    self.chan().is_aborted()
  }

  /// Runs `future` unless the task is aborted first.
  ///
  /// Returns `None` if the abort won the race. The future is then dropped at
  /// its last suspension point. Wrap a whole `select!` in it when several
  /// futures have to give way at once.
  ///
  /// ```
  /// # #[derive(Default, Clone)] struct P;
  /// # #[tokio::main(flavor = "current_thread")] async fn main() {
  /// let (task, handle) = taskee::Task::<P>::new("job");
  /// handle.abort();
  /// assert!(task.until_aborted(std::future::pending::<()>()).await.is_none());
  /// # }
  /// ```
  pub async fn until_aborted<F: Future>(&self, future: F) -> Option<F::Output> {
    let aborted = self.chan().aborted();
    tokio::select! {
      biased;
      () = aborted => None,
      output = future => Some(output),
    }
  }
}
