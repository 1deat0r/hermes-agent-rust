//! Parity oracles for the shared magic-byte audio container sniffer,
//! mirroring upstream tests/tools/test_audio_container.py @ b9aa928.
//!
//! Evidence tier: unit (pure byte-sniffing, no I/O).
//!
//! DEFERRED upstream cases (need unported subsystems, noted per rule 4):
//! - `TestInboundCacheUsesSniffer` — gateway/platforms/base.py
//!   `cache_audio_from_bytes` / `cache_audio_from_url` (gateway crate unported).
//! - `TestSignalDelegatesToCentralSniffer` — gateway/platforms/signal.py
//!   `_guess_extension` delegation (gateway crate unported).

use hermes_tools::audio_container::{container_to_ext, sniff_audio_ext, sniff_container, CONTAINER_TO_EXT};

// --- canonical headers ----------------------------------------------------
// Mirrors the upstream byte constants verbatim; `padded` appends the same
// trailing +64 zero bytes as upstream's `+ b"\x00" * 64`.
fn padded(head: &[u8]) -> Vec<u8> {
    let mut v = head.to_vec();
    v.resize(head.len() + 64, 0);
    v
}

fn ogg() -> Vec<u8> { padded(b"OggS\x00\x02") }
fn flac() -> Vec<u8> { padded(b"fLaC") }
fn wav() -> Vec<u8> { padded(b"RIFF\x24\x08\x00\x00WAVEfmt ") }
fn webp() -> Vec<u8> { padded(b"RIFF\x24\x08\x00\x00WEBPVP8 ") }
fn mp3_id3() -> Vec<u8> { padded(b"ID3\x04\x00\x00\x00\x00\x00\x00") }
fn mp3_frame() -> Vec<u8> { padded(b"\xff\xfb\x90\x00") }
fn aac_adts() -> Vec<u8> { padded(b"\xff\xf1\x50\x80") }
fn m4a() -> Vec<u8> { padded(b"\x00\x00\x00\x1cftypM4A ") }
fn m4b() -> Vec<u8> { padded(b"\x00\x00\x00\x1cftypM4B ") }
fn mp4_isom() -> Vec<u8> { padded(b"\x00\x00\x00\x18ftypisom") }
fn webm() -> Vec<u8> { padded(b"\x1a\x45\xdf\xa3") }
fn unknown() -> Vec<u8> { padded(b"not-audio-at-all") }

// TestSniffContainer.test_magic_bytes — all 10 parametrized cases.
#[test]
fn magic_bytes() {
    let cases: Vec<(Vec<u8>, &str)> = vec![
        (ogg(), "ogg"),
        (flac(), "flac"),
        (wav(), "wav"),
        (mp3_id3(), "mp3"),
        (mp3_frame(), "mp3"),
        (aac_adts(), "aac"),
        (m4a(), "m4a"),
        (m4b(), "m4a"),
        (mp4_isom(), "mp4"),
        (webm(), "webm"),
    ];
    for (data, expected) in cases {
        assert_eq!(sniff_container(&data), Some(expected));
    }
}

// Supplementary module-contract cases (upstream code is the oracle; the
// upstream test module defines WEBP but never sniffs it): images return None.
#[test]
fn images_are_not_claimed() {
    assert_eq!(sniff_container(&webp()), None);
    assert_eq!(sniff_container(&unknown()), None);
    assert_eq!(sniff_container(b""), None);
    assert_eq!(sniff_container(b"x"), None);
}

// TestSniffContainer.test_every_container_has_an_extension
#[test]
fn every_container_has_an_extension() {
    for data in [ogg(), flac(), wav(), mp3_id3(), aac_adts(), m4a(), mp4_isom(), webm()] {
        let container = sniff_container(&data).expect("container sniffs");
        assert!(
            container_to_ext(container).is_some(),
            "container {container:?} missing from CONTAINER_TO_EXT"
        );
    }
}

// TestSniffAudioExt.test_container_wins_over_claimed_ext — 8 parametrized cases.
#[test]
fn container_wins_over_claimed_ext() {
    let cases: Vec<(Vec<u8>, &str, &str)> = vec![
        (ogg(), ".ogg", ".mp3"),
        (flac(), ".flac", ".mp3"),
        (wav(), ".wav", ".ogg"),
        (mp3_id3(), ".mp3", ".ogg"),
        (mp3_frame(), ".mp3", ".ogg"),
        (aac_adts(), ".aac", ".ogg"),
        (m4a(), ".m4a", ".ogg"),
        (webm(), ".webm", ".ogg"),
    ];
    for (data, expected, claimed) in cases {
        assert_eq!(sniff_audio_ext(&data, claimed), expected);
    }
}

// Upstream: sniff_audio_ext(data, ".ogg" if expected != ".ogg" else ".mp3")
// — the claimed ext above is the complementing one per that rule.

// TestSniffAudioExt.test_fallback_without_dot_is_normalized
#[test]
fn fallback_without_dot_is_normalized() {
    assert_eq!(sniff_audio_ext(&unknown(), "mp3"), ".mp3");
    assert_eq!(sniff_audio_ext(&unknown(), ".wav"), ".wav");
    // Generic MP4 containers map to .m4a in an audio context.
    assert_eq!(sniff_audio_ext(&mp4_isom(), ".ogg"), ".m4a");
}

// CONTAINER_TO_EXT surface: id -> extension pairs match upstream exactly.
#[test]
fn container_to_ext_mapping() {
    let expected = [
        ("m4a", ".m4a"),
        ("mp4", ".mp4"),
        ("ogg", ".ogg"),
        ("flac", ".flac"),
        ("wav", ".wav"),
        ("mp3", ".mp3"),
        ("aac", ".aac"),
        ("webm", ".webm"),
    ];
    assert_eq!(CONTAINER_TO_EXT.len(), expected.len());
    for (id, ext) in expected {
        assert_eq!(container_to_ext(id), Some(ext));
    }
    assert_eq!(container_to_ext("nope"), None);
}
