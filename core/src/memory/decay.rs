use super::Knot;
use crate::config::MemoryConfig;

const SECONDS_PER_DAY: f64 = 86_400.0;

/// Effective clarity after 忘川 decay (does not mutate the knot).
///
/// Author: gz
pub fn effective_clarity(knot: &Knot, config: &MemoryConfig, now: i64) -> f32 {
    if knot.permanent || !config.decay_enabled {
        return knot.clarity;
    }
    let last = knot.last_accessed_at.unwrap_or(knot.updated_at);
    decay_clarity(knot.clarity, last, now, config.decay_half_life_days)
}

/// Author: gz
pub fn decay_clarity(clarity: f32, last_touch: i64, now: i64, half_life_days: f32) -> f32 {
    if half_life_days <= 0.0 {
        return clarity;
    }
    let elapsed_days = (now.saturating_sub(last_touch)) as f64 / SECONDS_PER_DAY;
    let factor = 0.5_f64.powf(elapsed_days / f64::from(half_life_days));
    (f64::from(clarity) * factor).clamp(0.0, 1.0) as f32
}

#[cfg(test)]
#[path = "../../test/unit/memory/decay.rs"]
mod tests;
