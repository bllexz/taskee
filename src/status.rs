use crate::task::{Task, TaskPayload};

/// A status enum that knows which of its variants end the task.
///
/// Implemented by `#[derive(TaskStatus)]` from the `#[terminal]` markers.
pub trait TaskStatus: Default + Clone + PartialEq + Send + Sync + 'static {
  /// Whether reaching this status means the task is over.
  fn is_terminal(&self) -> bool;
}

/// A payload that carries a task status.
///
/// Implemented by `#[derive(Task)]` with `#[taskee(status)]`.
pub trait HasStatus: TaskPayload {
  /// The status enum stored in the payload
  type Status: TaskStatus;

  /// Reads the current status
  fn status(&self) -> &Self::Status;
  /// Overwrites the current status
  fn set_status(&mut self, status: Self::Status);
}

impl<P: HasStatus> Task<P> {
  /// The current status
  pub fn status(&self) -> P::Status {
    self.snapshot().payload.status().clone()
  }

  /// Moves the task to `status`, and finishes it if that status is terminal.
  ///
  /// Only notifies when the status really changes.
  pub fn set_task_status(&self, status: P::Status) {
    let is_terminal = status.is_terminal();

    self.update(|payload| {
      if *payload.status() == status {
        return false;
      }
      payload.set_status(status);
      true
    });

    if is_terminal {
      self.finish();
    }
  }
}
