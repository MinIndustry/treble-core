//! Parameter sweeps declared through the `App` name-based API.
//! These run without starting the audio engine — declaring a sweep only
//! records data; the graph build resolves it to a node.

use std::collections::HashMap;

use treble::app::prelude::{AutomationTarget, ParameterRamp, RampCurve};
use treble::instruments::prelude::FxSpec;
use treble::prelude::App;

fn ramp() -> ParameterRamp {
    ParameterRamp {
        param: "factor".into(),
        from: 1.0,
        to: 0.0,
        start_frame: 0,
        end_frame: 4_096,
        curve: RampCurve::Linear,
    }
}

/// Register a kick carrying one gain stage, so slot fx index 0 exists.
fn app_with_gain_kick() -> App {
    let mut app = App::new();
    let mut spec = app.registry.get("kick").unwrap().clone();
    spec.name = "gain-kick".into();
    spec.fx.push(FxSpec {
        type_id: "GainFilter".into(),
        params: HashMap::from([("factor".to_string(), 1.0f32)]),
    });
    app.register_spec(spec).expect("valid spec registers");
    app.instantiate_as("bd", "gain-kick")
        .expect("registered spec instantiates");
    app
}

#[test]
fn an_instrument_sweep_is_addressed_by_instance_name() {
    let mut app = app_with_gain_kick();
    let slot = app.instrument_idx("bd").expect("bd has a slot");

    app.automate_instrument_fx("bd", 0, ramp())
        .expect("bd is live");
    assert_eq!(
        app.automations()[0].target,
        AutomationTarget::InstrumentFx { slot, fx_index: 0 }
    );

    let system = app.audio_graph.compile(44_100.0).expect("graph compiles");
    assert_eq!(system.automations().len(), 1);
    assert_eq!(
        app.audio_graph.instrument_fx_map.get(&(slot, 0)),
        Some(&system.automations()[0].node)
    );
}

#[test]
fn an_unknown_instance_name_is_rejected() {
    let mut app = App::new();
    assert!(app.automate_instrument_fx("nobody", 0, ramp()).is_err());
    assert!(app.automations().is_empty());
}

#[test]
fn a_bus_sweep_resolves_whichever_order_it_is_declared_in() {
    let mut app = app_with_gain_kick();
    // Declared before the bus exists: the build is what resolves the name, so
    // a consumer may replace bus and sweep sets independently.
    app.automate_bus_fx("drums", 0, ramp());
    app.set_buses(vec![(
        "drums".to_string(),
        vec![FxSpec {
            type_id: "GainFilter".into(),
            params: HashMap::from([("factor".to_string(), 1.0f32)]),
        }],
        vec!["bd".to_string()],
    )]);

    let system = app.audio_graph.compile(44_100.0).expect("graph compiles");
    assert_eq!(system.automations().len(), 1);
    assert_eq!(
        app.audio_graph.bus_fx_map.get(&("drums".to_string(), 0)),
        Some(&system.automations()[0].node)
    );
}

#[test]
fn clearing_drops_the_declared_set() {
    let mut app = app_with_gain_kick();
    app.automate_instrument_fx("bd", 0, ramp())
        .expect("bd live");
    app.automate_bus_fx("drums", 0, ramp());
    assert_eq!(app.automations().len(), 2);

    app.clear_automations();
    assert!(app.automations().is_empty());
    assert!(
        app.audio_graph
            .compile(44_100.0)
            .expect("graph compiles")
            .automations()
            .is_empty()
    );
}
