//! The DXBC container: a header, then a table of FourCC-tagged parts.
//!
//! ```text
//! 0x00  "DXBC"
//! 0x04  digest[16]
//! 0x14  version major u16, minor u16
//! 0x18  total size u32
//! 0x1C  part count u32
//! 0x20  part offsets u32[count]
//! part: fourcc[4], size u32, data[size]
//! ```

use crate::ShaderError;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct FourCC(pub [u8; 4]);

impl FourCC {
    pub const DXIL: FourCC = FourCC(*b"DXIL");
    pub const STAT: FourCC = FourCC(*b"STAT");
    pub const RTS0: FourCC = FourCC(*b"RTS0");
    pub const PSV0: FourCC = FourCC(*b"PSV0");
    pub const ISG1: FourCC = FourCC(*b"ISG1");
    pub const OSG1: FourCC = FourCC(*b"OSG1");
    /// Per-primitive outputs of a mesh shader.
    pub const PSG1: FourCC = FourCC(*b"PSG1");
    pub const SFI0: FourCC = FourCC(*b"SFI0");
    pub const HASH: FourCC = FourCC(*b"HASH");
    pub const ILDB: FourCC = FourCC(*b"ILDB");
    pub const ILDN: FourCC = FourCC(*b"ILDN");
}

impl std::fmt::Debug for FourCC {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl std::fmt::Display for FourCC {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for &b in &self.0 {
            let c = if b.is_ascii_graphic() { b as char } else { '.' };
            write!(f, "{c}")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Part<'a> {
    pub fourcc: FourCC,
    /// Offset of the part header within the container.
    pub offset: usize,
    pub data: &'a [u8],
}

#[derive(Debug, Clone)]
pub struct Container<'a> {
    pub digest: [u8; 16],
    pub version: (u16, u16),
    pub parts: Vec<Part<'a>>,
}

const HEADER_SIZE: usize = 0x20;
const MAX_PARTS: usize = 256;

impl<'a> Container<'a> {
    pub fn parse(data: &'a [u8]) -> Result<Self, ShaderError> {
        if data.len() < HEADER_SIZE || &data[0..4] != b"DXBC" {
            return Err(ShaderError::NotDxbc);
        }
        let digest = data[4..20].try_into().expect("16 bytes");
        let version = (u16_at(data, 0x14)?, u16_at(data, 0x16)?);
        let total = u32_at(data, 0x18)? as usize;
        if total > data.len() || total < HEADER_SIZE {
            return Err(ShaderError::Malformed("container size disagrees with data"));
        }
        let data = &data[..total];
        let count = u32_at(data, 0x1C)? as usize;
        if count > MAX_PARTS {
            return Err(ShaderError::Malformed("implausible part count"));
        }
        let mut parts = Vec::with_capacity(count);
        for i in 0..count {
            let offset = u32_at(data, HEADER_SIZE + 4 * i)? as usize;
            let fourcc = FourCC(
                data.get(offset..offset + 4)
                    .ok_or(ShaderError::Malformed("part header out of bounds"))?
                    .try_into()
                    .expect("4 bytes"),
            );
            let size = u32_at(data, offset + 4)? as usize;
            let body = data
                .get(offset + 8..offset + 8 + size)
                .ok_or(ShaderError::Malformed("part data out of bounds"))?;
            parts.push(Part {
                fourcc,
                offset,
                data: body,
            });
        }
        Ok(Container {
            digest,
            version,
            parts,
        })
    }

    pub fn part(&self, fourcc: FourCC) -> Option<&Part<'a>> {
        self.parts.iter().find(|p| p.fourcc == fourcc)
    }
}

/// Every container stored as is in `data`, found by its magic, as its offset
/// and its bytes (`total size` long), in file order. A `DXBC` inside one of
/// a found container's parts (a container kept in a part, say) is part of
/// that container, not another one. Its header is not such a place: a magic
/// in bytes before a real container may read its header from the real one,
/// and must not hide it. Offsets for which `skip` is true are not looked at
/// (bytes that belong to something else, such as a packed record).
pub fn find_all(data: &[u8], skip: impl Fn(usize) -> bool) -> Vec<(usize, &[u8])> {
    let mut out = Vec::new();
    // Parts of the containers found so far that may still hold a magic.
    let mut parts: Vec<std::ops::Range<usize>> = Vec::new();
    for offset in memchr::memmem::find_iter(data, b"DXBC") {
        parts.retain(|p| p.end > offset);
        if parts.iter().any(|p| p.contains(&offset)) || skip(offset) {
            continue;
        }
        let Ok(container) = Container::parse(&data[offset..]) else {
            continue;
        };
        // `parse` checked the size against the data.
        let total = u32_at(data, offset + 0x18).expect("parsed header") as usize;
        parts.extend(container.parts.iter().map(|p| {
            let start = offset + p.offset + 8;
            start..start + p.data.len()
        }));
        out.push((offset, &data[offset..offset + total]));
    }
    out
}

pub(crate) fn u16_at(data: &[u8], at: usize) -> Result<u16, ShaderError> {
    data.get(at..at + 2)
        .map(|b| u16::from_le_bytes(b.try_into().expect("2 bytes")))
        .ok_or(ShaderError::Malformed("truncated"))
}

pub(crate) fn u32_at(data: &[u8], at: usize) -> Result<u32, ShaderError> {
    data.get(at..at + 4)
        .map(|b| u32::from_le_bytes(b.try_into().expect("4 bytes")))
        .ok_or(ShaderError::Malformed("truncated"))
}

pub(crate) fn u64_at(data: &[u8], at: usize) -> Result<u64, ShaderError> {
    data.get(at..at + 8)
        .map(|b| u64::from_le_bytes(b.try_into().expect("8 bytes")))
        .ok_or(ShaderError::Malformed("truncated"))
}

/// Builds a container from parts (test helper).
#[cfg(test)]
pub fn build(parts: &[(FourCC, &[u8])]) -> Vec<u8> {
    let table = HEADER_SIZE + 4 * parts.len();
    let total = table + parts.iter().map(|(_, d)| 8 + d.len()).sum::<usize>();
    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(b"DXBC");
    out.extend_from_slice(&[0; 16]);
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&(total as u32).to_le_bytes());
    out.extend_from_slice(&(parts.len() as u32).to_le_bytes());
    let mut offset = table;
    for (_, d) in parts {
        out.extend_from_slice(&(offset as u32).to_le_bytes());
        offset += 8 + d.len();
    }
    for (fourcc, d) in parts {
        out.extend_from_slice(&fourcc.0);
        out.extend_from_slice(&(d.len() as u32).to_le_bytes());
        out.extend_from_slice(d);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_parts() {
        let bytes = build(&[(FourCC::SFI0, &[1, 2, 3, 4]), (FourCC::HASH, &[9; 20])]);
        let c = Container::parse(&bytes).unwrap();
        assert_eq!(c.parts.len(), 2);
        assert_eq!(c.part(FourCC::SFI0).unwrap().data, &[1, 2, 3, 4]);
        assert_eq!(c.part(FourCC::HASH).unwrap().data.len(), 20);
        assert!(c.part(FourCC::DXIL).is_none());
    }

    #[test]
    fn container_in_a_part_is_not_another_container() {
        let inner = build(&[(FourCC::SFI0, &[1, 2, 3, 4])]);
        let outer = build(&[(FourCC::ILDB, &inner), (FourCC::HASH, &[9; 20])]);
        let other = build(&[(FourCC::STAT, &[7; 12])]);
        let mut data = b"gap".to_vec();
        let outer_at = data.len();
        data.extend_from_slice(&outer);
        data.extend_from_slice(b"gap");
        let other_at = data.len();
        data.extend_from_slice(&other);
        let inner_at = outer_at + memchr::memmem::find(&outer[4..], b"DXBC").unwrap() + 4;
        // The nested one parses on its own.
        assert!(Container::parse(&data[inner_at..]).is_ok());

        let found = find_all(&data, |_| false);
        let offsets: Vec<_> = found.iter().map(|&(o, _)| o).collect();
        assert_eq!(offsets, [outer_at, other_at]);
        assert_eq!(found[0].1, &outer[..]);
        assert_eq!(found[1].1, &other[..]);

        // Skipping the outer one exposes the one it holds.
        let found = find_all(&data, |o| o == outer_at);
        let offsets: Vec<_> = found.iter().map(|&(o, _)| o).collect();
        assert_eq!(offsets, [inner_at, other_at]);
        assert_eq!(found[0].1, &inner[..]);
    }

    #[test]
    fn header_reading_into_a_container_does_not_hide_it() {
        // A magic 21 bytes before a real container: its digest and version
        // are the junk, its size and part count come from the real header
        // (0x43, the real "C", and 0 parts), so it parses, with no parts.
        let real = build(&[(FourCC::SFI0, &[1, 2, 3, 4])]);
        let mut data = b"DXBC not a container ".to_vec();
        let real_at = data.len();
        data.extend_from_slice(&real);
        assert_eq!(Container::parse(&data).unwrap().parts.len(), 0);

        let offsets: Vec<_> = find_all(&data, |_| false).iter().map(|&(o, _)| o).collect();
        assert_eq!(offsets, [0, real_at]);
    }

    #[test]
    fn finds_nothing_in_noise() {
        assert!(find_all(b"", |_| false).is_empty());
        assert!(find_all(b"DXBC", |_| false).is_empty());
        let mut bytes = build(&[(FourCC::SFI0, &[0; 8])]);
        bytes.pop(); // the stated size runs past the end
        assert!(find_all(&bytes, |_| false).is_empty());
    }

    #[test]
    fn rejects_bad_input() {
        assert_eq!(Container::parse(b"nope").unwrap_err(), ShaderError::NotDxbc);
        let mut bytes = build(&[(FourCC::SFI0, &[0; 8])]);
        let last = bytes.len() - 1;
        bytes[0x20] = 0xFF; // corrupt the first part offset
        assert!(Container::parse(&bytes).is_err());
        bytes.truncate(last);
        assert!(Container::parse(&bytes).is_err());
    }
}
