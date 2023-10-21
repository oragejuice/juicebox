
use std::fs;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;


pub async fn create_folder() -> Result<()>{
    fs::create_dir("juicebox")?;
    fs::create_dir("juicebox/cache/")?;
    Ok(())
}