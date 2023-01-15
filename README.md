# joystick_rust
Simple linux joystick-api library in rust

## Usage
```rust
  let mut reader = JoystickReader::new("/dev/input/js0");

  loop {
    // or read_event_now() if you want a non blocking read
    match reader.read_event() {
      Ok(event) => println!("Event: {:?}", event),
      Err(error) => eprintln!("Error: {:?}", error),
    }
  }
```

`JoystickReader::read_event()` returns an `Result<JoystickEvent, JoystickError>`.
`JoystickEvent` is an enum with two variants:
- Analog - for analog events
- Button - for button events

