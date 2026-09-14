use crate::data::TaskData;
use std::sync::Arc;
use tokio::sync::watch;

pub(crate) struct Channel<P> {
  data: watch::Sender<TaskData<P>>,
  abort: watch::Sender<bool>,
}

impl<P: Default> Channel<P> {
  pub(crate) fn new(id: Arc<str>) -> Self {
    Self {
      data: watch::Sender::new(TaskData::new(id)),
      abort: watch::Sender::new(false),
    }
  }
}

impl<P> Channel<P> {
  pub(crate) fn update(&self, f: impl FnOnce(&mut P) -> bool) {
    self.data.send_if_modified(|data| {
      if data.is_finished() {
        return false;
      }
      f(&mut data.payload)
    });
  }

  pub(crate) fn start(&self) {
    self.data.send_if_modified(TaskData::mark_started);
  }

  pub(crate) fn finish(&self, abandoned: bool) {
    self.data.send_if_modified(|data| data.mark_finished(abandoned));
  }

  pub(crate) fn request_abort(&self) {
    self.abort.send_replace(true);
    self.data.send_if_modified(TaskData::mark_abort_requested);
  }

  pub(crate) fn is_aborted(&self) -> bool {
    *self.abort.borrow()
  }

  pub(crate) async fn aborted(&self) {
    let _ = self.abort.subscribe().wait_for(|aborted| *aborted).await;
  }

  pub(crate) fn borrow(&self) -> watch::Ref<'_, TaskData<P>> {
    self.data.borrow()
  }

  pub(crate) fn subscribe(&self) -> watch::Receiver<TaskData<P>> {
    self.data.subscribe()
  }
}
