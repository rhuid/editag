use crate::parser::{GlobalMeta, TrackMeta};
use std::process::Command;
use std::path::Path;

pub fn write_metadata(global: &GlobalMeta, tracks: &[TrackMeta]) -> anyhow::Result<()> {
    for track in tracks {
        // Merge: track data may override global

        let title = track.title.as_deref().unwrap_or("Unknown");

        // Artist: use track artist(s) or fallback to global albumartist
        let artist_str = if !track.artist.is_empty() {
            track.artist.join("; ")
        } else {
            global.album_artist.join("; ")
        };

        let album = track.album.as_deref().or(global.album.as_deref()).unwrap_or("Unknown Album");

        let album_artist_str = if !track.album_artist.is_empty() {
            track.album_artist.join("; ")
        } else {
            global.album_artist.join("; ")
        };

        let publisher = track.publisher.as_deref().or(global.publisher.as_deref()).unwrap_or("");
        let copyright = track.copyright.as_deref().or(global.copyright.as_deref()).unwrap_or("");
        let date = track.date.as_deref().or(global.date.as_deref()).unwrap_or("");
        let genre = track.genre.as_deref().or(global.genre.as_deref()).unwrap_or("");
        let composer = track.composer.as_deref().or(global.composer.as_deref()).unwrap_or("");
        let comment = track.comment.as_deref().or(global.comment.as_deref()).unwrap_or("");
        let conductor = track.conductor.as_deref().or(global.conductor.as_deref()).unwrap_or("");
        let track_num = track.number.as_deref().unwrap_or("");
        let disc = track.disc.map(|d| d.to_string()).unwrap_or_default();

        let path = Path::new(&track.file);
        if !path.exists() {
            eprintln!("Warning: {} not found", track.file);
            continue;
        }

        println!("Writing metadata to {}", track.file);

        let mut cmd = Command::new("kid3-cli");
        cmd.arg(&track.file)
           .arg("-c").arg(format!("set TITLE \"{}\"", title))
           .arg("-c").arg(format!("set ARTIST \"{}\"", artist_str))
           .arg("-c").arg(format!("set ALBUM \"{}\"", album))
           .arg("-c").arg(format!("set ALBUMARTIST \"{}\"", album_artist_str))
           .arg("-c").arg(format!("set DATE \"{}\"", date))
           .arg("-c").arg(format!("set GENRE \"{}\"", genre))
           .arg("-c").arg(format!("set COMPOSER \"{}\"", composer))
           .arg("-c").arg(format!("set COMMENT \"{}\"", comment))
           .arg("-c").arg(format!("set CONDUCTOR \"{}\"", conductor))
           .arg("-c").arg(format!("set PUBLISHER \"{}\"", publisher))
           .arg("-c").arg(format!("set COPYRIGHT \"{}\"", copyright))
           .arg("-c").arg(format!("set TRACKNUMBER \"{}\"", track_num))
           .arg("-c").arg(format!("set DISCNUMBER \"{}\"", disc))
           .arg("-c").arg("save")
           .output()?;
    }

    Ok(())
}
