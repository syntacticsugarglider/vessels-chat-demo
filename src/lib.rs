use core::pin::Pin;
use futures::{Future, Stream};
use protocol::{allocated::ProtocolError, protocol};

pub mod util;

#[protocol]
#[derive(Debug, Clone)]
pub struct Error;

impl From<ProtocolError> for Error {
    fn from(input: ProtocolError) -> Self {
        println!("{}", input);
        Error
    }
}

#[protocol]
pub trait Chat {
    type Messages: Stream<Item = Result<String, Error>>;
    type Send: Future<Output = Result<(), Error>>;

    fn messages(&mut self, backlog: u32) -> Self::Messages;

    fn send(&mut self, message: String) -> Self::Send;
}

pub type ErasedChat = Box<
    dyn Chat<
            Messages = Pin<Box<dyn Stream<Item = Result<String, Error>> + Send>>,
            Send = Pin<Box<dyn Future<Output = Result<(), Error>> + Send>>,
        > + Send,
>;
