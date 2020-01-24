use futures::{lock::Mutex, SinkExt, StreamExt};
use pharos::{Observable, Pharos};
use std::sync::Arc;
use vessels::{
    channel::IdChannel,
    core::{hal::network::Server, run, spawn},
    format::Cbor,
    kind::{Infallible, Stream, TransportError},
    replicate::{Share, Shared},
};
use vessels_chat::{Chat, Message};

struct VesselsChat {
    messages: Vec<Message>,
    sink: Arc<Mutex<Pharos<Message>>>,
}

impl VesselsChat {
    fn new() -> Self {
        VesselsChat {
            messages: vec![],
            sink: Arc::new(Mutex::new(Pharos::new(0))),
        }
    }
}

impl Chat for VesselsChat {
    fn messages(&self) -> Infallible<(Vec<Message>, Stream<Result<Message, TransportError>>)> {
        let messages = self.messages.clone();
        let sink = self.sink.clone();
        Box::pin(async move {
            let stream = sink
                .lock()
                .await
                .observe(Default::default())
                .unwrap()
                .map(Ok);
            Ok((
                messages,
                Box::pin(stream) as Stream<Result<Message, TransportError>>,
            ))
        })
    }
    fn send(&mut self, message: Message) -> Infallible<()> {
        self.messages.push(message.clone());
        self.messages.truncate(100);
        let sink = self.sink.clone();
        spawn(async move {
            sink.lock().await.send(message).await.unwrap();
        });
        Box::pin(async move { Ok(()) })
    }
}

fn main() {
    let mut server = Server::new().unwrap();
    let chat = Shared::new(Box::new(VesselsChat::new()) as Box<dyn Chat>);

    run(async move {
        server
            .listen::<Box<dyn Chat>, IdChannel, Cbor>(
                "127.0.0.1:61200".parse().unwrap(),
                Box::new(move || {
                    let chat = chat.share();
                    Box::pin(async move { Box::new(chat.share()) as Box<dyn Chat> })
                }),
            )
            .await
            .unwrap()
    });
}
