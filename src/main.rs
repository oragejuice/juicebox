
mod controller;
mod scraper;

use std::sync::{Arc};
use tokio::sync::Mutex;
use tokio::sync::watch;

slint::include_modules!();


#[tokio::main]
async fn main() -> Result<(), slint::PlatformError>{
    let gui = App::new().unwrap();

    //let mut t: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::<Vec<String>>::new(vec![]));

    let (tx, mut rx) = watch::channel(String::from("initial search"));

    let atx = Arc::new(Mutex::new(tx));

    tokio::spawn( async move {
        rx.changed();
        loop {
            let query = rx.borrow_and_update().to_string();
            tokio::select! {
                results = scraper::search_for(query) => {
                    match results {
                        Ok(res) => println!("results.."),
                        Err(_) => ()
                    }
                    rx.changed().await;
                }
                v = rx.changed() => {
                    if v.is_err() {
                        return;
                    }
                }
            }
        }
    });


    gui.on_button_clicked({
        move || {
            let tx: Arc<Mutex<watch::Sender<String>>> = atx.clone();
            tokio::spawn(async move {
                tx.lock().await.send("goretrance".to_string());
            });
        }
    });

    
    gui.run()
}