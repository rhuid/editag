use anyhow::Result;
use regex::Regex;
use std::fs;
use std::path::Path;
use std::env;
use std::process::Command;

pub fn generate_template(output: Option<String>, from_file: bool) -> Result<()> {
    if from_file {
        generate_from_files(output)
    } else {
        generate_from_directory(output)
    }
}

// --- Directory-based generation ---

fn generate_from_directory(output: Option<String>) -> Result<()> {
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

// --- File-based generation ---

fn generate_from_files(output: Option<String>) -> Result<()> {
    let current_dir = env::current_dir()?;
    let audio_extensions = ["flac", "mp3", "ape", "aac", "opus", "m4a", "wv", "wma"];

    // First pass: read all metadata from all files
    let mut all_tracks: Vec<(String, std::collections::HashMap<String, String>)> = Vec::new();

    for entry in fs::read_dir(&current_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(ext) = path.extension() {
                let ext_lower = ext.to_string_lossy().to_lowercase();
                if audio_extensions.contains(&ext_lower.as_str()) {
                    if let Some(file_name) = path.file_name() {
                        let file_name_str = file_name.to_string_lossy().to_string();
                        let metadata = read_metadata_from_file(&path)?;
                        all_tracks.push((file_name_str, metadata));
                    }
                }
            }
        }
    }

    if all_tracks.is_empty() {
        println!("No audio files found in current directory.");
        return Ok(());
    }

    // Determine what goes in Global vs Track
    let (global_fields, track_fields) = analyze_metadata(&all_tracks);

    // Generate INI content
    let mut content = String::new();

    // Global section
    content.push_str("[Global]\n");

    // Global fields in order
    let global_order = ["album", "albumartist", "date", "genre", "composer", "conductor", "comment", "publisher", "copyright"];
    for field in &global_order {
        if let Some(value) = global_fields.get(*field) {
            if !value.is_empty() {
                let key = field_to_ini_key(field);
                content.push_str(&format!("{} = \"{}\"\n", key, value));
            } else {
                let key = field_to_ini_key(field);
                content.push_str(&format!("{} = \n", key));
            }
        } else {
            let key = field_to_ini_key(field);
            content.push_str(&format!("{} = \n", key));
        }
    }
    content.push('\n');

    // Track sections
    for (file_name, metadata) in &all_tracks {
        // Get track number for header
        let track_num = metadata.get("tracknumber").cloned().unwrap_or_default();
        let track_header = if !track_num.is_empty() {
            format!("[Track {}]", track_num)
        } else {
            "[Track]".to_string()
        };
        content.push_str(&format!("{}\n", track_header));

        // File (always first)
        content.push_str(&format!("File = \"{}\"\n", file_name));

        // Title (always from track)
        let title = metadata.get("title").cloned().unwrap_or_default();
        content.push_str(&format!("Title = \"{}\"\n", title));

        // Artist (always from track)
        let artist = metadata.get("artist").cloned().unwrap_or_default();
        content.push_str(&format!("Artist = \"{}\"\n", artist));

        // Disc (always from track)
        let disc = metadata.get("discnumber").cloned().unwrap_or_default();
        if !disc.is_empty() {
            content.push_str(&format!("Disc = \"{}\"\n", disc));
        }

        // Track-level overrides for fields that differ
        let override_fields = ["album", "albumartist", "date", "genre", "composer", "conductor", "comment", "publisher", "copyright"];
        for field in &override_fields {
            if let Some(track_value) = metadata.get(*field) {
                if let Some(global_value) = global_fields.get(*field) {
                    if track_value != global_value && !track_value.is_empty() {
                        let key = field_to_ini_key(field);
                        content.push_str(&format!("{} = \"{}\"\n", key, track_value));
                    }
                } else if !track_value.is_empty() {
                    let key = field_to_ini_key(field);
                    content.push_str(&format!("{} = \"{}\"\n", key, track_value));
                }
            }
        }

        content.push('\n');
    }

    // Write output file
    let output_path = output.unwrap_or_else(|| "tags.ini".to_string());
    fs::write(&output_path, content)?;
    println!("Generated template from file metadata: {}", output_path);
    println!("Found {} tracks", all_tracks.len());

    Ok(())
}

fn read_metadata_from_file(path: &Path) -> Result<std::collections::HashMap<String, String>> {
    let mut metadata = std::collections::HashMap::new();

    // Fields to read from kid3-cli
    let fields = [
        "album", "albumartist", "date", "genre", "composer",
        "conductor", "comment", "publisher", "copyright",
        "title", "artist", "tracknumber", "discnumber"
    ];

    for field in &fields {
        let output = Command::new("kid3-cli")
            .arg(path)
            .arg("-c")
            .arg(format!("get {}", field))
            .output()?;

        if output.status.success() {
            let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !value.is_empty() {
                metadata.insert(field.to_string(), value);
            }
        }
    }

    Ok(metadata)
}

fn analyze_metadata(all_tracks: &[(String, std::collections::HashMap<String, String>)]) -> (std::collections::HashMap<String, String>, Vec<std::collections::HashMap<String, String>>) {
    let mut global_fields = std::collections::HashMap::new();
    let mut track_fields = Vec::new();

    // Fields that can be Global or Track
    let analyzable_fields = [
        "album", "albumartist", "date", "genre", "composer",
        "conductor", "comment", "publisher", "copyright"
    ];

    for field in &analyzable_fields {
        let values: Vec<String> = all_tracks.iter()
            .filter_map(|(_, meta)| meta.get(*field).cloned())
            .filter(|v| !v.is_empty())
            .collect();

        if values.is_empty() {
            continue;
        }

        // Check if all values are the same
        let all_same = values.iter().all(|v| v == &values[0]);

        if all_same {
            global_fields.insert(field.to_string(), values[0].clone());
        }
    }

    // Return both maps without Result wrapper
    (global_fields, track_fields)
}

fn field_to_ini_key(field: &str) -> String {
    match field {
        "album" => "Album".to_string(),
        "albumartist" => "Album Artist".to_string(),
        "date" => "Date".to_string(),
        "genre" => "Genre".to_string(),
        "composer" => "Composer".to_string(),
        "conductor" => "Conductor".to_string(),
        "comment" => "Comment".to_string(),
        "publisher" => "Publisher".to_string(),
        "copyright" => "Copyright".to_string(),
        "title" => "Title".to_string(),
        "artist" => "Artist".to_string(),
        "tracknumber" => "Track Number".to_string(),
        "discnumber" => "Disc Number".to_string(),
        _ => field.to_string(),
    }
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
