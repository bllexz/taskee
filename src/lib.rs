mod channel;
mod data;
mod handle;
mod progress;
mod status;
mod task;

pub use data::TaskData;
pub use handle::TaskHandle;
pub use progress::Progress;
pub use status::{HasStatus, TaskStatus};
pub use task::{AsTask, Task, TaskPayload};

#[cfg(feature = "derive")]
pub use taskee_derive::{Task, TaskStatus};
