#[derive(Debug, Clone)]
pub struct BuildingData {
    pub x: f32,
    pub z: f32,
    pub width: f32,
    pub depth: f32,
    pub height: f32,
}

#[derive(Debug, Clone)]
pub struct RoadData {
    pub x: f32,
    pub z: f32,
    pub length: f32,
    pub horizontal: bool,
}

#[derive(Debug, Clone)]
pub struct BlockData {
    pub x: f32,
    pub z: f32,
    pub buildings: Vec<BuildingData>,
    pub roads: Vec<RoadData>,
}

#[derive(Debug, Clone)]
pub struct DistrictData {
    pub x: f32,
    pub z: f32,
    pub blocks: Vec<BlockData>,
}

#[derive(Debug, Clone)]
pub struct CityData {
    pub lat: f32,
    pub lon: f32,
    pub districts: Vec<DistrictData>,
}
