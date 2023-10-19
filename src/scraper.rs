use regex::Regex;
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


pub async fn search_for(search: &str) -> Result<Vec<SearchResultType>> {
    let search_url = "https://bandcamp.com/search?q=".to_string() + search;
    
    let html = reqwest::get(search_url)
        .await?
        .text()
        .await?;

    let link_regex = Regex::new(r">https:\/\/(.*?)\.bandcamp\.com(.*)<\/a>").unwrap();
    let matches = link_regex.find_iter(&html).map(|s| s.as_str()).collect::<Vec<&str>>();
    let results: Vec<SearchResultType> = matches.iter()
    .map(|&s| s[1..s.len()-4].to_string() )
    .map(|s|  get_result_type(s))
    .collect();

    Ok(results)
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