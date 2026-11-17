use crate::world_file::{Liquid, Tile, WorldFile};
use std::io;

const MAX_EDGE: u32 = 2048;

pub(crate) fn build(data: &[u8]) -> io::Result<Vec<i32>> {
    let world = WorldFile::read(data)?;
    let stride = world.width.max(world.height).div_ceil(MAX_EDGE);
    let width = world.width.div_ceil(stride);
    let height = world.height.div_ceil(stride);
    let mut result = vec![0; 2 + (width * height) as usize];
    result[..2].copy_from_slice(&[width as i32, height as i32]);
    world.visit_runs(|x, y, length, tile| {
        if x % stride != 0 {
            return;
        }
        let start = y.div_ceil(stride);
        let end = (y + length).div_ceil(stride);
        if start < end {
            let pixel = color(tile) as i32;
            let column = (x / stride) as usize;
            for row in start..end {
                result[2 + row as usize * width as usize + column] = pixel;
            }
        }
    })?;
    Ok(result)
}

fn color(tile: Tile) -> u32 {
    if let Some(kind) = tile.kind {
        return match kind {
            0 => 0xff895936,
            1 => 0xff777781,
            2 | 23 | 60 | 70 | 109 | 199 => 0xff5d9343,
            53 | 112 | 116 | 234 | 397 => 0xffc7ac6f,
            147 | 161 | 162 | 163 | 164 | 200 => 0xffaacbde,
            _ => 0xff776364,
        };
    }
    if let Some((liquid, _)) = tile.liquid {
        return match liquid {
            Liquid::Water => 0xff315eac,
            Liquid::Lava => 0xffee5a26,
            Liquid::Honey => 0xffcf8b2b,
            Liquid::Shimmer => 0xffb27fc9,
        };
    }
    if tile.wall.is_some() { 0xff3f3034 } else { 0xff1c2538 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_world_file_and_renders_columns_in_bitmap_order() {
        let mut world = Vec::new();
        world.extend(326u32.to_le_bytes());
        world.extend(b"relogic\x02");
        world.extend([0; 12]);
        world.extend(11u16.to_le_bytes());
        for offset in [73u32, 127, 133, 133, 133, 133, 133, 133, 133, 133, 133] {
            world.extend(offset.to_le_bytes());
        }
        world.extend(2u16.to_le_bytes());
        world.push(0);
        world.extend([0, 0]);
        world.extend([0; 44]);
        world.extend(2u32.to_le_bytes());
        world.extend(2u32.to_le_bytes());
        world.extend([0, 2, 0, 0x42, 1, 1]);
        world.push(1);

        let pixels = build(&world).unwrap();
        assert_eq!(&pixels[..2], &[2, 2]);
        assert_eq!(
            &pixels[2..],
            &[0xff1c2538u32 as i32, 0xff777781u32 as i32,
              0xff895936u32 as i32, 0xff777781u32 as i32],
        );
    }
}
