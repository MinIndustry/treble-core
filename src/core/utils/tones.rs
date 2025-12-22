//! The tones module contains the mapping between musical notes and their corresponding frequencies.
//! It can be used to generate audio signals for different musical notes.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Hash, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub enum NOTES {
    C = 0,
    CS = 1,
    D = 2,
    DS = 3,
    E = 4,
    F = 5,
    FS = 6,
    G = 7,
    GS = 8,
    A = 9,
    AS = 10,
    B = 11,
}

impl From<u8> for NOTES {
    fn from(value: u8) -> Self {
        let notes = [
            NOTES::C,
            NOTES::CS,
            NOTES::D,
            NOTES::DS,
            NOTES::E,
            NOTES::F,
            NOTES::FS,
            NOTES::G,
            NOTES::GS,
            NOTES::A,
            NOTES::AS,
            NOTES::B,
        ];

        notes[(value as usize) % notes.len()]
    }
}

pub const TONES_FREQ: [[f32; 9]; 12] = [
    [
        16.35, 32.70, 65.41, 130.81, 261.63, 523.25, 1046.50, 2093.00, 4186.00,
    ],
    [
        17.32, 34.65, 69.30, 138.59, 277.18, 554.37, 1108.73, 2217.46, 4434.92,
    ],
    [
        18.35, 36.71, 73.42, 146.83, 293.66, 587.33, 1174.66, 2349.32, 4698.63,
    ],
    [
        19.45, 38.89, 77.78, 155.56, 311.13, 622.25, 1244.51, 2489.00, 4978.00,
    ],
    [
        20.60, 41.20, 82.41, 164.81, 329.63, 659.25, 1318.51, 2637.00, 5274.00,
    ],
    [
        21.83, 43.65, 87.31, 174.61, 349.23, 698.46, 1396.91, 2793.83, 5587.65,
    ],
    [
        23.12, 46.25, 92.50, 185.00, 369.99, 739.99, 1479.98, 2959.96, 5919.91,
    ],
    [
        24.50, 49.00, 98.00, 196.00, 392.00, 783.99, 1567.98, 3135.96, 6271.93,
    ],
    [
        25.96, 51.91, 103.83, 207.65, 415.30, 830.61, 1661.22, 3322.44, 6644.88,
    ],
    [
        27.50, 55.00, 110.00, 220.00, 440.00, 880.00, 1760.00, 3520.00, 7040.00,
    ],
    [
        29.14, 58.27, 116.54, 233.08, 466.16, 932.33, 1864.66, 3729.31, 7458.62,
    ],
    [
        30.87, 61.74, 123.47, 246.94, 493.88, 987.77, 1975.53, 3951.00, 7902.13,
    ],
];
