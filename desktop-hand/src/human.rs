use rand::Rng;

pub fn char_delay_ms(rng: &mut impl Rng) -> u64 {
    rng.gen_range(30_u64..=150)
}

pub fn word_pause_ms(rng: &mut impl Rng) -> u64 {
    rng.gen_range(100_u64..=400)
}

pub fn sentence_pause_ms(rng: &mut impl Rng) -> u64 {
    rng.gen_range(300_u64..=800)
}

/// Random ±1–3 px jitter applied to each mouse waypoint.
pub fn jitter(rng: &mut impl Rng) -> (i32, i32) {
    (rng.gen_range(-3_i32..=3), rng.gen_range(-3_i32..=3))
}

/// ~2% chance of a typo (random char + backspace).
pub fn should_typo(rng: &mut impl Rng) -> bool {
    rng.gen_bool(0.02)
}

/// Random noise char to simulate a typo.
pub fn typo_char(rng: &mut impl Rng) -> char {
    let chars = b"abcdefghijklmnopqrstuvwxyz";
    chars[rng.gen_range(0..chars.len())] as char
}

/// Number of bezier path steps based on movement duration.
pub fn mouse_steps(total_ms: u64) -> usize {
    ((total_ms / 12).clamp(4, 100)) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mouse_steps_allow_short_fast_human_moves() {
        assert_eq!(mouse_steps(0), 4);
        assert_eq!(mouse_steps(20), 4);
        assert_eq!(mouse_steps(300), 25);
        assert_eq!(mouse_steps(2_000), 100);
    }
}
