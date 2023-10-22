
use crate::scraper;
use std::fs;
use async_std::path::Path;

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

    Ok(())
}

pub async fn retrieve_image(url: &str, name: &str) -> Result<String> {
    let image_path = format!("juicebox/cache/images/{}.jpg", scraper::sanitize_filename(str::replace(name, " ", "_")));
    if !Path::new(&image_path).exists().await {
        let _ =scraper::fetch_url(url.to_string(), image_path.clone()).await;
    }
    Ok(image_path)
}

pub async fn retrieve_image_from_path_or(url: &str, image_path: &str) -> Result<String> {
    if !Path::new(&image_path).exists().await {
        let _ = scraper::fetch_url(url.to_string(), image_path.to_string()).await;
    }
    Ok(image_path.to_string())
}