use vessels::{
    kind::{Infallible, Stream, TransportError},
    object, Kind,
};

#[derive(Kind, Debug, Clone)]
pub struct Message {
    pub content: String,
}

#[object]
pub trait Chat {
    fn messages(&self) -> Infallible<(Vec<Message>, Stream<Result<Message, TransportError>>)>;
    fn send(&mut self, message: Message) -> Infallible<()>;
}
