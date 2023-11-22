use crate::storage;

pub fn get_tracks_from_playlist(name: str) -> Result<[CachedTrack]> {
    let file = File::open("juicebox/playlists/" + name)?;
    let mut contents = String::new();
    file.read_to_string(&mut content);
}