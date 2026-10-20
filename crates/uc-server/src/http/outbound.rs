//! Requests to other HTTP services, sent from the IO runtime
//! (`uc_storage::runtime`): their connections run there, not on the
//! runtime that serves the API.

use uc_storage::runtime::{IoTask, spawn_io};

/// [`reqwest::RequestBuilder::send`] on the IO runtime.
pub trait SendOnIo {
    fn send_on_io(self) -> IoTask<reqwest::Result<reqwest::Response>>;
}

impl SendOnIo for reqwest::RequestBuilder {
    fn send_on_io(self) -> IoTask<reqwest::Result<reqwest::Response>> {
        spawn_io(self.send())
    }
}
