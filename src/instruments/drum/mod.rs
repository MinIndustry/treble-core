mod hihat;
mod kick;
mod snare;

pub use hihat::HiHat;
pub use kick::Kick;
pub use snare::Snare;

pub struct DrumKit {
    _hi_hat: HiHat,
    _kick: Kick,
    _snare: Snare,
}

impl DrumKit {
    pub fn new() -> Result<Self, String> {
        Ok(DrumKit {
            _hi_hat: HiHat::new()?,
            _kick: Kick::new(),
            _snare: Snare::new(),
        })
    }
}
