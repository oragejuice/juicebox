use regex::{Regex, RegexBuilder};
use reqwest::Client;
use rodio::Decoder;
use serde_json::Value;
use stream_download::{http::HttpStream, source::SourceStream, StreamDownload, storage::temp::TempStorageProvider, Settings};
use std::io::Cursor;

use serde::{Serialize, Deserialize};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;


pub async fn get_song_decoded(url: &str) -> Result<Box<rodio::Decoder<Cursor<Vec<u8>>>>> {
    let bytes = reqwest::get(url.to_string())
            .await?
            .bytes()
            .await?
            .to_vec();

    let cursor = Cursor::new(bytes);
    let source = rodio::Decoder::new(cursor)?;

    return Ok(Box::new(source));
}


pub async fn get_bytes_from_track_url(url: &str) -> Result<Box<rodio::Decoder<Cursor<Vec<u8>>>>> {
    println!("url: {}", url);
    let download_url = get_download_url(url.to_string()).await;
    println!("download url: {:?}", download_url);
    return get_song_decoded(download_url?.unwrap().as_str()).await;
}

pub async fn get_bytes_from_track_info(info: SearchResultType) -> Result<(Box<rodio::Decoder<Cursor<Vec<u8>>>>, String, String)> {
    match info {
        SearchResultType::Song { url, name, artist_name } => {
            let download_url = get_download_url(url.to_string()).await;
            Ok((get_song_decoded(download_url?.unwrap().as_str()).await?, name, artist_name))
        }
        _ => {
            println!("not a song buddy!");
            Err("AHHHH".into())
        },

    }
    
}

pub async fn get_download_url(song_url: String) -> Result<Option<String>> {
    let html = reqwest::get(song_url)
        .await?
        .text()
        .await?;

    let link_regex = Regex::new(r";https:\/\/t4\.bcbits\.com\/stream\/.*?\;}").unwrap();
    let result = link_regex.find(&html)
                                            .map(|s: regex::Match<'_>| s.as_str()[1..s.as_str().len() - 2].to_string());

    return Ok(result);
}

//returns download_url, track image, track name, artist name and 
pub async fn get_track_info(song_url: String) -> Result<TrackInfo> {

    let start_time = std::time::SystemTime::now();

    let html = reqwest::get(song_url.clone())
        .await?
        .text()
        .await?;

    let html_time = std::time::SystemTime::now();

    let link_regex = Regex::new(r";https:\/\/t4\.bcbits\.com\/stream\/.*?\;}").unwrap();
    let download_url = link_regex.find(&html)
                                            .map(|s: regex::Match<'_>| s.as_str()[1..s.as_str().len() - 2].to_string());

    let reader = get_stream(download_url.unwrap().as_str()).await?;

    let json = get_json_for_album(&song_url.clone()).await?;
    let image = json["image"].as_str().unwrap();
    let artist = json["inAlbum"]["byArtist"]["name"].to_string();
    let album = json["inAlbum"]["name"].to_string();
    let _name = json["name"].to_string();
    let name = _name[1.._name.len()-1].to_string();
    let image_path = format!("juicebox/cache/{}.jpg", sanitize_filename(str::replace(name.as_str(), " ", "_")));

    let parsing_time = std::time::SystemTime::now();

    println!("image url: {}, image path {}, name {}", image, image_path, name.as_str());
    fetch_url(image.to_string(), image_path.clone()).await?;

    let caching_time = std::time::SystemTime::now();

    println!("html download {:?}, json parsing {:?}, caching image {:?}", html_time.duration_since(start_time),
     parsing_time.duration_since(html_time), caching_time.duration_since(parsing_time));


     Ok(TrackInfo{
        file: rodio::Decoder::new(reader)?,
        name: name,
        album: album,
        artist: artist,
        image: image_path
     })       
}

pub async fn get_album_info(album_url: &str) -> Result<(Vec<String>, String)> {
    let json = get_json_for_album(album_url).await?;
    let image = json["image"].to_string();
    let album_release = json["albumRelease"].to_string();

    let link_regex = Regex::new(r#"https://([^" ]*?).bandcamp\.com/track/([^" ]*?)"#).unwrap();
    let tracks = link_regex.find_iter(&album_release)
        .map(|m| m.as_str().to_string())
        .collect::<Vec<String>>();


    Ok((tracks, image.to_string()))

}

async fn fetch_url(url: String, file_name: String) -> Result<()> {
    let response = reqwest::get(url).await?;
    let mut file = std::fs::File::create(file_name)?;
    let mut content =  Cursor::new(response.bytes().await?);
    std::io::copy(&mut content, &mut file)?;
    Ok(())
}

fn sanitize_filename(name: String) -> String {
    let re = Regex::new(r"[#.&%/\\*!$<>{}?]").unwrap();
    re.replace_all(name.as_str(), "x").to_string()
}

pub async fn search_for(search: &str) -> Result<Vec<SearchResultType>> {
    let search_url = "https://bandcamp.com/search?q=".to_string() + search;

    let start_time = std::time::SystemTime::now();
    let html = reqwest::get(search_url)
        .await?
        .text()
        .await?;
    let html_time = std::time::SystemTime::now();

    let link_regex = Regex::new(r">https:\/\/(.*?)\.bandcamp\.com(.*)<\/a>").unwrap();
    let matches = link_regex.find_iter(&html).map(|s| s.as_str()).collect::<Vec<&str>>();
    let results: Vec<SearchResultType> = matches.iter()
    .map(|&s| s[1..s.len()-4].to_string() )
    .map(|s|  get_result_type(s))
    .collect();
    let mapping_time = std::time::SystemTime::now();
    let loading = html_time.duration_since(start_time)?;
    let regex_time = mapping_time.duration_since(html_time)?;
    println!("html loading time {:?}, regex time: {:?}, for query {search}", loading, regex_time);
    Ok(results)
}

pub async fn get_stream(download_url: &str) -> Result<StreamDownload<TempStorageProvider>>{
    let stream = HttpStream::<Client>::create(download_url.parse()?,).await?;
    println!("content length={:?}", stream.content_length());
    println!("content type={:?}", stream.content_type());

    let reader: StreamDownload<TempStorageProvider> =
    StreamDownload::from_stream(stream, TempStorageProvider::new(), Settings::default())
        .await?;

    Ok(reader)
}

pub async fn get_json_for_album(album_url: &str) -> Result<Value> {
    let html = reqwest::get(album_url)
    .await?
    .text()
    .await?;

    let json_regex: Regex  = RegexBuilder::new(r#"<script type="application/ld\+json">(.*?)</script>"#)
    .dot_matches_new_line(true)
    .build()
    .unwrap();
    let matches = json_regex.find(&html);
    let _jsonld = matches.unwrap().as_str();
    let jsonld = &_jsonld[43.._jsonld.len() - 14];
    let v: Value = serde_json::from_str(jsonld)?;
    //println!("{:?}", v);
    Ok(v)
}

pub struct TrackInfo {
    pub file: Decoder<StreamDownload<TempStorageProvider>>,
    pub name: String,
    pub album: String,
    pub artist: String,
    pub image: String
}


#[derive(Serialize, Deserialize, Debug)]
pub enum SearchResultType {
    Artist{url: String, name: String},
    Album{url: String, name: String, artist_name: String},
    Song{url: String, name: String, artist_name: String}
}

fn get_result_type(url: String) -> SearchResultType {

    let name_regex = Regex::new(r"\/([^/]*?)$").unwrap();
    let artist_regex = Regex::new(r"https:\/\/([^/]*?)\.").unwrap();
    let artist = artist_regex.find(&url).expect("Failed to get artist name").as_str();
    let artist_name = artist[8.. artist.len() - 1].to_string();

    if url.contains("/album/") {
        let name: String = name_regex.find(&url).expect("Oh no...").as_str()[1..].to_string();
        return SearchResultType::Album { url: url, name: name, artist_name: artist_name }
    } else if url.contains("/track/") {
        let name: String = name_regex.find(&url).expect("Oh no...").as_str()[1..].to_string();
        return SearchResultType::Song { url: url, name: name, artist_name: artist_name };
    } else {
        return SearchResultType::Artist { url: url, name: artist_name };
    }
}