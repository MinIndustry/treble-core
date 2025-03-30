# Binaries
Folder: `bin`
This folder contains the different binary main files.
These are used mainly for testing purposes.

## Input
Used to test input method off the project. The binary lists all keyboards and tries to find the correct one.
It then displays events, trying to map them to actions in the project.

## Music
Produces music using the project's envelopes & generators. Used to test that the system is working correctly, outputting the audio
to a correct device and that the system (under low load at least) is fast enough to produce audio in real-time.

## Pipe
A mockup of the pipe & filter system. Simulates the execution of a pipe & filter system, with a feedback loop.

# Core
Folder: `core`
Contains the core functionality of the project. The App structure is defined here, as well as the main loop.
