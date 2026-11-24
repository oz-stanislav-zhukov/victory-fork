//! SGD5 records and the raw `DXBC` scan together, as `victory shader`
//! (`apps/victory/src/shader.rs`, `sources`) combines them: every record,
//! then every container stored as is outside the records and outside the
//! parts of other containers. The CLI does not build on Linux, so its
//! composition is checked here.

use victory_game::sgd5::{self, MAGIC};
use victory_shader::Container;
use victory_shader::dxbc::find_all;

/// What `sources` yields: the records, and the raw containers as (offset,
/// bytes).
fn survey(data: &[u8]) -> (Vec<sgd5::Record<'_>>, Vec<(usize, &[u8])>) {
    let records: Vec<_> = if sgd5::is_sgd5(data) {
        sgd5::records(data).collect()
    } else {
        Vec::new()
    };
    let raw = find_all(data, |offset| sgd5::in_record(&records, offset));
    (records, raw)
}

/// A container with the given parts, laid out the way `Container::parse`
/// reads one.
fn container(parts: &[(&[u8; 4], &[u8])]) -> Vec<u8> {
    let table = 0x20 + 4 * parts.len();
    let mut body = Vec::new();
    let mut offsets = Vec::new();
    for (fourcc, data) in parts {
        offsets.push((table + body.len()) as u32);
        body.extend_from_slice(*fourcc);
        body.extend_from_slice(&(data.len() as u32).to_le_bytes());
        body.extend_from_slice(data);
    }
    let mut out = b"DXBC".to_vec();
    out.extend_from_slice(&[0xA5; 16]);
    out.extend_from_slice(&[1, 0, 0, 0]);
    out.extend_from_slice(&((table + body.len()) as u32).to_le_bytes());
    out.extend_from_slice(&(parts.len() as u32).to_le_bytes());
    for o in offsets {
        out.extend_from_slice(&o.to_le_bytes());
    }
    out.extend(body);
    out
}

/// A record at the end of `file`, returning its marker's offset.
fn push_record(file: &mut Vec<u8>, name: &str, payload: &[u8], level: u8) -> usize {
    let packed = miniz_oxide::deflate::compress_to_vec(payload, level);
    let n = (packed.len() + 12) as u32;
    file.push(name.len() as u8 + 1);
    file.extend_from_slice(name.as_bytes());
    file.extend_from_slice(&[0, 0x32]);
    let at = file.len() + 4;
    for v in [n, 0x000D_DCE7, n, payload.len() as u32] {
        file.extend_from_slice(&v.to_le_bytes());
    }
    file.extend_from_slice(&packed);
    at
}

#[test]
fn stored_record_container_is_counted_once() {
    // A record stored as is (level 0), so its container's bytes, magic and
    // all, sit in the file as they are; then a container between records.
    let inner = container(&[(b"SFI0", &[1, 2, 3, 4])]);
    let mut file = MAGIC.to_vec();
    file.push(0x01);
    let first = push_record(&mut file, "PS_stored", &inner, 0);
    let between = file.len();
    let gap = container(&[(b"STAT", &[7; 12])]);
    file.extend_from_slice(&gap);
    file.push(0x01);
    let second = push_record(&mut file, "PS_packed", &inner, 6);

    let (records, raw) = survey(&file);
    let offsets: Vec<_> = records.iter().map(|r| r.offset).collect();
    assert_eq!(offsets, [first, second]);
    for r in &records {
        let bytes = r.unpack().expect("record unpacks");
        assert!(Container::parse(&bytes).is_ok());
    }
    assert_eq!(raw, [(between, &gap[..])]);

    // Without the magic the file is no SGD5 file: the stored container is
    // found as is, at its place in the record's packed bytes.
    let plain = &file[MAGIC.len()..];
    let (records, raw) = survey(plain);
    assert!(records.is_empty());
    let stored = memchr::memmem::find(plain, b"DXBC").unwrap();
    let offsets: Vec<_> = raw.iter().map(|&(o, _)| o).collect();
    assert_eq!(offsets, [stored, between - MAGIC.len()]);
    assert_eq!(raw[0].1, &inner[..]);
}

/// Seeded random archives: another file's container before the magic, then
/// records (some stored as is, holding a container, a record and a container
/// of their own) with gaps of noise, magics that do not parse, and
/// containers stored as is, some keeping another container in a part. The
/// survey yields exactly the planted records and the planted top-level
/// containers, each once, and no raw container overlaps a record.
#[test]
fn random_archives_survey_each_container_once() {
    let mut x = 0x2545_F491_u32;
    let mut next = move || {
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        x
    };
    for seed in 0..200 {
        let mut planted_raw: Vec<(usize, Vec<u8>)> = Vec::new();
        let mut planted: Vec<(usize, Vec<u8>)> = Vec::new();
        let mut gap_container = |file: &mut Vec<u8>, next: &mut dyn FnMut() -> u32| {
            let data: Vec<u8> = (0..4 + next() % 60).map(|_| next() as u8).collect();
            let c = if next() & 1 == 0 {
                let kept = container(&[(b"SFI0", &data)]);
                container(&[(b"ILDB", &kept), (b"HASH", &[9; 20])])
            } else {
                container(&[(b"STAT", &data)])
            };
            planted_raw.push((file.len(), c.clone()));
            file.extend_from_slice(&c);
        };

        let mut file = b"other file".to_vec();
        gap_container(&mut file, &mut next);
        file.extend_from_slice(MAGIC);
        for i in 0..1 + next() % 5 {
            for _ in 0..next() % 30 {
                file.push(next() as u8);
            }
            if next() % 3 == 0 {
                // A magic whose header does not parse (size past the end).
                file.extend_from_slice(b"DXBC");
                file.extend_from_slice(&[0xFF; 28]);
            }
            if next() & 1 == 0 {
                gap_container(&mut file, &mut next);
            }
            // Noise before a name's length byte must not read as the name.
            file.push(0x01);
            let data: Vec<u8> = (0..4 + next() % 200).map(|_| next() as u8).collect();
            let mut payload = container(&[(b"DXIL", &data), (b"HASH", &[3; 20])]);
            let level = if next() & 1 == 0 {
                let mut stored = Vec::new();
                push_record(&mut stored, "PS_stored", &payload, 0);
                payload.extend(stored);
                payload.extend(container(&[(b"SFI0", &[5; 8])]));
                0
            } else {
                6
            };
            let at = push_record(&mut file, &format!("PS_{seed}_{i}"), &payload, level);
            planted.push((at, payload));
        }
        for _ in 0..next() % 30 {
            file.push(next() as u8);
        }
        if next() & 1 == 0 {
            gap_container(&mut file, &mut next);
        }

        let (records, raw) = survey(&file);
        let got: Vec<_> = records.iter().map(|r| (r.offset, r.unpack())).collect();
        let want: Vec<_> = planted
            .iter()
            .map(|(at, payload)| (*at, Some(payload.clone())))
            .collect();
        assert_eq!(got, want, "seed {seed}: records");
        for (_, bytes) in &got {
            let bytes = bytes.as_deref().unwrap();
            assert!(Container::parse(bytes).is_ok(), "seed {seed}");
        }
        let raw_got: Vec<_> = raw.iter().map(|&(o, b)| (o, b.to_vec())).collect();
        assert_eq!(raw_got, planted_raw, "seed {seed}: raw containers");
        for &(offset, bytes) in &raw {
            let span = offset..offset + bytes.len();
            for r in &records {
                let rs = r.span();
                assert!(
                    span.end <= rs.start || rs.end <= span.start,
                    "seed {seed}: raw {span:?} overlaps record {rs:?}"
                );
            }
        }
    }
}
