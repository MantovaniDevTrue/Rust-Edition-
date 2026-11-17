use rust_edition::world_file::WorldFile;
use std::{collections::BTreeSet, env, fs, io};

fn main() -> io::Result<()> {
    let path = env::args_os().nth(1).ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "usage: inspect_world <file.wld>")
    })?;
    let data = fs::read(path)?;
    let world = WorldFile::read(&data)?;
    let mut tile_ids = BTreeSet::new();
    let mut cells = 0u64;
    let mut air = 0u64;
    world.visit_runs(|_, _, count, tile| {
        cells += u64::from(count);
        if let Some(id) = tile.kind {
            tile_ids.insert(id);
        } else {
            air += u64::from(count);
        }
    })?;
    println!("{} ({} × {}, revision {})", world.name, world.width, world.height, world.revision);
    println!("{cells} cells, {air} without tiles, {} tile IDs", tile_ids.len());
    Ok(())
}
