use std::env;
use std::sync::{Mutex, Arc};
use std::time::Duration;
use std::{fs::File, io::Read};
use std::sync::mpsc::{self, Sender, Receiver};
use std::thread;

const BUTTON_EVENT_TYPE: u8 = 1;
const ANALOG_EVENT_TYPE: u8 = 2;

type JoystickBuffer = [u8; 8];

#[derive(Debug)]
enum JoystickEvent {
    Analog {
        time: u32,
        value: i16,
        axis: u8,
    },

    Button {
        time: u32,
        pressed: bool,
        button: u8,
    }
}

#[derive(Debug)]
enum JoystickError {
    NoEvent,
    Disconnected,
    UnknownEventType(u8),
}

enum JoystickReaderState {
    WaitingJoystick,
    ReadingInput {
        handle: File,
        sender: Sender<JoystickBuffer>,
    },
}

struct JoystickReader {
    receiver: Receiver<JoystickBuffer>,
}

struct JoystickReaderStateMachine {
    path: String,
    state: JoystickReaderState,
}

impl JoystickReader {
    fn new(path: String) -> Self {
        let sm = JoystickReaderStateMachine {
            path,
            state: JoystickReaderState::WaitingJoystick,
        };

        let sm_ref = Arc::new(Mutex::new(sm));
        let sm_ref_copy = Arc::clone(&sm_ref);

        let (tx, rx) = mpsc::channel::<JoystickBuffer>();

        let reader = JoystickReader {
            receiver: rx,
        };
        
        thread::spawn(move || loop {
            let mut sm_mutex = sm_ref_copy.lock().unwrap();
            let mut sm = &mut *sm_mutex;

            match &mut sm.state {
                JoystickReaderState::WaitingJoystick => {
                    match File::open(&sm.path) {
                        Ok(handle) => {
                            sm.state = JoystickReaderState::ReadingInput { handle, sender: tx.clone() };
                        },
                        Err(_) => continue,
                    }
                },

                JoystickReaderState::ReadingInput { handle, sender } => {
                    let mut buffer = [0u8; 8];
                    let result = handle.read_exact(&mut buffer)
                        .map(|_| buffer);

                    match result {
                        Ok(buffer) => {
                            let result = sender.send(buffer);
                            if result.is_err() {
                                sm.state = JoystickReaderState::WaitingJoystick;
                            }
                        },
                        Err(_) => {
                            sm.state = JoystickReaderState::WaitingJoystick;
                        },
                    }
                },
            }
        });

        reader
    }

    fn read_event(&mut self) -> Result<JoystickEvent, JoystickError> {
        self.receiver.recv()
            .map_err(|_| JoystickError::Disconnected)
            .and_then(|buffer| self.read_event_from_bytes(&buffer))
    }

    fn read_event_now(&mut self) -> Result<JoystickEvent, JoystickError> {
        self.receiver.try_recv()
            .map_err(|error| match error {
                mpsc::TryRecvError::Empty => JoystickError::NoEvent,
                mpsc::TryRecvError::Disconnected => JoystickError::Disconnected,
            })
            .and_then(|buffer| self.read_event_from_bytes(&buffer))
    }

    fn read_event_from_bytes(&mut self, buffer: &[u8; 8]) -> Result<JoystickEvent, JoystickError> {
        let time = u32::from_le_bytes((&buffer[0..4]).try_into().unwrap());
        let value = i16::from_le_bytes((&buffer[4..6]).try_into().unwrap());
        let event_type = buffer[6] & 0b00000011; // Clearing the INIT event mask
        let number = buffer[7];

        match event_type {
            BUTTON_EVENT_TYPE => Ok(JoystickEvent::Button {
                time,
                pressed: value == 1,
                button: number,
            }),

            ANALOG_EVENT_TYPE => Ok(JoystickEvent::Analog {
                time,
                value,
                axis: number,
            }),

            _ => Err(JoystickError::UnknownEventType(event_type)),
        }
    }
}


fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <joystick-device-path>", args[0]);
        return;
    }

    let mut reader = JoystickReader::new(args[1].clone());

    loop {
        match reader.read_event_now() {
            Ok(event) => println!("{:?}", event),
            Err(error) => eprintln!("Error: {:?}", error),
        }

        thread::sleep(Duration::from_millis(10));
    }
}
