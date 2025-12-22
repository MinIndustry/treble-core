mod hihat;
mod kick;
mod snare;

pub use hihat::HiHat;
pub use kick::Kick;
pub use snare::Snare;

pub struct DrumKit {
    hi_hat: HiHat,
    kick: Kick,
    snare: Snare,
}

impl DrumKit {
    pub fn new() -> Result<Self, String> {
        Ok(DrumKit {
            hi_hat: HiHat::new()?,
            kick: Kick::new(),
            snare: Snare::new(),
        })
    }
}
