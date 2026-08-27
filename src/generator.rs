use anyhow::Result;
use regex::Regex;
use std::fs;
use std::path::Path;
use std::env;

pub fn generate_template(output: Option<String>) -> Result<()> {
    let current_dir = env::current_dir()?;
    let dir_name = current_dir.file_name().unwrap_or_default().to_string_lossy().to_string();

    // Determine if current directory is a disc directory (Disc N or CD N)
    let (disc_number, parent_dir) = detect_disc_directory(&current_dir);

    // Get album and date from the appropriate directory
    let (album, date) = if let Some(parent) = parent_dir {
        // We're in a disc subdirectory - use parent for album/date
        let parent_name = parent.file_name().unwrap_or_default().to_string_lossy().to_string();
        parse_album_from_dir(&parent_name)
    } else {
        // We're in the album directory
        parse_album_from_dir(&dir_name)
    };

    // Find all audio files
    let audio_extensions = ["flac", "mp3", "ape", "aac", "opus", "m4a", "wv", "wma"];
    let mut tracks = Vec::new();

    for entry in fs::read_dir(&current_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(ext) = path.extension() {
                let ext_lower = ext.to_string_lossy().to_lowercase();
                if audio_extensions.contains(&ext_lower.as_str()) {
                    if let Some(file_name) = path.file_name() {
                        let file_name_str = file_name.to_string_lossy().to_string();
                        let (track_num, title) = parse_filename(&file_name_str);
                        tracks.push((file_name_str, track_num, title));
                    }
                }
            }
        }
    }

    if tracks.is_empty() {
        println!("No audio files found in current directory.");
        return Ok(());
    }

    // Sort tracks by track number
    tracks.sort_by(|a, b| {
        let num_a = a.1.as_deref().unwrap_or("999").parse::<u32>().unwrap_or(999);
        let num_b = b.1.as_deref().unwrap_or("999").parse::<u32>().unwrap_or(999);
        num_a.cmp(&num_b)
    });

    // Generate INI content
    let mut content = String::new();
    content.push_str("[Global]\n");
    if let Some(album_name) = album {
        content.push_str(&format!("Album = \"{}\"\n", album_name));
    } else {
        content.push_str("Album = \n");
    }
    content.push_str("Album Artist = \n");
    if let Some(date_str) = date {
        content.push_str(&format!("Date = \"{}\"\n", date_str));
    }
    content.push_str("Genre = \n");
    content.push_str("Composer = \n\n");

    // Add disc section if we detected a disc number
    if let Some(disc) = disc_number {
        content.push_str(&format!("[Disc {}]\n\n", disc));
    }

    for (file_name, track_num, title) in &tracks {
        let track_header = if let Some(num) = track_num {
            format!("[Track {}]", num)
        } else {
            "[Track]".to_string()
        };
        content.push_str(&format!("{}\n", track_header));
        content.push_str(&format!("File = \"{}\"\n", file_name));
        if let Some(title_str) = title {
            content.push_str(&format!("Title = \"{}\"\n", title_str));
        } else {
            content.push_str("Title = \n");
        }
        content.push_str("Artist = \n\n");
    }

    // Write output file
    let output_path = output.unwrap_or_else(|| "tags.ini".to_string());
    fs::write(&output_path, content)?;
    println!("Generated template: {}", output_path);
    println!("Found {} tracks", tracks.len());
    if let Some(disc) = disc_number {
        println!("Detected disc: {}", disc);
    }

    Ok(())
}

fn detect_disc_directory(dir: &Path) -> (Option<u32>, Option<&Path>) {
    let dir_name = dir.file_name().unwrap_or_default().to_string_lossy().to_lowercase();

    // Check for "disc N" or "discN"
    let re = Regex::new(r"^disc\s*(\d+)$").unwrap();
    if let Some(caps) = re.captures(&dir_name) {
        if let Some(num_str) = caps.get(1) {
            if let Ok(num) = num_str.as_str().parse::<u32>() {
                return (Some(num), dir.parent());
            }
        }
    }

    // Check for "cd N" or "cdN"
    let re = Regex::new(r"^cd\s*(\d+)$").unwrap();
    if let Some(caps) = re.captures(&dir_name) {
        if let Some(num_str) = caps.get(1) {
            if let Ok(num) = num_str.as_str().parse::<u32>() {
                return (Some(num), dir.parent());
            }
        }
    }

    (None, None)
}

fn parse_album_from_dir(dir_name: &str) -> (Option<String>, Option<String>) {
    // Check for pattern: "YYYY - Album Name" or "YYYY Album Name" or "YYYY. Album Name"
    let patterns = [
        r"^(\d{4})\s*[-–—]\s*(.+)$",  // 2023 - Something
        r"^(\d{4})\s+(.+)$",            // 2023 Something
        r"^(\d{4})\.\s*(.+)$",          // 2023. Something
        r"^\((\d{4})\)\s*(.+)$",        // (2023) Something
        r"^\[(\d{4})\]\s*(.+)$",        // [2023] Something
    ];

    let re_year = Regex::new(r"^(\d{4})").unwrap();
    let re_separator = Regex::new(r"^\d{4}\s*[-–—]\s*(.+)$").unwrap();

    for pattern in patterns {
        let re = Regex::new(pattern).unwrap();
        if let Some(caps) = re.captures(dir_name) {
            if let (Some(date_match), Some(album_match)) = (caps.get(1), caps.get(2)) {
                let date = date_match.as_str().to_string();
                let album = album_match.as_str().trim().to_string();
                if !album.is_empty() {
                    return (Some(album), Some(date));
                }
            }
        }
    }

    // Try simpler: just check if it starts with 4 digits
    if let Some(caps) = re_year.captures(dir_name) {
        if let Some(date_match) = caps.get(1) {
            let date = date_match.as_str().to_string();
            // Try to extract album after separator
            if let Some(caps) = re_separator.captures(dir_name) {
                if let Some(album_match) = caps.get(1) {
                    let album = album_match.as_str().trim().to_string();
                    if !album.is_empty() {
                        return (Some(album), Some(date));
                    }
                }
            }
            // No separator, just use the rest after the year
            let rest = dir_name.trim_start_matches(&date).trim();
            if !rest.is_empty() {
                return (Some(rest.to_string()), Some(date));
            }
        }
    }

    // No pattern matched - use whole directory name as album
    (Some(dir_name.to_string()), None)
}

fn parse_filename(file_name: &str) -> (Option<String>, Option<String>) {
    // Remove extension
    let name = Path::new(file_name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(file_name);

    // Try patterns:
    // 1. "03. Title" -> track=03, title=Title
    // 2. "03 - Title" -> track=03, title=Title
    // 3. "03 Title" -> track=03, title=Title
    // 4. "03-Title" -> track=03, title=Title
    // 5. "03_ Title" -> track=03, title=Title

    let patterns = [
        r"^(\d{1,3})\s*[\.\-_]\s*(.+)$",     // 03. Title, 03 - Title, 03_Title
        r"^(\d{1,3})\s+(.+)$",                // 03 Title
        r"^(\d{1,3})[\.\-_](.+)$",            // 03.Title, 03-Title
    ];

    for pattern in patterns {
        let re = Regex::new(pattern).unwrap();
        if let Some(caps) = re.captures(name) {
            if let (Some(num_match), Some(title_match)) = (caps.get(1), caps.get(2)) {
                let track_num = num_match.as_str().to_string();
                let title = title_match.as_str().trim().to_string();
                if !title.is_empty() {
                    return (Some(track_num), Some(title));
                }
            }
        }
    }

    // No pattern matched - just use filename without extension as title
    (None, Some(name.to_string()))
}
