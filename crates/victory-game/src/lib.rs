//! Everything specific to the title: its container formats, build
//! identities, and the hooks we place in its code (located by per-build
//! offsets or byte-pattern signatures). What may and may not be stored here
//! is defined in docs/asset-policy.md.

pub mod sgd5;

/// A specific build of the title executable we know how to run.
#[derive(Debug, Clone, Copy)]
pub struct KnownBuild {
    /// Our own label, e.g. "1.0.0 retail".
    pub label: &'static str,
    /// BLAKE3 of the original executable, hex.
    pub blake3: &'static str,
}

/// Builds we support. Empty until the executable is available.
pub const KNOWN_BUILDS: &[KnownBuild] = &[];

/// Looks up a build by the BLAKE3 hash of its executable.
pub fn identify(blake3_hex: &str) -> Option<&'static KnownBuild> {
    KNOWN_BUILDS
        .iter()
        .find(|b| b.blake3.eq_ignore_ascii_case(blake3_hex))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A BLAKE3 digest written the way `blake3::Hash::to_hex` writes it.
    fn is_blake3_hex(s: &str) -> bool {
        s.len() == 64 && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
    }

    #[test]
    fn known_builds_are_well_formed() {
        for (i, build) in KNOWN_BUILDS.iter().enumerate() {
            assert!(!build.label.trim().is_empty(), "build {i} has no label");
            assert!(
                is_blake3_hex(build.blake3),
                "{:?}: blake3 must be 64 lowercase hex digits, got {:?}",
                build.label,
                build.blake3
            );
            for other in &KNOWN_BUILDS[..i] {
                assert_ne!(other.blake3, build.blake3, "{:?} listed twice", build.label);
                assert_ne!(
                    other.label, build.label,
                    "label {:?} used twice",
                    build.label
                );
            }
        }
    }

    #[test]
    fn every_known_build_is_identified_by_its_hash() {
        for build in KNOWN_BUILDS {
            let found = identify(build.blake3).expect("own hash identifies the build");
            assert_eq!(found.label, build.label);
            let upper = build.blake3.to_ascii_uppercase();
            assert_eq!(identify(&upper).map(|b| b.label), Some(build.label));
        }
    }

    #[test]
    fn unknown_hashes_are_not_identified() {
        // BLAKE3 of empty input: no executable hashes to it.
        let empty = "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262";
        assert!(is_blake3_hex(empty));
        assert!(identify(empty).is_none());
        assert!(identify("").is_none());
        assert!(identify("not a hash").is_none());
    }
}
