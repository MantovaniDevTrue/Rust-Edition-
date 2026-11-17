pub mod world_file;

#[cfg(unix)]
mod world_preview;

use std::{fs, io, path::Path};

const WIDTH: usize = 160;
const HEIGHT: usize = 72;
const TILES: usize = WIDTH * HEIGHT;
const MAGIC: &[u8; 4] = b"RE01";
const AIR: u8 = 0;
const GRASS: u8 = 1;
const DIRT: u8 = 2;
const STONE: u8 = 3;
const PLAYER_WIDTH: f32 = 0.75;
const PLAYER_HEIGHT: f32 = 1.65;
const REACH: f32 = 5.0;

struct World {
    tiles: Vec<u8>,
    x: f32,
    y: f32,
    velocity_y: f32,
    grounded: bool,
}

impl World {
    fn new() -> Self {
        let mut tiles = vec![AIR; TILES];
        for x in 0..WIDTH {
            let surface = (27_i32 + (3.0 * (x as f32 * 0.13).sin()).round() as i32) as usize;
            for y in surface..HEIGHT {
                tiles[y * WIDTH + x] = match y - surface {
                    0 => GRASS,
                    1..=5 => DIRT,
                    _ => STONE,
                };
            }
        }
        Self {
            tiles,
            x: WIDTH as f32 * 0.5,
            y: 22.0,
            velocity_y: 0.0,
            grounded: false,
        }
    }

    fn blocked(&self, x: f32, y: f32) -> bool {
        let right = (x + PLAYER_WIDTH - 0.001).floor() as i32;
        let bottom = (y + PLAYER_HEIGHT - 0.001).floor() as i32;
        for row in y.floor() as i32..=bottom {
            for col in x.floor() as i32..=right {
                if col < 0 || row < 0 || col >= WIDTH as i32 || row >= HEIGHT as i32 {
                    return true;
                }
                if self.tiles[row as usize * WIDTH + col as usize] != AIR {
                    return true;
                }
            }
        }
        false
    }

    fn step(&mut self, axis: i32, jump: bool) {
        let dx = axis.clamp(-1, 1) as f32 * 0.19;
        if !self.blocked(self.x + dx, self.y) {
            self.x += dx;
        }
        if jump && self.grounded {
            self.velocity_y = -0.40;
        }
        self.velocity_y = (self.velocity_y + 0.023).min(0.65);
        let next_y = self.y + self.velocity_y;
        if self.blocked(self.x, next_y) {
            let floor = (next_y + PLAYER_HEIGHT - 0.001).floor();
            let ceiling = next_y.floor() + 1.0;
            let boundary = if self.velocity_y > 0.0 {
                floor - PLAYER_HEIGHT
            } else {
                ceiling
            };
            if !self.blocked(self.x, boundary) {
                self.y = boundary;
            }
            self.grounded = self.velocity_y > 0.0;
            self.velocity_y = 0.0;
        } else {
            self.y = next_y;
            self.grounded = false;
        }
    }

    fn edit(&mut self, x: i32, y: i32, place: bool) -> bool {
        if !(0..WIDTH as i32).contains(&x) || !(0..HEIGHT as i32 - 1).contains(&y) {
            return false;
        }
        let dx = x as f32 + 0.5 - (self.x + PLAYER_WIDTH * 0.5);
        let dy = y as f32 + 0.5 - (self.y + PLAYER_HEIGHT * 0.5);
        if dx * dx + dy * dy > REACH * REACH {
            return false;
        }
        let index = y as usize * WIDTH + x as usize;
        if !place && self.tiles[index] != AIR {
            self.tiles[index] = AIR;
            return true;
        }
        if place && self.tiles[index] == AIR {
            self.tiles[index] = DIRT;
            if self.blocked(self.x, self.y) {
                self.tiles[index] = AIR;
                return false;
            }
            return true;
        }
        false
    }

    fn encode(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(12 + TILES);
        bytes.extend(MAGIC);
        bytes.extend(self.x.to_le_bytes());
        bytes.extend(self.y.to_le_bytes());
        bytes.extend(&self.tiles);
        bytes
    }

    fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != 12 + TILES || !bytes.starts_with(MAGIC) {
            return None;
        }
        let x = f32::from_le_bytes(bytes[4..8].try_into().ok()?);
        let y = f32::from_le_bytes(bytes[8..12].try_into().ok()?);
        if !x.is_finite() || !y.is_finite()
            || !(0.0..=WIDTH as f32 - PLAYER_WIDTH).contains(&x)
            || !(0.0..=HEIGHT as f32 - PLAYER_HEIGHT).contains(&y)
            || bytes[12..].iter().any(|tile| *tile > STONE)
        {
            return None;
        }
        let world = Self { tiles: bytes[12..].to_vec(), x, y, velocity_y: 0.0, grounded: false };
        (!world.blocked(x, y)).then_some(world)
    }

    fn save(&self, path: &Path) -> io::Result<()> {
        let pending = path.with_extension("tmp");
        fs::write(&pending, self.encode())?;
        fs::rename(pending, path)
    }
}

#[cfg(unix)]
mod android;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gravity_lands_and_jump_lifts_player() {
        let mut world = World::new();
        for _ in 0..120 {
            world.step(0, false);
        }
        assert!(world.grounded);
        let floor = world.y;
        world.step(0, true);
        assert!(world.y < floor);
        assert!(!world.blocked(world.x, world.y));
    }

    #[test]
    fn edits_respect_reach_and_save_round_trips() {
        let mut world = World::new();
        for _ in 0..120 {
            world.step(0, false);
        }
        let x = world.x as i32;
        let floor = (world.y + PLAYER_HEIGHT + 0.001) as i32;
        assert!(!world.edit(0, 0, true));
        assert!(!world.edit(x, world.y.ceil() as i32, true));
        assert!(world.edit(x, floor, false));
        assert!(world.edit(x, floor, true));
        let encoded = world.encode();
        assert_eq!(World::decode(&encoded).unwrap().encode(), encoded);
        assert!(World::decode(&encoded[..12]).is_none());
    }
}
