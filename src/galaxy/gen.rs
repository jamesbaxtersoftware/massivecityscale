use bevy::math::DVec3;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

/// Number of distant backdrop stars (LOD point tier). Slice budget, not galaxy scale.
pub const BACKDROP_STARS: usize = 500;
/// Backdrop stars scatter in a cube of this half-extent (metres) — a far backdrop
/// (2 million km) so they read as fixed points.
pub const BACKDROP_HALF_EXTENT: f64 = 2.0e9;
/// Number of nearby star systems the player can approach.
pub const SYSTEMS: usize = 3;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum PlanetType {
    Rock,
    Fire,
    Water,
}

#[derive(Clone, Copy)]
pub struct Star {
    pub pos: DVec3,
}

#[derive(Clone, Copy)]
pub struct Planet {
    pub pos: DVec3,
    pub radius: f64,
    pub kind: PlanetType,
}

pub struct GalaxyData {
    pub stars: Vec<Star>,
    pub planets: Vec<Planet>,
}

fn planet_type(rng: &mut ChaCha8Rng) -> PlanetType {
    match rng.gen_range(0..3) {
        0 => PlanetType::Rock,
        1 => PlanetType::Fire,
        _ => PlanetType::Water,
    }
}

/// Build the entire slice galaxy deterministically from `seed`.
pub fn generate(seed: u64) -> GalaxyData {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);

    let mut stars = Vec::with_capacity(BACKDROP_STARS);
    for _ in 0..BACKDROP_STARS {
        let e = BACKDROP_HALF_EXTENT;
        stars.push(Star {
            pos: DVec3::new(
                rng.gen_range(-e..e),
                rng.gen_range(-e..e),
                rng.gen_range(-e..e),
            ),
        });
    }

    let mut planets = Vec::new();
    for s in 0..SYSTEMS {
        // Distances in metres. First system sits ~40,000 km dead ahead so there's
        // something to fly toward immediately; later systems scatter further out.
        let center = if s == 0 {
            DVec3::new(
                rng.gen_range(-4.0e6..4.0e6),
                rng.gen_range(-2.0e6..2.0e6),
                -1.8e7,
            )
        } else {
            DVec3::new(
                rng.gen_range(-2.0e7..2.0e7),
                rng.gen_range(-8.0e6..8.0e6),
                -1.8e7 - s as f64 * 3.0e7,
            )
        };
        let n = rng.gen_range(1..=3);
        for _p in 0..n {
            let offset = DVec3::new(
                rng.gen_range(-1.5e7..1.5e7),
                rng.gen_range(-4.0e6..4.0e6),
                rng.gen_range(-1.5e7..1.5e7),
            );
            planets.push(Planet {
                // Planet radius 2,000–6,000 km (vs a ~30 m ship).
                pos: center + offset,
                radius: rng.gen_range(2.0e6..6.0e6),
                kind: planet_type(&mut rng),
            });
        }
    }

    GalaxyData { stars, planets }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_produces_identical_galaxy() {
        let a = generate(42);
        let b = generate(42);
        assert_eq!(a.stars.len(), b.stars.len());
        assert_eq!(a.planets.len(), b.planets.len());
        for (x, y) in a.stars.iter().zip(b.stars.iter()) {
            assert_eq!(x.pos, y.pos);
        }
        for (x, y) in a.planets.iter().zip(b.planets.iter()) {
            assert_eq!(x.pos, y.pos);
            assert_eq!(x.radius, y.radius);
            assert_eq!(x.kind, y.kind);
        }
    }

    #[test]
    fn different_seeds_differ() {
        let a = generate(1);
        let b = generate(2);
        assert!(a.stars[0].pos != b.stars[0].pos, "seed changes the starfield");
    }

    #[test]
    fn scale_sanity_planets_are_world_sized_vs_ship() {
        // Ship is ~30 m; planets must be thousands of km (millions of m), i.e.
        // at least ~10,000x the ship — fixing the old inverted ratio.
        const SHIP_LEN_M: f64 = 30.0;
        for p in generate(42).planets {
            assert!(
                (2.0e6..=6.0e6).contains(&p.radius),
                "planet radius {} m out of intended 2,000-6,000 km band",
                p.radius
            );
            assert!(p.radius / SHIP_LEN_M > 10_000.0, "planet dwarfs the ship");
        }
    }

    #[test]
    fn has_backdrop_stars_and_at_least_one_planet() {
        let g = generate(7);
        assert_eq!(g.stars.len(), BACKDROP_STARS);
        assert!(!g.planets.is_empty());
    }

}
