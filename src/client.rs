mod event;

use std::{io::{self, Write}, collections::VecDeque};

use event::{Event, Events};
use termion::cursor::Goto;
use termion::event::Key;
use termion::input::MouseTerminal;
use termion::raw::IntoRawMode;
use termion::screen::AlternateScreen;
use tui::backend::TermionBackend;
use tui::layout::{Constraint, Direction, Layout};
use tui::widgets::{Block, Borders, List, Paragraph, Text, Widget};
use tui::Terminal;
use unicode_width::UnicodeWidthStr;

use futures::{channel::mpsc::unbounded, StreamExt};
use std::sync::{Arc, Mutex};
use vessels::{
    channel::IdChannel,
    core::{hal::network::Client, run, spawn},
    format::Cbor,
};
use vessels_chat::{Chat, Message};

struct App {
    input: String,
    messages: Arc<Mutex<VecDeque<String>>>,
}

impl Default for App {
    fn default() -> App {
        App {
            input: String::new(),
            messages: Arc::new(Mutex::new(VecDeque::new())),
        }
    }
}

fn main() -> Result<(), anyhow::Error> {
    let stdout = io::stdout().into_raw_mode()?;
    let stdout = MouseTerminal::from(stdout);
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let events = Events::new();

    let mut app = App::default();

    let messages = app.messages.clone();

    let (sender, mut receiver) = unbounded();

    let url = std::env::args().skip(1).next().expect("pass url with port as arg").parse().unwrap();

    std::thread::spawn(move || {
        run(async move {
            let mut network = Client::new().unwrap();
            let mut chat = network
                .connect::<Box<dyn Chat>, IdChannel, Cbor>(url)
                .await
                .unwrap();
            let (initial, mut stream) = chat.messages().await.unwrap();
            *messages.lock().unwrap() = initial.into_iter().map(|m| m.content).rev().collect();
            spawn(async move {
                while let Some(item) = receiver.next().await {
                    chat.send(Message { content: item }).await.unwrap();
                }
            });
            while let Some(item) = stream.next().await {
                messages.lock().unwrap().push_front(item.unwrap().content);
            }
        });
    });

    loop {
        terminal.draw(|mut f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(2)
                .constraints(
                    [
                        Constraint::Length(1),
                        Constraint::Length(3),
                        Constraint::Min(1),
                    ]
                    .as_ref(),
                )
                .split(f.size());
            let mut messages = app.messages.lock().unwrap();
            messages.truncate(100);
            let messages = messages.iter().map(|m| Text::raw(m.to_owned()));
            List::new(messages)
                .block(Block::default().borders(Borders::ALL))
                .render(&mut f, chunks[2]);
            let help_message = "exit: q | send: enter";
            Paragraph::new([Text::raw(help_message)].iter()).render(&mut f, chunks[0]);
            Paragraph::new([Text::raw(&app.input)].iter())
                .block(Block::default().borders(Borders::ALL))
                .render(&mut f, chunks[1]);
        })?;

        write!(
            terminal.backend_mut(),
            "{}",
            Goto(4 + app.input.width() as u16, 5)
        )?;
        io::stdout().flush().ok();

        if let Ok(event) = events.next() {
            match event {
                Event::Input(input) => match input {
                    Key::Char('\n') => {
                        let message: String = app.input.drain(..).collect();
                        sender.unbounded_send(message).unwrap();
                    }
                    Key::Char('q') => break,
                    Key::Char(c) => app.input.push(c),
                    Key::Backspace => {
                        app.input.pop();
                    }
                    _ => {}
                },
            };
        }
    }
    Ok(())
}
