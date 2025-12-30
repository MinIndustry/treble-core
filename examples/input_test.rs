use std::io;

#[cfg(not(feature = "linux"))]
fn main() -> io::Result<()> {
    println!("Missing feature linux for this example");
    Ok(())
}

#[cfg(feature = "linux")]
fn main() -> io::Result<()> {
    use evdev::EventType;

    use rustic::inputs;
    use rustic::prelude::App;

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
                    log::info!("Error fetching events");
                }
            }
        },
        None => {
            println!("No keyboard could be found");
        }
    }

    Ok(())
}
