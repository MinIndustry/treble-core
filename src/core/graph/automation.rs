//! Block-rate parameter automation.
//!
//! A filter parameter used to change only when the graph was rebuilt, which
//! made every timed sweep — pattern-level and bus-level filter ramps — a
//! rebuild-per-step affair. Automations move that into the render loop: the
//! [`System`](super::System) evaluates each active ramp at the start frame of
//! every block and applies it through the filter's own `set_parameter`.
//!
//! Evaluation is block-rate, not per-sample (design decision D3). At 512
//! frames / 44.1 kHz that is a value change every ~11.6 ms, which is
//! inaudible for cutoff and gain travel; per-sample smoothing is a later
//! refinement and needs no change to the shape of this data.

use petgraph::prelude::NodeIndex;

/// How a ramp travels between its endpoints.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum RampCurve {
    #[default]
    Linear,
    /// Geometric travel: `from * (to/from)^t`. Pitch and cutoff are heard
    /// logarithmically, so a linear sweep from 300 Hz to 9 kHz spends most of
    /// its time in the top octave and does not sound like an even sweep.
    Exponential,
}

/// A parameter sweep against one compiled filter node.
///
/// The frames are absolute engine frames rather than offsets into the ramp's
/// own life. That is what lets a graph rebuild at a loop boundary hand the
/// sweep to a freshly built filter and have it resume where its predecessor
/// was instead of restarting — the same reason [`Filter::on_transport`]
/// anchors LFO phase to the engine timeline.
///
/// [`Filter::on_transport`]: super::Filter::on_transport
#[derive(Debug, Clone)]
pub struct ParameterAutomation {
    pub node: NodeIndex<u32>,
    pub param: String,
    pub from: f32,
    pub to: f32,
    /// Absolute engine frame at which the ramp leaves `from`.
    pub start_frame: u64,
    /// Absolute engine frame at which the ramp has arrived at `to`.
    pub end_frame: u64,
    pub curve: RampCurve,
}

impl ParameterAutomation {
    /// The swept value at an absolute engine frame.
    ///
    /// A ramp holds `from` before it starts and `to` once it has arrived:
    /// ramps in the language arrive and then hold, they do not wrap or decay
    /// back. A zero-length ramp therefore reads as already arrived.
    pub fn value_at(&self, frame: u64) -> f32 {
        if frame >= self.end_frame {
            return self.to;
        }
        if frame <= self.start_frame {
            return self.from;
        }
        let progress =
            (frame - self.start_frame) as f32 / (self.end_frame - self.start_frame) as f32;
        match self.curve {
            // A non-positive endpoint has no geometric path between the two
            // values, and equal endpoints would raise 1.0 to a power for
            // nothing; both fall back to linear rather than yielding NaN.
            RampCurve::Exponential if self.from > 0.0 && self.to > 0.0 && self.from != self.to => {
                self.from * (self.to / self.from).powf(progress)
            }
            _ => self.from + (self.to - self.from) * progress,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ramp(from: f32, to: f32, curve: RampCurve) -> ParameterAutomation {
        ParameterAutomation {
            node: NodeIndex::new(0),
            param: "cutoff".into(),
            from,
            to,
            start_frame: 1_000,
            end_frame: 3_000,
            curve,
        }
    }

    #[test]
    fn a_linear_ramp_reaches_its_midpoint_at_the_middle_frame() {
        let automation = ramp(300.0, 9_000.0, RampCurve::Linear);
        assert!((automation.value_at(2_000) - 4_650.0).abs() < 1e-3);
    }

    #[test]
    fn an_exponential_ramp_midpoint_is_the_geometric_mean() {
        let automation = ramp(300.0, 9_000.0, RampCurve::Exponential);
        let geometric = (300.0f32 * 9_000.0).sqrt();
        assert!(
            (automation.value_at(2_000) - geometric).abs() < 1.0,
            "{} is not the geometric mean {geometric}",
            automation.value_at(2_000)
        );
        // The arithmetic midpoint is what a linear ramp would give, and the
        // two must not be confusable for this range.
        assert!((automation.value_at(2_000) - 4_650.0).abs() > 1_000.0);
    }

    #[test]
    fn ramps_hold_before_the_start_and_after_the_end() {
        for curve in [RampCurve::Linear, RampCurve::Exponential] {
            let automation = ramp(300.0, 9_000.0, curve);
            assert_eq!(automation.value_at(0), 300.0);
            assert_eq!(automation.value_at(1_000), 300.0);
            assert_eq!(automation.value_at(3_000), 9_000.0);
            assert_eq!(automation.value_at(u64::MAX), 9_000.0);
        }
    }

    #[test]
    fn degenerate_exponential_endpoints_fall_back_to_linear() {
        let through_zero = ramp(-1.0, 1.0, RampCurve::Exponential);
        assert!((through_zero.value_at(2_000) - 0.0).abs() < 1e-6);

        let to_zero = ramp(1.0, 0.0, RampCurve::Exponential);
        assert!((to_zero.value_at(2_000) - 0.5).abs() < 1e-6);

        let flat = ramp(440.0, 440.0, RampCurve::Exponential);
        assert_eq!(flat.value_at(2_000), 440.0);
    }

    #[test]
    fn a_zero_length_ramp_reads_as_arrived() {
        let mut automation = ramp(300.0, 9_000.0, RampCurve::Exponential);
        automation.end_frame = automation.start_frame;
        assert_eq!(automation.value_at(automation.start_frame), 9_000.0);
        assert_eq!(automation.value_at(0), 300.0);
    }
}
