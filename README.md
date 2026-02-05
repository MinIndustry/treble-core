# Rustic

## Project Overview

Rustic is designed as a frontend-agnostic core library that provides composable primitives for audio synthesis. The architecture separates concerns between real-time audio processing, DSP operations, and user interface, allowing the same synthesis engine to be used by GUI applications, command-line tools, and embedded systems.

The framework implements a lock-free three-thread architecture for audio processing that ensures real-time safety: a command thread handles user input, a render thread performs DSP synthesis, and a CPAL callback thread manages hardware audio output. This design prevents audio dropouts while maintaining responsive interaction.
