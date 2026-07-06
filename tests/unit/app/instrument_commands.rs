//! Instrument registry management through the command system.
//! These run without starting the audio engine — `Instantiate` only
//! recompiles + hot-swaps when the engine is running.

use treble::app::commands::{Command, InstrumentCommand};
use treble::instruments::prelude::FxSpec;
use treble::prelude::App;

#[test]
fn register_valid_spec_through_command() {
    let mut app = App::new();
    let mut spec = app.registry.get("kick").unwrap().clone();
    spec.name = "my-kick".into();

    app.send(Command::Instrument(InstrumentCommand::Register(spec)))
        .expect("valid spec registers");
    assert!(app.registry.get("my-kick").is_some());
}

#[test]
fn register_invalid_spec_fails_at_definition_time() {
    let mut app = App::new();
    let mut spec = app.registry.get("kick").unwrap().clone();
    spec.name = "broken".into();
    spec.fx.push(FxSpec {
        type_id: "NotAFilter".into(),
        params: Default::default(),
    });

    let result = app.send(Command::Instrument(InstrumentCommand::Register(spec)));
    assert!(result.is_err(), "unknown filter must be rejected");
    assert!(app.registry.get("broken").is_none());
}

#[test]
fn instantiate_is_idempotent_and_tracked_by_name() {
    let mut app = App::new();

    app.send(Command::Instrument(InstrumentCommand::Instantiate {
        name: "kick".into(),
    }))
    .expect("built-in instantiates");
    let idx = app.instrument_idx("kick").expect("kick has a slot");

    // Second instantiate returns the same slot instead of duplicating.
    app.send(Command::Instrument(InstrumentCommand::Instantiate {
        name: "kick".into(),
    }))
    .expect("re-instantiating is a no-op");
    assert_eq!(app.instrument_idx("kick"), Some(idx));
    assert_eq!(app.audio_graph.len(), 1);
}

#[test]
fn instantiate_unknown_name_fails() {
    let mut app = App::new();
    let result = app.send(Command::Instrument(InstrumentCommand::Instantiate {
        name: "does-not-exist".into(),
    }));
    assert!(result.is_err());
}

#[test]
fn unregister_through_command() {
    let mut app = App::new();
    app.send(Command::Instrument(InstrumentCommand::Unregister {
        name: "bell".into(),
    }))
    .expect("built-in unregisters");
    assert!(app.registry.get("bell").is_none());

    let result = app.send(Command::Instrument(InstrumentCommand::Unregister {
        name: "bell".into(),
    }));
    assert!(result.is_err(), "double unregister must fail");
}
