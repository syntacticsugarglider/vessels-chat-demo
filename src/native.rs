use abstract_ws::SocketProvider;
use abstract_ws_tungstenite::{Provider, Socket, WsError};
use futures::{
    channel::mpsc::unbounded,
    stream::{SplitSink, SplitStream},
    task::SpawnExt,
    FutureExt, SinkExt, StreamExt,
};
use iui::controls::{Button, Entry, HorizontalBox, Label, TextEntry, VerticalBox};
use iui::prelude::*;
use protocol_mve_transport::Coalesce;
use smol::{block_on, run, Async};
use std::net::{TcpListener, TcpStream};
use std::thread;

use vessels_chat_demo::{
    util::{CloseOnDrop, Spawner},
    ErasedChat,
};

fn main() {
    let ui = UI::init().expect("Couldn't initialize UI library");
    let mut win = Window::new(&ui, "Vessels Chat", 450, 600, WindowType::NoMenubar);

    let mut vbox = VerticalBox::new(&ui);
    vbox.set_padded(&ui, true);

    let mut hbox = HorizontalBox::new(&ui);

    let entry = Entry::new(&ui);

    let mut button = Button::new(&ui, "Send");

    let (mut sender, mut receiver) = unbounded();

    let (outgoing_sender, mut outgoing_receiver) = unbounded();

    let ui_handle = ui.clone();
    let mut entry_handle = entry.clone();

    button.on_clicked(&ui, {
        move |_| {
            let val = entry_handle.value(&ui_handle);
            if val != "" {
                outgoing_sender.unbounded_send(val).unwrap();
                entry_handle.set_value(&ui_handle, &"");
            }
        }
    });

    let num_threads = num_cpus::get().max(1);

    for _ in 0..num_threads {
        thread::spawn(|| smol::run(futures::future::pending::<()>()));
    }

    thread::spawn(move || {
        block_on(
            Provider::new()
                .connect("ws://127.0.0.1:8080".parse().unwrap())
                .then(|connection| {
                    let connection = connection.unwrap();
                    let (sender, receiver) = connection.split();
                    Coalesce::<
                        Spawner,
                        SplitStream<Socket<Async<TcpStream>>>,
                        CloseOnDrop<SplitSink<Socket<Async<TcpStream>>, Vec<u8>>, Vec<u8>>,
                        ErasedChat,
                    >::new(receiver, CloseOnDrop::new(sender), Spawner)
                })
                .then(|chat| async move {
                    let mut chat = chat.unwrap();

                    let mut messages = chat.messages(100);

                    Spawner
                        .spawn(async move {
                            while let Some(message) = outgoing_receiver.next().await {
                                chat.send(message).await.unwrap();
                            }
                        })
                        .unwrap();

                    while let Some(message) = messages.next().await {
                        sender.send(message.unwrap()).await.unwrap();
                    }
                }),
        )
    });

    let mut label = Label::new(&ui, &"");

    vbox.append(&ui, label.clone(), LayoutStrategy::Stretchy);
    hbox.append(&ui, entry, LayoutStrategy::Stretchy);
    hbox.append(&ui, button, LayoutStrategy::Compact);
    vbox.append(&ui, hbox, LayoutStrategy::Compact);

    win.set_child(&ui, vbox);

    win.show(&ui);

    let mut label_text = format!("");

    let mut event_loop = ui.event_loop();

    let ui_handle = ui.clone();

    let mut done = false;

    event_loop.on_tick(&ui, move || {
        if !done {
            match receiver.try_next() {
                Ok(Some(message)) => {
                    label_text.push_str(&format!("{}\n", message));
                    label.set_text(&ui_handle, &label_text);
                }
                Ok(None) => done = true,
                _ => {}
            }
        }
    });

    event_loop.run_delay(&ui, 200);
}
