use abstract_ws::ServerProvider;
use abstract_ws_tungstenite::{ServerConfig, TlsServer};
use core::pin::Pin;
use futures::{
    channel::mpsc::{unbounded, UnboundedSender},
    future::ready,
    lock::Mutex,
    sink::drain,
    stream::{FuturesUnordered, Stream},
    Future, FutureExt, SinkExt, StreamExt, TryStreamExt,
};
use protocol_mve_transport::Unravel;
use rustls::NoClientAuth;
use smol::{block_on, Async, Task};
use std::net::TcpListener;
use std::sync::Arc;
use std::thread;

use vessels_chat_demo::{
    util::{CloseOnDrop, Spawner, TransportError},
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

                receiver.map(|item| Ok(item))
            }
            .flatten_stream(),
        )
    }

    fn send(&mut self, message: String) -> Self::Send {
        let message = format!("({}): {}", self.id, message);

        let senders = self.senders.clone();

        Box::pin(async move {
            let mut senders = senders.lock().await;

            let stream: FuturesUnordered<_> = senders
                .iter_mut()
                .map(|sink| {
                    sink.send(message.clone()).map(|e| {
                        if let Err(e) = e {
                            println!("send error: {:?}", e);
                        }
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

    let num_threads = num_cpus::get().max(1);

    for _ in 0..num_threads {
        thread::spawn(|| smol::run(futures::future::pending::<()>()));
    }

    block_on(async move {
        let mut connections =
            TlsServer::<Async<TcpListener>>::new(Arc::new(ServerConfig::new(NoClientAuth::new())))
                .listen("0.0.0.0:443".parse().unwrap());

        let server = ChatServer {
            senders: Arc::new(Mutex::new(vec![])),
            id,
        };

        while let Some(connection) = connections.next().await {
            let server = server.duplicate(id);
            id += 1;

            let connection = if let Ok(connection) = connection.map_err(|e| println!("{:?}", e)) {
                connection
            } else {
                continue;
            };

            let (sender, receiver) = connection.split();

            let receiver = receiver.map_err(TransportError::new);
            let sender = sender.sink_map_err(TransportError::new);

            Task::spawn(
                Unravel::<_, _, _, ErasedChat>::new(
                    receiver,
                    CloseOnDrop::new(sender),
                    Spawner,
                    Box::new(server),
                )
                .then(|out| {
                    println!("{:?}", out);
                    ready(())
                }),
            )
            .detach();
        }
    });
}
