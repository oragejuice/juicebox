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
        SearchResultType::Song { url, name, artist_name, image, image_path } => {
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

    let json = parse_for_album(html.as_str()).await?;
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

    println!("html download {:?}, json parsing {:?}, caching image {:?}",
     html_time.duration_since(start_time),parsing_time.duration_since(html_time), caching_time.duration_since(parsing_time));


     Ok(TrackInfo{
        file: rodio::Decoder::new(reader)?,
        name: name,
        album: album,
        artist: artist,
        image: image_path
     })       
}

pub async fn get_album_info(album_url: &str) -> Result<(Vec<String>, String)> {
    let html = reqwest::get(album_url)
        .await?
        .text()
        .await?;

    let json = parse_for_album(&html).await?;
    let image = json["image"].to_string();
    let album_release = json["albumRelease"].to_string();

    let link_regex = Regex::new(r#"https://([^" ]*?).bandcamp\.com/track/([^" ]*?)"#).unwrap();
    let tracks = link_regex.find_iter(&album_release)
        .map(|m| m.as_str().to_string())
        .collect::<Vec<String>>();


    Ok((tracks, image.to_string()))

}

pub async fn fetch_url(url: String, file_name: String) -> Result<()> {
    let response = reqwest::get(url).await?;
    let mut file = std::fs::File::create(file_name)?;
    let mut content =  Cursor::new(response.bytes().await?);
    std::io::copy(&mut content, &mut file)?;
    Ok(())
}

pub fn sanitize_filename(name: String) -> String {
    let re = Regex::new(r"[#.&%/\\*!$<>{}?| ]").unwrap();
    re.replace_all(name.as_str(), "_").to_string()
}

pub fn trim_whitespace(s: &str) -> String {
    let mut new_str = s.trim().to_owned();
    let mut prev = ' '; // The initial value doesn't really matter
    new_str.retain(|ch| {
        let result = ch != ' ' || prev != ' ';
        prev = ch;
        result
    });
    let re = Regex::new(r"(\\n|\\t)").unwrap();
    re.replace_all(&new_str, " ").to_string()
}

pub async fn search_for(query: &str) -> Result<Vec<Option<SearchResultType>>> {
    let search_url = "https://bandcamp.com/search?q=".to_string() + query;
    let start_time = std::time::SystemTime::now();
    let html = reqwest::get(search_url)
        .await?
        .text()
        .await?
        .to_string();
    let html_download_time = std::time::SystemTime::now();

    let item_search = item_type_search(&html);
    let title_and_links = title_and_link_search(&html);
    let subheads = subhead_search(&html);
    let cover_art = cover_art_search(&html);

    let (items,  titles_links,  subheads, images) 
        = futures::join!(item_search, title_and_links, subheads, cover_art);

    let async_regex_time = std::time::SystemTime::now();

    let search_results = items.iter().zip(subheads.iter().zip(images.iter().zip(titles_links.iter())));

    let zipping_time = std::time::SystemTime::now();

    let mut ret: Vec<Option<SearchResultType>> = vec![];
    for res in search_results {
        let (item, (subhead,(image, (link, title)))) = res;
        let file_path = format!("juicebox/cache/{}.jpg", sanitize_filename(title.clone().unwrap().to_string()));
        ret.push(get_result_type(link, title, image, subhead, item, &Some(file_path)));
    }

    let image_futures = ret.iter().map(|search_result| {
        match search_result {
            Some(res) => {
                match res {
                    SearchResultType::Artist { url, name, image, image_path } => {
                        Some(fetch_url(image.clone(), image_path.to_string()))
                    },
                    SearchResultType::Album { url, name, artist_name, image, image_path } => {
                        Some(fetch_url(image.clone(), image_path.to_string()))
                    },
                    SearchResultType::Label { url, name, image, image_path } => {
                        Some(fetch_url(image.clone(), image_path.to_string()))
                    },
                    SearchResultType::Song { url: _, name, artist_name, image, image_path } => {
                        Some(fetch_url(image.clone(), image_path.to_string()))
                    },
                }
            },
            None => None,
        }
    })
    .flatten()
    .collect::<Vec<_>>();



    futures::future::join_all(image_futures).await;

    //let ret: Vec<Option<SearchResultType>> = search_results;

    let result_type_time = std::time::SystemTime::now();
    let html_delay = html_download_time.duration_since(start_time).unwrap();
    let regex_delay = async_regex_time.duration_since(html_download_time).unwrap();
    let zipping_delay = zipping_time.duration_since(async_regex_time).unwrap();
    let result_delay = result_type_time.duration_since(zipping_time).unwrap();
    println!("html {:?}, regex: {:?}, zipping: {:?}, result: {:?}", html_delay, regex_delay, zipping_delay, result_delay);

    Ok(ret)
}
/*
pub async fn search_for(search: &str) -> Result<Vec<SearchResultType>> {
    let search_url = "https://bandcamp.com/search?q=".to_string() + search;

    let start_time = std::time::SystemTime::now();
    let html = reqwest::get(search_url)
        .await?
        .text()
        .await?;
    let html_time = std::time::SystemTime::now();

    let link_regex = Regex::new(r">https:\/\/(.*?)\.bandcamp\.com(.*)<\/a>").unwrap();
    let mut matches = link_regex.find_iter(&html).map(|s| s.as_str()).collect::<Vec<&str>>();
    let regex_time = std::time::SystemTime::now();

    let results: Vec<SearchResultType> = matches.par_iter_mut()
        .map(|s| s[1..s.len()-4].to_string())
        .map(|s| get_result_type(s))
        .collect();

    let mapping_time = std::time::SystemTime::now();

    let loading = html_time.duration_since(start_time)?;
    let doing_regex = regex_time.duration_since(html_time)?;
    let mappings_time = mapping_time.duration_since(regex_time);
    let total_time = mapping_time.duration_since(start_time);
    println!("html loading time {:?}, regex time: {:?}, mapping time {:?}, total {:?} for query {search}", loading, doing_regex, mappings_time, total_time);

    Ok(results)
}
*/

async fn item_type_search(html: &String) -> Vec<Option<String>> {
    
    let item_type_pattern = RegexBuilder::new(r#"<div class="itemtype">(.*?)</div>"#)
        .dot_matches_new_line(true)
        .build()
        .unwrap();
    let matches = item_type_pattern.captures_iter(&html)
        .map(|capture| capture.get(1))
        .map(|s| {
            match s {
                Some(m) => Some(m.as_str().trim().to_string()),
                None => None 
            }
        }).collect::<Vec<Option<String>>>();
    return matches;
}

async fn title_and_link_search(html: &String) -> Vec<(Option<String>, Option<String>)> {
    
    let item_type_pattern = RegexBuilder::new(r#"<div class="heading">(.*?)<a href="(.*?)">(.*?)</a>(.*?)</div>"#)
        .dot_matches_new_line(true)
        .build()
        .unwrap();
    let matches = item_type_pattern.captures_iter(&html)
        .map(|capture| (capture.get(2), capture.get(3)))
        .map(|s| {
            let (_links, _titles) = s;
            let links = match _links {
                Some(m) => Some(m.as_str().trim().to_string()),
                None => None 
            };
            let titles: Option<String> = match _titles {
                Some(m) => Some(m.as_str().trim().to_string()),
                None => None 
            };
            (links, titles)
        }).collect::<Vec<(Option<String>, Option<String>)>>();
    return matches;
}

async fn subhead_search(html: &String) -> Vec<Option<String>> {
    
    let item_type_pattern = RegexBuilder::new(r#"<div class="subhead">(.*?)</div>"#)
        .dot_matches_new_line(true)
        .build()
        .unwrap();
    let matches = item_type_pattern.captures_iter(&html)
        .map(|capture| capture.get(1))
        .map(|s| {
            match s {
                Some(m) => Some(trim_whitespace(m.as_str())),
                None => None 
            }
        }).collect::<Vec<Option<String>>>();
    return matches;
}

async fn cover_art_search(html: &String) -> Vec<Option<String>> {
    
    let item_type_pattern = RegexBuilder::new(r#"<div class="art">(.*?)<img src="(.*?)">(.*?)</div>"#)
        .dot_matches_new_line(true)
        .build()
        .unwrap();
    let matches = item_type_pattern.captures_iter(&html)
        .map(|capture| capture.get(2))
        .map(|s| {
            match s {
                Some(m) => Some(m.as_str().trim().to_string()),
                None => None 
            }
        }).collect::<Vec<Option<String>>>();
    return matches;
}

pub async fn get_stream(download_url: &str) -> Result<StreamDownload<TempStorageProvider>>{
    let stream = HttpStream::<Client>::create(download_url.parse()?,).await?;

    let reader: StreamDownload<TempStorageProvider> =
    StreamDownload::from_stream(stream, TempStorageProvider::new(), Settings::default())
        .await?;

    Ok(reader)
}

pub async fn parse_for_album(html: &str) -> Result<Value> {

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
    Artist{url: String, name: String, image: String, image_path: String},
    Album{url: String, name: String, artist_name: String, image: String, image_path: String},
    Label{url: String, name: String, image: String, image_path: String},
    Song{url: String, name: String, artist_name: String, image: String, image_path: String}
}


fn get_result_type(
    _url: &Option<String>,
    _name: &Option<String>,
    _image: &Option<String>, 
    subheading: &Option<String>, 
    item: &Option<String>,
    _image_path: &Option<String>,
    ) -> Option<SearchResultType> {
    let i = (*item).clone()?;
    let url = (*_url).clone()?;
    let name = (*_name).clone()?;
    let image = (*_image).clone()?;
    let image_path = (*_image_path).clone()?;

    let result = match i.as_str() {
        "ARTIST" => Some(SearchResultType::Artist { url: url, name: name, image: image, image_path: image_path }),
        "ALBUM" => Some(SearchResultType::Album { url: url, name: name, artist_name: (*subheading).clone()?, image: image, image_path: image_path }),
        "TRACK" => Some(SearchResultType::Song { url: url, name: name, artist_name: (*subheading).clone()?, image: image, image_path: image_path }),
        "LABEL" => Some(SearchResultType::Label { url: url, name: name, image: image,image_path: image_path }),
        _ => None
    };
    return result;

}