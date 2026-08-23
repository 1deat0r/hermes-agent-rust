//! Shared magic-byte audio/AV container detection.
//!
//! PARITY: tools/audio_container.py @ b9aa928 (97 LOC, ported 1:1).
//!
//! ONE sniffer owns container detection for the whole codebase:
//! - **Outbound** (`tools/tts_tool.py`): TTS backends silently ignore the
//!   requested opus format (Edge emits MP3, Piper writes WAV, ...), so the
//!   synthesized file is sniffed and repaired when the bytes don't match the
//!   `.ogg` extension (PR #73072).
//! - **Inbound** (`gateway/platforms/base.py` `cache_audio_from_bytes` /
//!   `cache_audio_from_url`): platform adapters frequently pass a wrong or
//!   guessed extension for voice notes (Telegram `.oga`, iOS Signal M4A-branded
//!   MP4, RIFF/WAVE attachments). The cache sniffs the real container so STT
//!   and downstream players get an honest extension — the inbound mirror of
//!   the outbound repair.
//! - `gateway/platforms/signal.py` `_guess_extension` delegates its audio/AV
//!   branches here instead of duplicating the byte patterns.

/// Container id -> canonical file extension.
///
/// Iterable like the upstream dict; lookup helper `container_to_ext`.
pub const CONTAINER_TO_EXT: &[(&str, &str)] = &[
    ("m4a", ".m4a"),
    ("mp4", ".mp4"),
    ("ogg", ".ogg"),
    ("flac", ".flac"),
    ("wav", ".wav"),
    ("mp3", ".mp3"),
    ("aac", ".aac"),
    ("webm", ".webm"),
];

/// Look up the canonical extension for a container id (`CONTAINER_TO_EXT[..]`).
pub fn container_to_ext(container: &str) -> Option<&'static str> {
    CONTAINER_TO_EXT
        .iter()
        .find(|(id, _)| *id == container)
        .map(|(_, ext)| *ext)
}

/// Return a container id from magic bytes, or `None` when unknown.
///
/// Possible ids: `m4a`, `mp4`, `ogg`, `flac`, `wav`, `mp3`, `aac`, `webm`.
/// Only audio/AV containers are claimed — images (including RIFF/WEBP)
/// return `None` so callers can layer their own image detection first.
pub fn sniff_container(data: &[u8]) -> Option<&'static str> {
    if data.len() >= 8 && &data[4..8] == b"ftyp" {
        // Brand at bytes 8-11: audio brands ("M4A ", "M4B ") are voice
        // notes / audiobooks; everything else (isom/mp42/avc1/qt) is video.
        // PARITY: upstream compares `data[8:12].lower()` against the
        // lowercase brand bytes; `eq_ignore_ascii_case` is equivalent.
        if data.len() >= 12
            && (data[8..12].eq_ignore_ascii_case(b"m4a ")
                || data[8..12].eq_ignore_ascii_case(b"m4b "))
        {
            return Some("m4a");
        }
        return Some("mp4");
    }
    if data.starts_with(b"OggS") {
        return Some("ogg");
    }
    if data.starts_with(b"fLaC") {
        return Some("flac");
    }
    if data.len() >= 12 && data.starts_with(b"RIFF") && &data[8..12] == b"WAVE" {
        return Some("wav");
    }
    if data.starts_with(b"ID3") {
        return Some("mp3");
    }
    if data.len() >= 2 && data[0] == 0xFF && (data[1] & 0xE0) == 0xE0 {
        // `0xFF 0xFx` is shared by MP3 and ADTS AAC. Bits 3-1 of byte 1
        // disambiguate: ADTS has `ID=0` and `layer=00` (mask 0xF6, target
        // 0xF0); MP3 has `ID=1` and `layer` in {01,10,11}.
        if (data[1] & 0xF6) == 0xF0 {
            return Some("aac");
        }
        return Some("mp3");
    }
    if data.starts_with(b"\x1a\x45\xdf\xa3") {
        return Some("webm");
    }
    None
}

/// Return a container-matching extension, or `fallback_ext` when unknown.
///
/// Used on inbound audio paths where the caller *claims* the bytes are audio:
/// generic MP4 containers are mapped to `.m4a` (audio-in-MP4) because in an
/// audio context the payload is AAC audio regardless of brand — STT accepts
/// `.m4a`/`.mp4` but voice-bubble routing keys off audio extensions.
pub fn sniff_audio_ext(data: &[u8], fallback_ext: &str) -> String {
    let fallback = if fallback_ext.starts_with('.') {
        fallback_ext.to_string()
    } else {
        format!(".{fallback_ext}")
    };
    match sniff_container(data) {
        None => fallback,
        Some("mp4") => ".m4a".to_string(),
        Some(container) => container_to_ext(container)
            .expect("sniff_container only returns ids in CONTAINER_TO_EXT")
            .to_string(),
    }
}
