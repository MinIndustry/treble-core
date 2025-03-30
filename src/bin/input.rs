use evdev::EventType;
use log::info;
use std::io;

use rustic::inputs;
use rustic::prelude::App;

fn main() -> io::Result<()> {
    let app: App = App::init();

    let mapping = app.get_key_mapping();

    match inputs::keyboard::find_keyboard() {
        Some(mut keyboard) => loop {
            match keyboard.fetch_events() {
                Ok(events) => {
                    for event in events {
                        if event.event_type() == EventType::KEY && event.value() != 2 {
                            if mapping.contains_key(&event.code()) {
                                println!("{:?}", mapping[&event.code()]);
                            } else {
                                println!(
                                    "Event:\n\t{:?}\t{:?}\t{:?}",
                                    event.event_type(),
                                    event.code(),
                                    event.value()
                                );
                            }
                        }
                    }
                }
                Err(_) => {
                    info!("Error fetching events");
                }
            }
        },
        None => {
            println!("No keyboard could be found");
        }
    }

    Ok(())
}
