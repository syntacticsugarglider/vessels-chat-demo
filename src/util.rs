use core::{
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};
use futures::{
    task::{FutureObj, Spawn, SpawnError},
    Sink, SinkExt,
};
use smol::Task;
use std::fmt::{Debug, Display};

use thiserror::Error;

#[derive(Error, Clone)]
#[error("{display}")]
pub struct TransportError {
    display: String,
    debug: String,
}

impl Debug for TransportError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(formatter, "{}", self.debug)
    }
}

impl TransportError {
    pub fn new<T: Display + Debug>(item: T) -> Self {
        Self {
            display: format!("{}", item),
            debug: format!("{:?}", item),
        }
    }
}

#[derive(Clone)]
pub struct Spawner;

impl Spawn for Spawner {
    fn spawn_obj(&self, future: FutureObj<'static, ()>) -> Result<(), SpawnError> {
        Task::spawn(future).detach();
        Ok(())
    }
}

pub struct CloseOnDrop<T: Send + Sink<U> + Unpin + 'static, U> {
    sink: Option<T>,
    marker: PhantomData<U>,
}

impl<T: Send + Sink<U> + Unpin + 'static, U> CloseOnDrop<T, U> {
    pub fn new(sink: T) -> Self {
        CloseOnDrop {
            sink: Some(sink),
            marker: PhantomData,
        }
    }
}

impl<T: Send + Sink<U> + Unpin + 'static, U> Drop for CloseOnDrop<T, U> {
    fn drop(&mut self) {
        let mut sink = self.sink.take().unwrap();

        Task::spawn(async move {
            let _ = sink.close().await;
        })
        .detach();
    }
}

impl<T: Send + Sink<U> + Unpin + 'static, U> Unpin for CloseOnDrop<T, U> {}

impl<T: Send + Sink<U> + Unpin + 'static, U> Sink<U> for CloseOnDrop<T, U> {
    type Error = T::Error;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        Pin::new(self.sink.as_mut().expect("invalid state")).poll_ready(cx)
    }

    fn start_send(mut self: Pin<&mut Self>, item: U) -> Result<(), Self::Error> {
        Pin::new(self.sink.as_mut().expect("invalid state")).start_send(item)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        Pin::new(self.sink.as_mut().expect("invalid state")).poll_flush(cx)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        Pin::new(self.sink.as_mut().expect("invalid state")).poll_close(cx)
    }
}
