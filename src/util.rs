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

#[derive(Clone)]
pub struct Spawner;

impl Spawn for Spawner {
    fn spawn_obj(&self, future: FutureObj<'static, ()>) -> Result<(), SpawnError> {
        Task::spawn(future).detach();
        Ok(())
    }
}
