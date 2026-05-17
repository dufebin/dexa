use rand::Rng;

#[derive(Clone, Copy)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

pub struct Waypoint {
    pub x: i32,
    pub y: i32,
    pub delay_ms: u64,
}

pub fn generate_path(
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    steps: usize,
    total_ms: u64,
    rng: &mut impl Rng,
) -> Vec<Waypoint> {
    let steps = steps.max(2);
    let p0 = Point {
        x: x0 as f64,
        y: y0 as f64,
    };
    let p3 = Point {
        x: x1 as f64,
        y: y1 as f64,
    };
    let (cp1, cp2) = random_control_points(x0, y0, x1, y1, rng);

    // Compute per-step timestamps using ease-in-out timing.
    // Earlier and later steps get more time (slower), middle steps less (faster).
    let timestamps: Vec<f64> = (0..steps)
        .map(|i| ease_in_out(i as f64 / (steps - 1) as f64) * total_ms as f64)
        .collect();

    let delays: Vec<u64> = timestamps
        .windows(2)
        .map(|w| (w[1] - w[0]).ceil() as u64)
        .collect();

    (0..steps)
        .map(|i| {
            let t = i as f64 / (steps - 1) as f64;
            let pt = cubic_bezier(p0, cp1, cp2, p3, t);
            let delay = if i + 1 < steps { delays[i] } else { 0 };
            Waypoint {
                x: pt.x.round() as i32,
                y: pt.y.round() as i32,
                delay_ms: delay,
            }
        })
        .collect()
}

fn cubic_bezier(p0: Point, p1: Point, p2: Point, p3: Point, t: f64) -> Point {
    let u = 1.0 - t;
    Point {
        x: u * u * u * p0.x + 3.0 * u * u * t * p1.x + 3.0 * u * t * t * p2.x + t * t * t * p3.x,
        y: u * u * u * p0.y + 3.0 * u * u * t * p1.y + 3.0 * u * t * t * p2.y + t * t * t * p3.y,
    }
}

fn ease_in_out(t: f64) -> f64 {
    if t < 0.5 {
        2.0 * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
    }
}

fn random_control_points(x0: i32, y0: i32, x1: i32, y1: i32, rng: &mut impl Rng) -> (Point, Point) {
    let dx = (x1 - x0) as f64;
    let dy = (y1 - y0) as f64;
    let len = (dx * dx + dy * dy).sqrt().max(1.0);

    let perp_x = -dy / len;
    let perp_y = dx / len;

    let sign1 = if rng.gen_bool(0.5) { 1.0_f64 } else { -1.0_f64 };
    let sign2 = if rng.gen_bool(0.5) { 1.0_f64 } else { -1.0_f64 };
    let offset1 = len * rng.gen_range(0.05_f64..0.20_f64) * sign1;
    let offset2 = len * rng.gen_range(0.05_f64..0.20_f64) * sign2;

    let cp1 = Point {
        x: x0 as f64 + dx * 0.33 + perp_x * offset1,
        y: y0 as f64 + dy * 0.33 + perp_y * offset1,
    };
    let cp2 = Point {
        x: x0 as f64 + dx * 0.67 + perp_x * offset2,
        y: y0 as f64 + dy * 0.67 + perp_y * offset2,
    };

    (cp1, cp2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{rngs::SmallRng, SeedableRng};

    #[test]
    fn generated_path_keeps_requested_size_and_exact_endpoints() {
        let mut rng = SmallRng::seed_from_u64(7);
        let path = generate_path(10, 20, 110, 220, 8, 320, &mut rng);

        assert_eq!(path.len(), 8);
        assert_eq!((path[0].x, path[0].y), (10, 20));
        assert_eq!(path.last().map(|p| (p.x, p.y)), Some((110, 220)));
        assert_eq!(path.last().map(|p| p.delay_ms), Some(0));
    }

    #[test]
    fn generated_path_clamps_to_at_least_two_steps() {
        let mut rng = SmallRng::seed_from_u64(11);
        let path = generate_path(0, 0, 1, 1, 0, 10, &mut rng);

        assert_eq!(path.len(), 2);
    }
}
