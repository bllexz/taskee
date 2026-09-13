pub trait Progress {
  fn current(&self) -> u64;
  fn total(&self) -> u64;
  fn progress(&self) -> u8;
  fn set_current(&mut self, val: u64);
  fn set_total(&mut self, val: u64);
  fn set_progress(&mut self, val: u8);
  fn calc_progress(&self) -> u8 {
    if self.total() == 0 {
      return 0;
    }
    ((self.current() as f64 / self.total() as f64) * 100.0).min(100.0) as u8
  }
}

impl<P: Default + Clone + Progress> crate::Task<P> {
  pub async fn set_total(&self, total: u64) {
    self
      .update(|payload| {
        payload.set_total(total);
        true
      })
      .await;
  }

  pub async fn add_progress(&self, amount: u64) {
    self
      .update(|payload| {
        let prev_progress = payload.progress();

        let new_current = payload.current().saturating_add(amount).min(payload.total());
        payload.set_current(new_current);

        let new_progress = payload.calc_progress();
        payload.set_progress(new_progress);

        prev_progress < new_progress
      })
      .await;
  }

  pub async fn set_progress(&self, current: u64) {
    self
      .update(|payload| {
        let prev_progress = payload.progress();

        payload.set_current(current.min(payload.total()));

        let new_progress = payload.calc_progress();
        payload.set_progress(new_progress);

        prev_progress != new_progress
      })
      .await;
  }
}
