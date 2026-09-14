use crate::task::{Task, TaskPayload};

/// A payload that tracks a quantity against a total.
///
/// Implemented by `#[derive(Task)]` with `#[taskee(progress)]` on a payload with
/// `current: u64`, `total: u64` and `progress: u8` fields.
pub trait Progress {
  /// Units completed so far.
  fn current(&self) -> u64;
  /// Units expected in total, or `0` when the total is not known yet.
  fn total(&self) -> u64;
  /// The last percentage written, from `0` to `100`.
  fn progress(&self) -> u8;
  /// Overwrites the completed units.
  fn set_current(&mut self, current: u64);
  /// Overwrites the expected total.
  fn set_total(&mut self, total: u64);
  /// Overwrites the stored percentage.
  fn set_progress(&mut self, progress: u8);

  /// The percentage implied by `current` and `total`.
  fn percent(&self) -> u8 {
    let total = self.total();
    if total == 0 {
      return 0;
    }
    ((self.current() as f64 / total as f64) * 100.0).clamp(0.0, 100.0) as u8
  }
}

impl<P: TaskPayload + Progress> Task<P> {
  /// Sets the expected total and recomputes the percentage.
  pub fn set_total(&self, total: u64) {
    self.update(|payload| {
      if payload.total() == total {
        return false;
      }
      payload.set_total(total);
      payload.set_current(payload.current().min(total));
      payload.set_progress(payload.percent());
      true
    });
  }

  /// Advances progress by `amount`, saturating at the total.
  ///
  /// Only notifies when the percentage moves.
  pub fn advance(&self, amount: u64) {
    self.update(|payload| advance_to(payload, payload.current().saturating_add(amount)));
  }

  /// Sets the units completed, saturating at the total.
  ///
  /// Only notifies when the percentage moves.
  pub fn set_current(&self, current: u64) {
    self.update(|payload| advance_to(payload, current));
  }

  /// Advances progress by `amount` and runs `f` on the payload. Always notifies.
  pub fn advance_with(&self, amount: u64, f: impl FnOnce(&mut P)) {
    self.set(|payload| {
      advance_to(payload, payload.current().saturating_add(amount));
      f(payload);
    });
  }
}

fn advance_to<P: Progress>(payload: &mut P, current: u64) -> bool {
  let previous = payload.progress();

  payload.set_current(current.min(payload.total()));
  let percent = payload.percent();
  payload.set_progress(percent);

  previous != percent
}
