use std::future::Future;

use tokio::runtime::Handle;
use tokio::sync::oneshot;

pub struct AsyncTask<T> {
    rx: Option<oneshot::Receiver<T>>,
    last: Option<T>,
}

impl<T> Default for AsyncTask<T> {
    fn default() -> Self {
        Self {
            rx: None,
            last: None,
        }
    }
}

impl<T: Send + 'static> AsyncTask<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn spawn<F>(&mut self, handle: &Handle, fut: F)
    where
        F: Future<Output = T> + Send + 'static,
    {
        let (tx, rx) = oneshot::channel();
        handle.spawn(async move {
            let value = fut.await;
            let _ = tx.send(value);
        });
        self.rx = Some(rx);
    }

    pub fn poll(&mut self) -> AsyncState<'_, T> {
        if let Some(rx) = &mut self.rx {
            match rx.try_recv() {
                Ok(v) => {
                    self.rx = None;
                    self.last = Some(v);
                    AsyncState::JustCompleted(self.last.as_ref().unwrap())
                }
                Err(oneshot::error::TryRecvError::Empty) => AsyncState::Pending,
                Err(oneshot::error::TryRecvError::Closed) => {
                    self.rx = None;
                    AsyncState::Idle
                }
            }
        } else {
            AsyncState::Idle
        }
    }

    pub fn is_pending(&self) -> bool {
        self.rx.is_some()
    }
}

pub enum AsyncState<'a, T> {
    Pending,
    JustCompleted(&'a T),
    Idle,
}
