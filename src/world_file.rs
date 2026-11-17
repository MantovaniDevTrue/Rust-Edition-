use std::io::{self, Read};

const MOBILE_WORLD_VERSION: u32 = 326;
const SECTION_COUNT: usize = 11;

fn invalid(reason: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, reason)
}

fn bytes<const N: usize>(input: &mut &[u8]) -> io::Result<[u8; N]> {
    let mut result = [0; N];
    input.read_exact(&mut result)?;
    Ok(result)
}

fn byte(input: &mut &[u8]) -> io::Result<u8> {
    Ok(bytes::<1>(input)?[0])
}

fn text<'a>(input: &mut &'a [u8]) -> io::Result<&'a str> {
    let mut length = 0u32;
    for shift in (0..=28).step_by(7) {
        let next = byte(input)?;
        if shift == 28 && next > 7 {
            break;
        }
        length |= u32::from(next & 127) << shift;
        if next & 128 == 0 {
            let (string, remaining) = input
                .split_at_checked(length as usize)
                .ok_or_else(|| invalid("truncated world string"))?;
            *input = remaining;
            return std::str::from_utf8(string).map_err(|_| invalid("invalid world string"));
        }
    }
    Err(invalid("invalid world string length"))
}

pub struct WorldFile<'a> {
    pub name: &'a str,
    pub seed: &'a str,
    pub revision: u32,
    pub width: u32,
    pub height: u32,
    framed_tiles: &'a [u8],
    tile_count: usize,
    tile_data: &'a [u8],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tile {
    pub kind: Option<u16>,
    pub frame: Option<(i16, i16)>,
    pub paint: Option<u8>,
    pub wall: Option<u16>,
    pub wall_paint: Option<u8>,
    pub liquid: Option<(Liquid, u8)>,
    pub wires_and_shape: u8,
    pub actuator_flags: u8,
    pub coatings: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Liquid {
    Water,
    Lava,
    Honey,
    Shimmer,
}

impl<'a> WorldFile<'a> {
    pub fn read(mut input: &'a [u8]) -> io::Result<Self> {
        let file = input;
        if u32::from_le_bytes(bytes(&mut input)?) != MOBILE_WORLD_VERSION {
            return Err(invalid("expected Terraria world format 326"));
        }
        if &bytes::<8>(&mut input)? != b"relogic\x02" {
            return Err(invalid("expected a Terraria .wld file"));
        }
        let revision = u32::from_le_bytes(bytes(&mut input)?);
        let _flags: [u8; 8] = bytes(&mut input)?;
        if usize::from(u16::from_le_bytes(bytes(&mut input)?)) != SECTION_COUNT {
            return Err(invalid("unexpected number of world sections"));
        }
        let mut offsets = [0; SECTION_COUNT];
        for offset in &mut offsets {
            *offset = u32::from_le_bytes(bytes(&mut input)?) as usize;
        }
        if offsets.windows(2).any(|pair| pair[0] > pair[1])
            || offsets[SECTION_COUNT - 1] >= file.len()
        {
            return Err(invalid("invalid world section offsets"));
        }

        let tile_count = usize::from(u16::from_le_bytes(bytes(&mut input)?));
        let (framed_tiles, remaining) = input
            .split_at_checked(tile_count.div_ceil(8))
            .ok_or_else(|| invalid("truncated world tile frame flags"))?;
        if file.len() - remaining.len() != offsets[0] {
            return Err(invalid("tile frame flags do not end at the world header"));
        }
        let mut header = &file[offsets[0]..offsets[1]];
        let name = text(&mut header)?;
        let seed = text(&mut header)?;
        // World generation version, GUID, world ID and pixel bounds.
        let _header_fields: [u8; 44] = bytes(&mut header)?;
        let height = u32::from_le_bytes(bytes(&mut header)?);
        let width = u32::from_le_bytes(bytes(&mut header)?);
        if width == 0 || height == 0 {
            return Err(invalid("empty Terraria world"));
        }

        Ok(Self {
            name,
            seed,
            revision,
            width,
            height,
            framed_tiles,
            tile_count,
            tile_data: &file[offsets[1]..offsets[2]],
        })
    }

    pub fn visit_runs(&self, mut visit: impl FnMut(u32, u32, u32, Tile)) -> io::Result<()> {
        let mut data = self.tile_data;
        for x in 0..self.width {
            let mut y = 0;
            while y < self.height {
                let (tile, run) = self.read_tile(&mut data)?;
                if run > self.height - y {
                    return Err(invalid("tile run crosses the bottom of its column"));
                }
                visit(x, y, run, tile);
                y += run;
            }
        }
        if !data.is_empty() {
            return Err(invalid("tile decoder did not reach the next world section"));
        }
        Ok(())
    }

    fn read_tile(&self, input: &mut &[u8]) -> io::Result<(Tile, u32)> {
        let first = byte(input)?;
        let second = chained_flags(input, first)?;
        let third = chained_flags(input, second)?;
        let fourth = chained_flags(input, third)?;

        let mut kind = None;
        let mut frame = None;
        let mut paint = None;
        if first & 0x02 != 0 {
            let id = u16::from_le_bytes([
                byte(input)?,
                if first & 0x20 != 0 { byte(input)? } else { 0 },
            ]);
            let index = usize::from(id);
            if index >= self.tile_count {
                return Err(invalid("tile ID is outside the world frame table"));
            }
            if self.framed_tiles[index / 8] & (1 << (index % 8)) != 0 {
                frame = Some((
                    i16::from_le_bytes(bytes(input)?),
                    i16::from_le_bytes(bytes(input)?),
                ));
            }
            if third & 0x08 != 0 {
                paint = Some(byte(input)?);
            }
            kind = Some(id);
        }

        let wall_low = (first & 0x04 != 0).then(|| byte(input)).transpose()?;
        let wall_paint = (wall_low.is_some() && third & 0x10 != 0)
            .then(|| byte(input))
            .transpose()?;
        let liquid_type = match first & 0x18 {
            0x08 if third & 0x80 != 0 => Some(Liquid::Shimmer),
            0x08 => Some(Liquid::Water),
            0x10 => Some(Liquid::Lava),
            0x18 => Some(Liquid::Honey),
            _ => None,
        };
        let liquid = liquid_type.map(|kind| byte(input).map(|amount| (kind, amount))).transpose()?;
        let wall_high = if third & 0x40 != 0 { byte(input)? } else { 0 };
        let wall = wall_low.map(|low| u16::from_le_bytes([low, wall_high]));
        let repeats = match first >> 6 {
            0 => 0,
            1 => u32::from(byte(input)?),
            2 => u32::from(u16::from_le_bytes(bytes(input)?)),
            _ => return Err(invalid("invalid tile repeat encoding")),
        };

        Ok((Tile {
            kind,
            frame,
            paint,
            wall,
            wall_paint,
            liquid,
            wires_and_shape: second & 0x7e,
            actuator_flags: third & 0x26,
            coatings: fourth & 0x1e,
        }, repeats + 1))
    }
}

fn chained_flags(input: &mut &[u8], previous: u8) -> io::Result<u8> {
    if previous & 1 != 0 { byte(input) } else { Ok(0) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world(tile_data: &[u8]) -> WorldFile<'_> {
        WorldFile {
            name: "test", seed: "", revision: 0, width: 1, height: 4,
            framed_tiles: &[1], tile_count: 1, tile_data,
        }
    }

    #[test]
    fn framed_tile_and_byte_run_fill_one_column() {
        let data = [0x42, 0, 18, 0, 36, 0, 3];
        let mut count = 0;
        world(&data).visit_runs(|x, y, run, tile| {
            assert_eq!((x, y, run), (0, 0, 4));
            assert_eq!(tile.kind, Some(0));
            assert_eq!(tile.frame, Some((18, 36)));
            count += 1;
        }).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn rejects_runs_spilling_into_next_section_or_column() {
        for data in [&[0x40, 4][..], &[0x40, 3, 0], &[0x40]] {
            assert!(world(data).visit_runs(|_, _, _, _| {}).is_err());
        }
    }
}
