pub mod planet;
pub mod city;
pub mod street;

pub const WORLD_SEED: u64 = 42;

#[derive(Debug)]
pub struct WorldData {
    pub planet: planet::PlanetData,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_data_holds_planet() {
        let planet = planet::PlanetData { continents: vec![] };
        let world = WorldData { planet };
        assert_eq!(world.planet.continents.len(), 0);
    }
}
