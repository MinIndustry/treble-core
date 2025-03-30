#[cfg(feature = "meta")]
mod meta;

mod amplifier;
mod clipper;
mod combinator;
mod delay;
mod moving_average;
pub mod pass;
mod resonant_bandpass;
mod structural;
mod tremolo;

pub mod prelude {
    pub use super::amplifier::*;
    pub use super::clipper::*;
    pub use super::combinator::*;
    pub use super::delay::*;
    pub use super::moving_average::*;
    pub use super::pass::*;
    pub use super::resonant_bandpass::*;
    pub use super::structural::*;
    pub use super::tremolo::*;
}
