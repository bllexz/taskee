use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

#[derive(Debug, Clone, Copy)]
struct Stamp {
  at: SystemTime,
  instant: Instant,
}

impl Stamp {
  fn now() -> Self {
    Self {
      at: SystemTime::now(),
      instant: Instant::now(),
    }
  }
}

/// A snapshot with the current state of a task.
#[derive(Debug, Clone)]
pub struct TaskData<P> {
  /// The identifier the task was created with.
  pub id: Arc<str>,
  /// The state a producer writes into.
  pub payload: P,
  start: Option<Stamp>,
  finish: Option<Stamp>,
  abandoned: bool,
  abort_requested: bool,
}

impl<P: Default> TaskData<P> {
  pub(crate) fn new(id: Arc<str>) -> Self {
    Self {
      id,
      payload: P::default(),
      start: None,
      finish: None,
      abandoned: false,
      abort_requested: false,
    }
  }
}

impl<P> TaskData<P> {
  pub(crate) fn mark_started(&mut self) -> bool {
    if self.start.is_some() {
      return false;
    }
    self.start = Some(Stamp::now());
    true
  }

  pub(crate) fn mark_finished(&mut self, abandoned: bool) -> bool {
    if self.finish.is_some() {
      return false;
    }
    let now = Stamp::now();
    self.start.get_or_insert(now);
    self.finish = Some(now);
    self.abandoned = abandoned;
    true
  }

  pub(crate) fn mark_abort_requested(&mut self) -> bool {
    if self.abort_requested {
      return false;
    }
    self.abort_requested = true;
    true
  }

  /// Whether the task has been started.
  pub fn is_started(&self) -> bool {
    self.start.is_some()
  }

  /// Whether the task has ended.
  pub fn is_finished(&self) -> bool {
    self.finish.is_some()
  }

  /// Whether the task ended because every producer was dropped.
  pub fn is_abandoned(&self) -> bool {
    self.abandoned
  }

  /// Whether an abort was requested. Not whether the producer acted on it.
  pub fn is_abort_requested(&self) -> bool {
    self.abort_requested
  }

  /// When the task started.
  pub fn started_at(&self) -> Option<SystemTime> {
    self.start.map(|stamp| stamp.at)
  }

  /// When the task finished.
  pub fn finished_at(&self) -> Option<SystemTime> {
    self.finish.map(|stamp| stamp.at)
  }

  /// Total run time, available only once the task has finished.
  pub fn duration(&self) -> Option<Duration> {
    Some(self.finish?.instant.duration_since(self.start?.instant))
  }

  /// Time since the task started. Keeps counting while it runs.
  pub fn elapsed(&self) -> Option<Duration> {
    let start = self.start?.instant;
    Some(match self.finish {
      Some(finish) => finish.instant.duration_since(start),
      None => start.elapsed(),
    })
  }
}
