use std::io;
use std::sync::mpsc;
use std::thread;

use termion::event::Key;
use termion::input::TermRead;

use std::time::Duration;

pub enum Event<I> {
    Input(I),
}

pub struct Events {
    rx: mpsc::Receiver<Event<Key>>,
}

impl Events {
    pub fn new() -> Events {
        let (tx, rx) = mpsc::channel();
        let tx = tx.clone();
        thread::spawn(move || {
            let stdin = io::stdin();
            for evt in stdin.keys() {
                match evt {
                    Ok(key) => {
                        if let Err(_) = tx.send(Event::Input(key)) {
                            return;
                        }
                    }
                    Err(_) => {}
                }
            }
        });
        Events { rx }
    }

    pub fn next(&self) -> Result<Event<Key>, mpsc::RecvTimeoutError> {
        self.rx.recv_timeout(Duration::from_millis(50))
    }
}
