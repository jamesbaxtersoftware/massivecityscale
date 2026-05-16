#[derive(Debug, Clone)]
pub struct ContinentData {
    pub cells: Vec<(i32, i32)>,
}

#[derive(Debug)]
pub struct PlanetData {
    pub continents: Vec<ContinentData>,
}
