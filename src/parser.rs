use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Default, Clone)]
pub struct GlobalMeta {
    pub album: Option<String>,
    pub album_artist: Vec<String>,
    pub composer: Option<String>,
    pub genre: Option<String>,
    pub date: Option<String>,
    pub comment: Option<String>,
    pub conductor: Option<String>,
    pub publisher: Option<String>,
    pub copyright: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TrackMeta {
    pub file: String,
    pub number: Option<String>,
    pub title: Option<String>,
    pub artist: Vec<String>,
    pub disc: Option<u8>,
    pub composer: Option<String>,
    pub genre: Option<String>,
    pub date: Option<String>,
    pub comment: Option<String>,
    pub conductor: Option<String>,
}

impl Default for TrackMeta {
    fn default() -> Self {
        Self {
            file: String::new(),
            number: None,
            title: None,
            artist: Vec::new(),
            disc: None,
            composer: None,
            genre: None,
            date: None,
            comment: None,
            conductor: None,
        }
    }
}

fn normalize_key(key: &str) -> String {
    key.trim()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("")
}

pub fn parse_config(path: &Path) -> anyhow::Result<(GlobalMeta, Vec<TrackMeta>)> {
    let content = fs::read_to_string(path)?;
    let lines: Vec<&str> = content.lines().collect();

    let mut global = GlobalMeta::default();
    let mut tracks = Vec::new();
    let mut current_disc: Option<u8> = None;
    let mut current_track: Option<TrackMeta> = None;

    for line in lines {
        let line = line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }

        // Check for [Global] section
        if line == "[Global]" {
            continue;
        }

        // Check for [Disc X]
        if let Some(disc_str) = line.strip_prefix("[Disc") {
            if let Some(num_str) = disc_str.strip_suffix(']') {
                if let Ok(num) = num_str.trim().parse::<u8>() {
                    current_disc = Some(num);
                }
            }
            continue;
        }

        // Check for [Track X] section
        if line.starts_with("[Track") {
            // Push previous track if exists
            if let Some(mut track) = current_track.take() {
                track.disc = track.disc.or(current_disc);
                tracks.push(track);
            }

            let mut track = TrackMeta::default();

            // Extract track number from [Track 01] or [Track]
            if let Some(inner) = line.strip_prefix("[Track") {
                if let Some(num_str) = inner.strip_suffix(']') {
                    let num_str = num_str.trim();
                    if !num_str.is_empty() {
                        track.number = Some(num_str.to_string());
                    }
                }
            }

            current_track = Some(track);
            continue;
        }

        // Parse key = value (or key: value)
        let (key, raw_value) = if let Some((k, v)) = line.split_once('=') {
            (k, v)
        } else if let Some((k, v)) = line.split_once(':') {
            (k, v)
        } else {
            continue;
        };

        let key = key.trim();
        let raw_value = raw_value.trim();
        let normalized = normalize_key(key);

        // Check if there was any value after the separator
        let has_value = !raw_value.is_empty();
        let value = if has_value {
            raw_value.trim_matches('"').to_string()
        } else {
            String::new()
        };

        if let Some(track) = &mut current_track {
            match normalized.as_str() {
                "file" => track.file = value,
                "number" => track.number = Some(value),
                "title" => track.title = Some(value),
                "artist" => {
                    if has_value {
                        track.artist.push(value);
                    }
                    // If no value, skip entirely (don't push anything)
                }
                "disc" => {
                    if let Ok(num) = value.parse::<u8>() {
                        track.disc = Some(num);
                    }
                }
                "composer" => track.composer = Some(value),
                "genre" => track.genre = Some(value),
                "date" => track.date = Some(value),
                "comment" => track.comment = Some(value),
                "conductor" => track.conductor = Some(value),
                _ => {} // Ignore unknown keys in track
            }
        } else {
            // Global section
            match normalized.as_str() {
                "album" => global.album = Some(value),
                "albumartist" => {
                    if has_value {
                        global.album_artist.push(value);
                    }
                    // If no value, skip entirely
                }
                "composer" => global.composer = Some(value),
                "genre" => global.genre = Some(value),
                "date" => global.date = Some(value),
                "comment" => global.comment = Some(value),
                "conductor" => global.conductor = Some(value),
                "publisher" => global.publisher = Some(value),
                "copyright" => global.copyright = Some(value),
                _ => {} // Ignore unknown keys in global
            }
        }
    }

    // Push last track
    if let Some(mut track) = current_track.take() {
        track.disc = track.disc.or(current_disc);
        tracks.push(track);
    }

    Ok((global, tracks))
}
