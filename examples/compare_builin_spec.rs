#[cfg(not(feature = "plotting"))]
fn main() {
    println!("This example requires the plotting feature to be enabled");
}

#[cfg(feature = "plotting")]
fn main() {
    use std::path::Path;

    use treble::instruments::Instrument;
    use treble::instruments::prelude::{InstrumentRegistry, Kick};
    use treble::instruments::spec::compile_spec;
    use treble::plotting::PlotBuilder;
    use treble::{Note, core::Block};

    let mut og_kick = Kick::new().as_system(44100.0);
    let registry = InstrumentRegistry::built_in();
    let kick_spec = registry.get("kick").expect("kick is a built-in");
    let mut kick_system = match compile_spec(kick_spec, 44100.0) {
        Ok(s) => s,
        Err(e) => {
            println!("Unable to build a kick from the spec: {e}");
            return;
        }
    };

    if let Err(e) = og_kick.compute() {
        println!("Unable to compute original kick's graph: {e}");
    }

    if let Err(e) = kick_system.compute() {
        println!("Unable to compute spec kick's graph: {e}");
    }

    og_kick.start_note(0, Note::new(treble::NOTES::A, 4), 1.0);
    kick_system.start_note(0, Note::new(treble::NOTES::A, 4), 1.0);

    let _ = kick_system.save_to_file(Path::new("kick_system.toml"));

    let og_kick_samples = (0..200)
        .map(|_| {
            og_kick.run();
            match og_kick.get_sink(0) {
                Ok(s) => s.consume(),
                Err(e) => {
                    println!("Unable to get og_kick's sink: {}", e);
                    Block::new()
                }
            }
        })
        .flatten()
        .collect::<Vec<[f32; 2]>>();

    let kick_system_samples = (0..200)
        .map(|_| {
            kick_system.run();
            match kick_system.get_sink(0) {
                Ok(s) => s.consume(),
                Err(e) => {
                    println!("Unable to get kick_system's sink: {}", e);
                    Block::new()
                }
            }
        })
        .flatten()
        .collect::<Vec<[f32; 2]>>();

    println!(
        "Kick system's max: {}",
        kick_system_samples
            .iter()
            .map(|e| e[0])
            .reduce(f32::max)
            .unwrap_or(0.0)
    );

    if let Err(e) = PlotBuilder::new()
        .title("Kick comparisons")
        .x_label("Time (s)")
        .y_label("Amplitude")
        .x_range(0.0, 1.0)
        .y_range(-1.0, 1.0)
        .add_series(
            og_kick_samples
                .iter()
                .enumerate()
                .map(|(idx, data)| (idx as f32 / 44100.0, data[0]))
                .collect::<Vec<(f32, f32)>>(),
            "OG Kick",
            Some((255, 0, 0)),
        )
        .add_series(
            kick_system_samples
                .iter()
                .enumerate()
                .map(|(idx, data)| (idx as f32 / 44100.0, data[0]))
                .collect::<Vec<(f32, f32)>>(),
            "Kick System",
            Some((0, 0, 255)),
        )
        .resolution(1920, 1080)
        .show_legend(true)
        .save("kick_comparison.png")
    {
        log::error!("Error: {}", e);
    }
}
