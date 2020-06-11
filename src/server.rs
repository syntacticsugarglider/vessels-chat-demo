use abstract_ws::ServerProvider;
use abstract_ws_tungstenite::{Server, Socket, WsError};
use core::{
    iter::repeat,
    pin::Pin,
    task::{Context, Poll},
};
use futures::{
    channel::mpsc::{unbounded, UnboundedSender},
    future::ready,
    lock::Mutex,
    sink::drain,
    stream::{iter, FuturesUnordered, SplitSink, SplitStream, Stream},
    Future, FutureExt, SinkExt, StreamExt,
};
use protocol_mve_transport::Unravel;
use smol::{block_on, Async, Task};
use std::{
    net::{TcpListener, TcpStream},
    sync::Arc,
};

use vessels_chat_demo::{util::Spawner, Chat, ErasedChat, Error};

pub struct ChatServer;

impl Chat for ChatServer {
    type Messages = Pin<Box<dyn Stream<Item = Result<String, Error>> + Send>>;

    fn messages(&mut self) -> Self::Messages {
        Box::pin(iter(repeat(Ok("test".to_owned()))))
    }
}

fn main() {
    block_on(async move {
        let mut connections =
            Server::<Async<TcpListener>>::new().listen("127.0.0.1:8080".parse().unwrap());

        while let Some(connection) = connections.next().await {
            let connection = if let Ok(connection) = connection {
                connection
            } else {
                continue;
            };

            let (sender, receiver) = connection.split();

            Task::spawn(
                Unravel::<
                    WsError,
                    Spawner,
                    SplitStream<Socket<Async<TcpStream>>>,
                    SplitSink<Socket<Async<TcpStream>>, Vec<u8>>,
                    ErasedChat,
                >::new(receiver, sender, Spawner, Box::new(ChatServer))
                .then(|out| {
                    println!("{:?}", out);
                    ready(())
                }),
            )
            .detach();
        }
    });
}
