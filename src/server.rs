use abstract_ws::ServerProvider;
use abstract_ws_tungstenite::{Server, Socket, WsError};
use async_std::{
    net::{TcpListener, TcpStream},
    task::{block_on, spawn},
};
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
use std::sync::Arc;

use vessels_chat_demo::{
    util::{CloseOnDrop, Spawner},
    Chat, ErasedChat, Error,
};

pub struct ChatServer {
    id: u32,
    senders: Arc<Mutex<Vec<UnboundedSender<String>>>>,
}

impl ChatServer {
    fn duplicate(&self, id: u32) -> Self {
        ChatServer {
            id,
            senders: self.senders.clone(),
        }
    }
}

pub struct DebugDrop<T> {
    item: T,
}

impl<T> DebugDrop<T> {
    fn new(item: T) -> Self {
        DebugDrop { item }
    }
}

impl<T> Drop for DebugDrop<T> {
    fn drop(&mut self) {
        println!("dropped")
    }
}

impl<T: Stream + Unpin> Stream for DebugDrop<T> {
    type Item = T::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.item).poll_next(cx)
    }
}

impl Chat for ChatServer {
    type Messages = Pin<Box<dyn Stream<Item = Result<String, Error>> + Send>>;
    type Send = Pin<Box<dyn Future<Output = Result<(), Error>> + Send>>;

    fn messages(&mut self, _: u32) -> Self::Messages {
        let senders = self.senders.clone();

        Box::pin(
            async move {
                let mut senders = senders.lock().await;

                let (sender, receiver) = unbounded();

                senders.push(sender);

                DebugDrop::new(
                    iter(repeat(Ok("test".to_owned()))).chain(receiver.map(|item| {
                        println!("msg: {}", item);
                        Ok(item)
                    })),
                )
            }
            .flatten_stream(),
        )
    }

    fn send(&mut self, message: String) -> Self::Send {
        println!("send");

        let message = format!("({}): {}", self.id, message);

        let senders = self.senders.clone();

        Box::pin(async move {
            let mut senders = senders.lock().await;
            println!("sendlock");

            let stream: FuturesUnordered<_> = senders
                .iter_mut()
                .map(|sink| {
                    sink.send(message.clone()).map(|e| {
                        println!("{:?}", e);
                        Ok(())
                    })
                })
                .collect();

            stream.forward(drain()).await.unwrap();

            Ok(())
        })
    }
}

fn main() {
    let mut id = 0u32;

    block_on(async move {
        let mut connections =
            Server::<TcpListener>::new().listen("127.0.0.1:8080".parse().unwrap());

        let server = ChatServer {
            senders: Arc::new(Mutex::new(vec![])),
            id,
        };

        while let Some(connection) = connections.next().await {
            let server = server.duplicate(id);
            id += 1;

            let connection = if let Ok(connection) = connection {
                connection
            } else {
                continue;
            };

            let (sender, receiver) = connection.split();

            spawn(
                Unravel::<
                    WsError,
                    Spawner,
                    SplitStream<Socket<TcpStream>>,
                    CloseOnDrop<SplitSink<Socket<TcpStream>, Vec<u8>>, Vec<u8>>,
                    ErasedChat,
                >::new(
                    receiver,
                    CloseOnDrop::new(sender),
                    Spawner,
                    Box::new(server),
                )
                .then(|out| {
                    println!("{:?}", out);
                    ready(())
                }),
            );
        }
    });
}
