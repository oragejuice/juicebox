
use crate::scraper;
use std::fs::{OpenOptions, self, File};
use std::io::{Read, Write, Seek};
use async_std::path::Path;
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use serde::{Serialize, Deserialize};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub async fn create_folder() -> Result<()>{
    if !Path::new("juicebox").exists().await {
        fs::create_dir("juicebox")?;
    }
    if !Path::new("juicebox/cache/").exists().await {
        fs::create_dir("juicebox/cache/")?;
    }
    if !Path::new("juicebox/cache/images/").exists().await {
        fs::create_dir("juicebox/cache/images")?;
    }
    if !Path::new("juicebox/cache/tracks.json").exists().await {
        let mut file = File::create("juicebox/cache/tracks.json")?;
        file.write_all("[]".as_bytes());
    }

    Ok(())
}

pub async fn retrieve_image(url: &str) -> Result<String> {
    let image_path = format!("juicebox/cache/images/{}.jpg", scraper::sanitize_filename(str::replace(url, "https://f4.bcbits.com/img/", "")));
    if !Path::new(&image_path).exists().await {
        let r = scraper::fetch_url(url.to_string(), image_path.clone()).await;
        if r.is_err() {
            eprintln!("failed to save file {image_path}, err: {}", r.err().unwrap().to_string());
        }
    }
    Ok(image_path)
}

pub async fn retrieve_image_from_path_or(url: &str, image_path: &str) -> Result<String> {
    if !Path::new(&image_path).exists().await {
        let r = scraper::fetch_url(url.to_string(), image_path.to_string()).await;
        if r.is_err() {
            eprintln!("failed to save file {image_path}, err: {}", r.err().unwrap().to_string());
        }
    }
    Ok(image_path.to_string())
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CachedTrack {
    pub name: String,
    pub artist: String,
    pub album: String,
    pub track_url: String,
    pub download_url: String,
    pub image_path: String,
    pub track_length: i32
}

impl PartialEq for CachedTrack {
    fn eq(&self, other: &Self) -> bool {
        self.track_url == other.track_url
    }
}

pub async fn cache_song(track_info: CachedTrack) -> Result<()> {

    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open("juicebox/cache/tracks.json")?;

    let mut file_data = String::new();
    file.read_to_string(&mut file_data)?;

    let mut data: Vec<CachedTrack> = serde_json::from_str(&file_data)?;
    
    //if the cache doesnt already include the track
    if !data.contains(&track_info) {
        data.push(track_info);
        let updated_data = serde_json::to_string_pretty(&data)?;
    
        file.seek(std::io::SeekFrom::Start(0))?;
        file.set_len(0)?;
        file.write_all(updated_data.as_bytes())?;
    }
    
    Ok(())

}

pub async fn retrieve_from_cache(track_url: String) -> Result<Option<CachedTrack>> {
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open("juicebox/cache/tracks.json")?;

    let mut file_data = String::new();
    file.read_to_string(&mut file_data)?;

    let data: Vec<CachedTrack> = serde_json::from_str(&file_data)?;
    let res = data.par_iter().find_first(|ct|{
        ct.track_url == track_url
        }).map(|o| o.to_owned());
    if res.is_some() {println!("found song in cache");}
    Ok(res)
}