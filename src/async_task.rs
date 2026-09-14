use std::future::Future;

use tokio::runtime::Handle;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

/// A single in-flight background job whose result the UI thread polls once per
/// frame.
///
/// Holds the `JoinHandle` as well as the result channel so that a superseded or
/// dropped task is actually cancelled rather than left running against
/// credentials the app has since moved on from.
pub struct AsyncTask<T> {
    rx: Option<oneshot::Receiver<T>>,
    join: Option<JoinHandle<()>>,
}

impl<T> Default for AsyncTask<T> {
    fn default() -> Self {
        Self {
            rx: None,
            join: None,
        }
    }
}

impl<T> AsyncTask<T> {
    /// Cancel the in-flight job, if any. Safe to call when idle.
    ///
    /// Deliberately not bounded on `T: Send`, so that `Drop` (which cannot add
    /// bounds beyond the struct's own) can call it.
    pub fn abort(&mut self) {
        if let Some(join) = self.join.take() {
            join.abort();
        }
        self.rx = None;
    }

    fn clear(&mut self) {
        self.rx = None;
        self.join = None;
    }
}

impl<T> Drop for AsyncTask<T> {
    fn drop(&mut self) {
        self.abort();
    }
}

impl<T: Send + 'static> AsyncTask<T> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Start a job, cancelling any previous one still in flight.
    pub fn spawn<F>(&mut self, handle: &Handle, fut: F)
    where
        F: Future<Output = T> + Send + 'static,
    {
        // Without this, switching profiles mid-request leaves e.g. a 5,000-user
        // list running to completion against the old profile's token.
        self.abort();

        let (tx, rx) = oneshot::channel();
        let join = handle.spawn(async move {
            let value = fut.await;
            let _ = tx.send(value);
        });
        self.rx = Some(rx);
        self.join = Some(join);
    }

    /// Returns the result by value on the single frame it completes.
    pub fn poll(&mut self) -> AsyncState<T> {
        let Some(rx) = &mut self.rx else {
            return AsyncState::Idle;
        };
        match rx.try_recv() {
            Ok(v) => {
                self.clear();
                AsyncState::JustCompleted(v)
            }
            Err(oneshot::error::TryRecvError::Empty) => AsyncState::Pending,
            // The sender was dropped without sending: the future panicked or was
            // aborted. Reported distinctly so callers can show an error instead
            // of the spinner silently stopping with no explanation.
            Err(oneshot::error::TryRecvError::Closed) => {
                self.clear();
                AsyncState::Failed
            }
        }
    }

    pub fn is_pending(&self) -> bool {
        self.rx.is_some()
    }
}

pub enum AsyncState<T> {
    Pending,
    /// Completed this frame; the payload is handed over by value.
    JustCompleted(T),
    /// The background task died without producing a result.
    Failed,
    Idle,
}
