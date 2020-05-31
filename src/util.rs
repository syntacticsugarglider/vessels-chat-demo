use async_std::task::spawn;
use core::{
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};
use futures::{
    task::{FutureObj, Spawn, SpawnError},
    Sink, SinkExt,
};

#[derive(Clone)]
pub struct Spawner;

impl Spawn for Spawner {
    fn spawn_obj(&self, future: FutureObj<'static, ()>) -> Result<(), SpawnError> {
        spawn(future);
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

        spawn(async move {
            let _ = sink.close().await;
        });
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
