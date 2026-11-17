use std::{
    env,
    fs::File,
    io::{self, Cursor, Read, Seek, SeekFrom},
};

const MAX_INDEX_BYTES: u32 = 16 * 1024 * 1024;

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn read_array<const N: usize>(reader: &mut impl Read) -> io::Result<[u8; N]> {
    let mut bytes = [0; N];
    reader.read_exact(&mut bytes)?;
    Ok(bytes)
}

fn read_string(reader: &mut impl Read) -> io::Result<String> {
    let mut bytes = Vec::new();
    for _ in 0..4096 {
        let [byte] = read_array(reader)?;
        if byte == 0 {
            return String::from_utf8(bytes).map_err(|_| invalid("invalid UnityFS string"));
        }
        bytes.push(byte);
    }
    Err(invalid("UnityFS string exceeds the supported length"))
}

fn lz4_length(input: &mut &[u8], base: usize) -> io::Result<usize> {
    let mut length = base;
    if base == 15 {
        loop {
            let [extra] = read_array(input)?;
            length += usize::from(extra);
            if extra != 255 {
                break;
            }
        }
    }
    Ok(length)
}

fn decode_lz4(mut input: &[u8], expected: usize) -> io::Result<Vec<u8>> {
    let mut output = Vec::with_capacity(expected);
    while !input.is_empty() {
        let [token] = read_array(&mut input)?;
        let literals = lz4_length(&mut input, usize::from(token >> 4))?;
        if literals > input.len() || literals > expected - output.len() {
            return Err(invalid("invalid LZ4 literal length"));
        }
        output.extend_from_slice(&input[..literals]);
        input = &input[literals..];
        if input.is_empty() {
            break;
        }
        let offset = usize::from(u16::from_le_bytes(read_array(&mut input)?));
        let length = lz4_length(&mut input, usize::from(token & 15))? + 4;
        if offset == 0 || offset > output.len() || length > expected - output.len() {
            return Err(invalid("invalid LZ4 match"));
        }
        for _ in 0..length {
            output.push(output[output.len() - offset]);
        }
    }
    if output.len() != expected {
        return Err(invalid("UnityFS index size mismatch"));
    }
    Ok(output)
}

#[derive(Debug)]
struct Entry {
    path: String,
    offset: u64,
    size: u64,
}

fn read_index(reader: &mut (impl Read + Seek)) -> io::Result<(String, Vec<Entry>)> {
    let length = reader.seek(SeekFrom::End(0))?;
    reader.rewind()?;
    if read_string(reader)? != "UnityFS" {
        return Err(invalid("expected a UnityFS bundle"));
    }
    let version = u32::from_be_bytes(read_array(reader)?);
    if version != 8 {
        return Err(invalid("only UnityFS format 8 is supported"));
    }
    read_string(reader)?;
    let revision = read_string(reader)?;
    let file_size = u64::from_be_bytes(read_array(reader)?);
    let compressed = u32::from_be_bytes(read_array(reader)?);
    let uncompressed = u32::from_be_bytes(read_array(reader)?);
    let flags = u32::from_be_bytes(read_array(reader)?);
    if file_size != length || compressed > MAX_INDEX_BYTES || uncompressed > MAX_INDEX_BYTES {
        return Err(invalid("invalid UnityFS bundle or index size"));
    }

    let aligned_header = (reader.stream_position()? + 15) & !15;
    let trailing_index = file_size
        .checked_sub(u64::from(compressed))
        .filter(|offset| *offset >= aligned_header)
        .ok_or_else(|| invalid("UnityFS index overlaps its header"))?;
    let index_offset = if flags & 0x80 == 0 { aligned_header } else { trailing_index };
    reader.seek(SeekFrom::Start(index_offset))?;
    let mut payload = vec![0; compressed as usize];
    reader.read_exact(&mut payload)?;
    let decoded = match flags & 0x3f {
        0 if compressed == uncompressed => payload,
        2 | 3 => decode_lz4(&payload, uncompressed as usize)?,
        _ => return Err(invalid("unsupported UnityFS index compression")),
    };

    let mut table = Cursor::new(&decoded[..]);
    let _: [u8; 16] = read_array(&mut table)?;
    let blocks = u32::from_be_bytes(read_array(&mut table)?);
    let mut data_size = 0u64;
    for _ in 0..blocks {
        data_size += u64::from(u32::from_be_bytes(read_array(&mut table)?));
        let _: [u8; 6] = read_array(&mut table)?;
    }
    let entry_count = u32::from_be_bytes(read_array(&mut table)?);
    let mut entries = Vec::new();
    for _ in 0..entry_count {
        let offset = u64::from_be_bytes(read_array(&mut table)?);
        let size = u64::from_be_bytes(read_array(&mut table)?);
        let _: [u8; 4] = read_array(&mut table)?;
        let path = read_string(&mut table)?;
        if offset > data_size || size > data_size - offset {
            return Err(invalid("UnityFS entry exceeds the data blocks"));
        }
        entries.push(Entry { path, offset, size });
    }
    Ok((revision, entries))
}

fn main() -> io::Result<()> {
    let path = env::args_os().nth(1).ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "usage: unityfs_index <data.unity3d>")
    })?;
    let (revision, entries) = read_index(&mut File::open(path)?)?;
    println!("Unity {revision}");
    for entry in entries {
        println!("{:>12} {:>12} {}", entry.offset, entry.size, entry.path);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_overlapping_lz4_matches() {
        assert_eq!(decode_lz4(&[0x11, b'a', 1, 0], 6).unwrap(), b"aaaaaa");
        assert!(decode_lz4(&[0x10, b'a', 0, 0], 5).is_err());
    }
}
