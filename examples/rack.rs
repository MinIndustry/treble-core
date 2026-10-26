//! Render a `.trbl` patch without a device: cargo run --example rack --no-default-features
use treble::{
    Note,
    core::graph::System,
    patch::{ModuleRegistry, PatchRuntime, PatchSession, parse_document},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let json = match std::env::args().nth(1) {
        Some(path) => std::fs::read_to_string(path)?,
        None => include_str!("fixtures/rack.trbl").to_owned(),
    };
    let document = parse_document(&json)?;
    let mut session = PatchSession::new(ModuleRegistry::builtins(), document)?;
    let mut system = System::silent();
    let mut runtime = PatchRuntime::default();

    // A snapshot can be moved to a background worker. Installation is explicit.
    let prepared = session.snapshot().prepare(48000.0, 256)?;
    runtime.apply(&mut system, session.commit(prepared)?)?;
    if matches!(
        session.document().instrument.voice,
        treble::patch::metadata::VoiceConfiguration::Monophonic
    ) {
        runtime.apply(&mut system, session.note_on(Note::from_midi(69), 0.8)?)?;
    }
    system.run();
    let block = system.get_sink(0)?.consume();
    let peak = block
        .iter()
        .flat_map(|frame| frame.iter())
        .map(|x| x.abs())
        .fold(0.0, f32::max);
    println!("Rendered {} stereo frames, peak {peak:.4}", block.len());
    runtime.apply(&mut system, session.all_notes_off()?)?;
    Ok(())
}
