use crate::storage::CachedTrack;
use std::{fs::{File, self}, io::Read, io::Write};
use serde::{Serialize, Deserialize};


type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Serialize, Deserialize, Debug)]
pub struct Playlist {
    name: String,
    file_name: String,
    image_path: Option<String>,
    tracks: Vec<CachedTrack>
}

pub enum PlaylistIns {
    RELOAD,
    DELETE
}

impl Playlist {

    pub fn new(name: &str, file_name: &str) -> Playlist {
        Playlist {
            name: String::from(name), 
            file_name: String::from("juicebox/playlists/".to_owned() + file_name + ".json"),
            image_path: None, 
            tracks: Vec::new()}
    }
    
    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_file_name(&self) -> &str {
        &self.file_name
    }

    pub fn add_track(&mut self, track: CachedTrack) {
        if !self.tracks.contains(&track) {
            let _ = &self.tracks.push(track);
        }
    }

    pub fn set_cover(&mut self, path: String) {
        self.image_path = Some(path);
    }

    pub fn get_tracks(&self) -> &Vec<CachedTrack> {
        &self.tracks
    }

}

pub fn get_playlist(file_name: &str) -> Result<Playlist> {
    let mut file = File::open(file_name)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

   let p: Playlist = serde_json::from_str(&contents)?;
   return Ok(p);
}

pub fn save_playlist(playlist: &Playlist) -> Result<()> {

    let j = serde_json::to_string_pretty(playlist)?;
    let mut file = File::create(playlist.file_name.as_str())?;
    let res = fs::write(playlist.file_name.as_str(), j);
    return Ok(res?);
}

pub fn load_playlists() -> Result<Vec<Playlist>> {
    let files = fs::read_dir("juicebox/playlists/").unwrap();
    let mut playlists: Vec<Playlist> = Vec::new();
    for file in files {
        let p = get_playlist(file.unwrap().path().to_str().unwrap())?;
        playlists.push(p);
    }

    Ok(playlists)
}