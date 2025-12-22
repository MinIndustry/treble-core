# Rustic Examples

This directory contains a comprehensive collection of examples demonstrating various features and capabilities of the Rustic audio synthesis library. The examples are organized by complexity and functionality to help you learn and explore the system.

## Available Examples

All examples are located in the `examples/` directory and can be run using `cargo run --example <name>`.

### Basic Examples

Simple examples that demonstrate fundamental concepts:

- **`one_note`** - Play a single note with basic envelope shaping
- **`simple_melody`** - Create and play a basic melody sequence

```bash
cargo run --example one_note
cargo run --example simple_melody
```

### Instrument Examples

Examples showcasing different instrument types and their capabilities:

- **`keyboard_demo`** - Interactive keyboard instrument with live input (requires `input` feature)
- **`drum_machine`** - Drum pattern sequencer with multiple percussion sounds

```bash
cargo run --example keyboard_demo --features="input"
cargo run --example drum_machine
```

### Analysis Examples

Examples demonstrating audio analysis and visualization capabilities:

- **`waveform_plot`** - Generate and visualize audio waveforms (requires `plotting` feature)

```bash
cargo run --example waveform_plot --features="plotting"
```

### Advanced Examples

Complex examples showing sophisticated audio processing techniques:

- **`graph_processing`** - Audio signal graph processing and routing
- **`score_playback`** - Musical score parsing and playback
- **`pitch_bend`** - Real-time pitch modulation and envelope control
- **`audio_merger`** - Multi-source audio mixing and processing
- **`audio_pipe`** - Audio pipeline processing and effects

```bash
cargo run --example graph_processing
cargo run --example score_playback
cargo run --example pitch_bend
cargo run --example audio_merger
cargo run --example audio_pipe
```

### Development Tools

Utility examples for testing and development:

- **`input_test`** - Test hardware input devices (requires `input` feature)
- **`meta_test`** - Demonstrate metadata and introspection system (requires `meta` feature)

```bash
cargo run --example input_test --features="input"
cargo run --example meta_test --features="meta"
```

## Running Examples

### Basic Usage

Most examples can be run directly without additional features:

```bash
cargo run --example <example_name>
```

### With Features

Some examples require specific features to be enabled:

```bash
# For examples requiring input handling
cargo run --example keyboard_demo --features="input"

# For examples with visualization
cargo run --example waveform_plot --features="plotting"

# For examples using metadata system
cargo run --example meta_test --features="meta"

# Multiple features
cargo run --example advanced_demo --features="plotting,input,meta"
```

### Platform-Specific Features

Some examples may require platform-specific features:

```bash
# On Linux
cargo run --example keyboard_demo --features="linux"

# On macOS
cargo run --example keyboard_demo --features="macos"

# On Windows
cargo run --example keyboard_demo --features="windows"
```

## Example Categories

### Learning Path

**Beginners** should start with:
1. `one_note` - Understanding basic note generation
2. `simple_melody` - Working with sequences
3. `drum_machine` - Exploring different instrument types

**Intermediate** users should explore:
1. `keyboard_demo` - Interactive real-time audio
2. `waveform_plot` - Audio analysis and visualization
3. `pitch_bend` - Advanced modulation techniques

**Advanced** users can study:
1. `graph_processing` - Complex audio routing
2. `audio_merger` - Multi-source processing
3. `score_playback` - Complete musical compositions

### By Feature

**Core Audio Processing:**
- `one_note`, `simple_melody`, `pitch_bend`

**Instrument Systems:**
- `keyboard_demo`, `drum_machine`

**Real-time Interaction:**
- `keyboard_demo`, `input_test`

**Visualization and Analysis:**
- `waveform_plot`

**Signal Processing:**
- `graph_processing`, `audio_merger`, `audio_pipe`

**Musical Notation:**
- `score_playback`

**Development Tools:**
- `input_test`, `meta_test`

## Common Patterns

### Audio Output

Most examples use the `rodio` crate for audio output:

```rust
use rodio::{OutputStream, Sink};

let (_stream, stream_handle) = OutputStream::try_default().unwrap();
let sink = Sink::try_new(&stream_handle).unwrap();
```

### Instrument Setup

Typical instrument initialization:

```rust
use rustic::instruments::prelude::*;
use rustic::instruments::Instrument;

let mut keyboard = Keyboard::<4>::new(); // 4-voice polyphony
keyboard.start_note(note, velocity);
```

### Audio Generation Loop

Common pattern for generating audio samples:

```rust
let mut samples = Vec::new();
for _ in 0..sample_count {
    instrument.tick();
    samples.push(instrument.get_output());
}
```

## Dependencies

Examples may require additional dependencies beyond the core Rustic library:

- **Audio Output:** `rodio` - Cross-platform audio playback
- **Input Handling:** `evdev` (Linux) - Hardware input device access  
- **Visualization:** `plotters` - Chart and graph generation
- **Logging:** `log`, `colog` - Runtime logging and debugging
- **Command Line:** `clap` - Argument parsing for interactive examples

## Troubleshooting

### Audio Issues

- **No audio output:** Check that your system has available audio devices
- **Crackling/distortion:** Try increasing buffer sizes or reducing CPU load
- **Latency issues:** Adjust sample rates and buffer configurations

### Input Issues

- **Keyboard not detected:** Ensure proper permissions for `/dev/input` access on Linux
- **Events not received:** Check that the input device is not being used by other applications

### Compilation Issues

- **Missing features:** Enable required features with `--features="feature_name"`
- **Platform dependencies:** Ensure platform-specific libraries are installed

### Performance Issues

- **High CPU usage:** Some examples are computationally intensive; consider reducing sample rates for testing
- **Memory usage:** Long-running examples may accumulate samples; implement proper cleanup for production use

## Contributing

When adding new examples:

1. **Choose appropriate category:** Place in the most relevant subdirectory
2. **Document dependencies:** List required features and external dependencies
3. **Include comments:** Explain key concepts and techniques
4. **Follow patterns:** Use consistent code style and error handling
5. **Update this README:** Add your example to the appropriate sections

## See Also

- [Core Module Documentation](../docs/modules/core.md)
- [Instruments Documentation](../docs/modules/instruments.md)
- [Score System Documentation](../docs/modules/score.md)
- [Main README](../README.md) - Project overview and setup instructions