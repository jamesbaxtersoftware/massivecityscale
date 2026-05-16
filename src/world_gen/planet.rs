use noise::{NoiseFn, Simplex};

pub const GRID_SIZE: i32 = 64;
pub const LAND_THRESHOLD: f64 = 0.0;

#[derive(Debug, Clone)]
pub struct ContinentData {
    pub cells: Vec<(i32, i32)>,
}

#[derive(Debug)]
pub struct PlanetData {
    pub continents: Vec<ContinentData>,
}

pub fn generate(seed: u64) -> PlanetData {
    let noise = Simplex::new(seed as u32);
    let mut land: Vec<Vec<bool>> = vec![vec![false; GRID_SIZE as usize]; GRID_SIZE as usize];

    for x in 0..GRID_SIZE {
        for z in 0..GRID_SIZE {
            let nx = x as f64 / GRID_SIZE as f64 * 4.0;
            let nz = z as f64 / GRID_SIZE as f64 * 4.0;
            land[x as usize][z as usize] = noise.get([nx, nz]) > LAND_THRESHOLD;
        }
    }

    let continents = flood_fill_continents(&land);
    PlanetData { continents }
}

fn flood_fill_continents(land: &Vec<Vec<bool>>) -> Vec<ContinentData> {
    let size = land.len();
    let mut visited = vec![vec![false; size]; size];
    let mut continents = Vec::new();

    for x in 0..size {
        for z in 0..size {
            if land[x][z] && !visited[x][z] {
                let cells = bfs(land, &mut visited, x, z, size);
                if cells.len() >= 4 {
                    continents.push(ContinentData { cells });
                }
            }
        }
    }
    continents
}

fn bfs(land: &Vec<Vec<bool>>, visited: &mut Vec<Vec<bool>>, sx: usize, sz: usize, size: usize) -> Vec<(i32, i32)> {
    let mut queue = std::collections::VecDeque::new();
    let mut cells = Vec::new();
    queue.push_back((sx, sz));
    visited[sx][sz] = true;

    while let Some((x, z)) = queue.pop_front() {
        cells.push((x as i32, z as i32));
        for (dx, dz) in [(-1i32, 0), (1, 0), (0, -1i32), (0, 1)] {
            let nx = x as i32 + dx;
            let nz = z as i32 + dz;
            if nx >= 0 && nx < size as i32 && nz >= 0 && nz < size as i32 {
                let (nx, nz) = (nx as usize, nz as usize);
                if land[nx][nz] && !visited[nx][nz] {
                    visited[nx][nz] = true;
                    queue.push_back((nx, nz));
                }
            }
        }
    }
    cells
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn planet_has_continents() {
        let planet = generate(42);
        assert!(!planet.continents.is_empty(), "seed 42 should produce at least one continent");
    }

    #[test]
    fn continents_have_minimum_size() {
        let planet = generate(42);
        for c in &planet.continents {
            assert!(c.cells.len() >= 4, "all continents must have at least 4 cells");
        }
    }

    #[test]
    fn planet_generation_is_deterministic() {
        let a = generate(42);
        let b = generate(42);
        assert_eq!(a.continents.len(), b.continents.len());
        assert_eq!(a.continents[0].cells.len(), b.continents[0].cells.len());
    }
}
