use abstract_ws::SocketProvider;
use abstract_ws_tungstenite::{Provider, Socket, WsError};
use futures::{
    channel::mpsc::unbounded,
    stream::{SplitSink, SplitStream},
    FutureExt, SinkExt, StreamExt,
};
use iui::controls::{Button, Entry, HorizontalBox, Label, TextEntry, VerticalBox};
use iui::prelude::*;
use protocol_mve_transport::Coalesce;
use smol::{block_on, Async};
use std::net::TcpStream;

use vessels_chat_demo::{util::Spawner, ErasedChat};

fn main() {
    block_on(
        Provider::new()
            .connect("ws://127.0.0.1:8080".parse().unwrap())
            .then(|connection| {
                let connection = connection.unwrap();
                let (sender, receiver) = connection.split();
                Coalesce::<
                    WsError,
                    Spawner,
                    SplitStream<Socket<Async<TcpStream>>>,
                    SplitSink<Socket<Async<TcpStream>>, Vec<u8>>,
                    ErasedChat,
                >::new(receiver, sender, Spawner)
            })
            .then(|chat| async move {
                let mut chat = chat.unwrap();

                let mut messages = chat.messages();

                while let Some(message) = messages.next().await {
                    println!("{:?}", message);
                }
            }),
    );
}
