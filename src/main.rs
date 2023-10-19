
mod controller;
mod scraper;

use std::sync::Arc;

use tokio::sync::{mpsc, Mutex};
use rodio::{OutputStream, Sink};

slint::include_modules!();


macro_rules! juiceloop {
    ($message:ident, $result:ident, $future:expr, $logic:block) => {
        {
            let (tx, mut rx) = mpsc::unbounded_channel::<Option<String>>();

            tokio::spawn( async move {

                let Some(mut v) = rx.recv().await else { return };
                loop {
                tokio::select! {
                    $result = async {
                        match v.take() {
                            Some($message) => $future.await,
                            None => futures::future::pending().await
                        }
                    } => $logic,

                    r = rx.recv() => match r {
                        Some(nv) => {
                            v = nv;
                        },
                        None => return,
                    },
                }
                }
            });

            tx
        }
    };
}



#[tokio::main]
async fn main() -> Result<(), slint::PlatformError>{

    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&stream_handle).unwrap();
    let mut controller: controller::Controller = controller::new(sink);


    let gui = App::new().unwrap();
    let gui_weak = gui.as_weak();

    let playback_loop: mpsc::UnboundedSender<Option<String>> = juiceloop!(
        download_url,
        results,
        scraper::get_song_decoded(download_url.as_str()),
        {
            match results {
                Ok(file) => {
                    controller.play_file(*file);
                    let gui_copy = gui_weak.clone();
                    let _ = slint::invoke_from_event_loop(move || {
                        let gui = gui_copy.unwrap();
                        let _ = gui.set_name("currently playing".into());
                    });
                }
                Err(_) => ()
            }
        }
    );

    let search_loop: mpsc::UnboundedSender<Option<String>> = juiceloop!(
        query,
        results,
        scraper::search_for(query.as_str()),
        { 
            match results {
                Ok(res) => {


                    println!("results.. \n {:?}", res);
                },
                Err(_) => ()
            }
        }
    );



    gui.on_play_pause({
        move || {
            let url = "https://t4.bcbits.com/stream/e997186749c3dc5cea496ff7b4405695/mp3-128/2261903040?p=0&ts=1697813995&t=6fd3039d1af75cced2f5836d77627b68c10b3e52&token=1697813995_6ef94c7ad323a96a233befdeda405e2a19d0ba97".to_string();
            let _ = playback_loop.send(Some(url));
        }
    });

    gui.on_searched({
        move |query| {
            println!{"searched for: {}", query}
            let _ = search_loop.send(Some(query.to_string()));

        }
    });

    
    gui.run()
}