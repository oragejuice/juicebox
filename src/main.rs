
mod controller;
mod scraper;
mod storage;
mod stopwatch;

use std::{rc::Rc, sync::Arc, path::Path, time::Duration};

use controller::MediaControlIns;
use slint::{ModelRc, VecModel, Image};
use tokio::sync::{mpsc, Mutex};
use rodio::{OutputStream, Sink};

slint::include_modules!();


macro_rules! juiceloop {
    ($type:ty, $message:ident, $result:ident, $future:expr, $logic:block) => {
        {
            let (tx, mut rx) = mpsc::unbounded_channel::<$type>();

            tokio::spawn( async move {

                let Some(mut v) = rx.recv().await else { return };
                loop {
                tokio::select! {
                    $result = async {
                        match v.take() {
                            Some($message) => $future.await,
                            None => futures::future::pending().await
                        }
                    } => {
                        $logic
                    },

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
async fn main() -> Result<(), slint::PlatformError> {

    let _ = storage::create_folder().await;

    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&stream_handle).unwrap();
    sink.set_volume(0.5);
    let mut _controller: controller::Controller = controller::new(sink);
    let controller: Arc<Mutex<controller::Controller>> = Arc::new(Mutex::new(_controller));

    let gui = App::new().unwrap();

    
    let playback_gui_weak  = gui.as_weak();
    let playback_controller = controller.clone();
    let playback_loop: mpsc::UnboundedSender<Option<String>> = juiceloop!(
        Option<String>,
        song_url,
        results,
        scraper::get_track_info(song_url),
        {
            match results {
                Ok(info) => {
                    println!("playing song!");
                    playback_controller.lock().await.play_stream(info.file);
                    playback_controller.lock().await.stopwatch.reset();
                    let playing_data = controller::Playing {
                        name: info.name.clone(),
                        artist: info.artist.clone(),
                        total_length: Duration::from_secs(info.track_length.try_into().unwrap()),
                    };   
                    playback_controller.lock().await.track_data = Some(playing_data);   

                    let gui_copy = playback_gui_weak.clone();
                    let _ = slint::invoke_from_event_loop(move || {
                        gui_copy.unwrap().set_song_title(info.name.into());
                        let img_result = Image::load_from_path(Path::new(info.image.as_str()));
                        match img_result {
                            Ok(img) => gui_copy.unwrap().set_cover_art(img),
                            Err(e) => println!("{:?}",e)
                        }
                        
                    });             
                }
                Err(e) => println!("error on loading track {:?}", e)
            }
        }
    );

    let gui_search_weak = gui.as_weak();
    let search_loop: mpsc::UnboundedSender<Option<String>> = juiceloop!(
        Option<String>,
        query,
        results,
        scraper::search_for(query.as_str()),
        { 
            match results {
                Ok(res) => {                    
                    let gui_copy = gui_search_weak.clone();
                    let _ = slint::invoke_from_event_loop(move || {
                        let gui = gui_copy.unwrap();
                        let filtered_results: Vec<scraper::SearchResultType> = res.into_iter().flatten().collect();
                        let widgets_data: Vec<SearchData> = filtered_results.iter()
                            .map(|r| {
                                match r {
                                    scraper::SearchResultType::Artist {url,name,image, image_path } => {
                                        let img_result = Image::load_from_path(Path::new(image_path.as_str()));
                                        SearchData { artist: name.into(), name: "".into(), url: url.into(), result_type: 2, image: img_result.unwrap() }
                                    },
                                    scraper::SearchResultType::Album {url,name,artist_name,image, image_path } => {
                                        let img_result = Image::load_from_path(Path::new(image_path.as_str()));
                                        SearchData { artist: artist_name.into(), name: name.into(), url: url.into(), result_type: 1, image: img_result.unwrap() }
                                    },
                                    scraper::SearchResultType::Song {url,name,artist_name,image, image_path } => {
                                        let img_result = Image::load_from_path(Path::new(image_path.as_str()));
                                        SearchData { artist: artist_name.into(), name: name.into(), url: url.into(), result_type: 0, image: img_result.unwrap() }
                                    },
                                    scraper::SearchResultType::Label{url,name,image, image_path } => {
                                        let img_result = Image::load_from_path(Path::new(image_path.as_str()));
                                        SearchData { artist: "".into(), name: name.into(), url: url.into(), result_type: 2, image: img_result.unwrap()}
                                    },
                                }
                            }).collect();
                        let widgets_rc = ModelRc::from(Rc::new(VecModel::from(widgets_data)).clone());
                        gui.global::<SearchScreen>().set_search_results(widgets_rc);
                        gui.global::<SearchScreen>().set_is_loading(false);
                        println!("loading finished");

                    });
                },
                Err(_) => ()
            }
        }
    );

    let gui_controls_weak = gui.as_weak();
    let control_controller = controller.clone();
    let media_control_loop: mpsc::UnboundedSender<Option<MediaControlIns>> = juiceloop!(
        Option<MediaControlIns>,
        ins,
        results,
        async move {ins},
        { 
            let gui_copy = gui_controls_weak.clone();
            match results {
                MediaControlIns::Play => {
                    control_controller.lock().await.play();
                    let _ =slint::invoke_from_event_loop(move || {
                        let gui = gui_copy.unwrap();
                        gui.set_is_paused(false);
                    });
                },
                MediaControlIns::Pause => {
                    control_controller.lock().await.pause();
                    let _ = slint::invoke_from_event_loop(move || {
                        let gui = gui_copy.unwrap();
                        gui.set_is_paused(true);
                    });
                },
                MediaControlIns::TogglePausePlay => {
                    let val = control_controller.lock().await.toggle_pause_play();
                    let _ = slint::invoke_from_event_loop(move || {
                        let gui = gui_copy.unwrap();
                        gui.set_is_paused(val);
                    });
                },
                MediaControlIns::Skip => todo!(),
                MediaControlIns::Back => todo!(),
            }

        }
    );

    gui.on_play_pause({
        move || {
            let _ = media_control_loop.send(Some(MediaControlIns::TogglePausePlay));
        }
    });

    let gui_search_callback_weak = gui.as_weak();
    gui.on_searched({
        move |query| {
            let gui = gui_search_callback_weak.clone().unwrap();
            gui.global::<SearchScreen>().set_is_loading(true);
            println!("loading started");
            let _ = search_loop.send(Some(query.to_string()));

        }
    });

    let gui_loop_weak = gui.as_weak();
    let loop_controller = controller.clone();
    tokio::spawn(async move {
        let gui_copy = gui_loop_weak.clone();
        loop {
            match loop_controller.try_lock() {
                Ok(mut con) => {
                    let elapsed = con.stopwatch.get_total_elapsed().clone();
                    match &mut con.track_data {
                        Some(td) => {
                            let progress = elapsed.as_secs_f32() / td.total_length.as_secs_f32();
                            let gui = gui_copy.clone();
                            let e = slint::invoke_from_event_loop(move || {
                                gui.clone().unwrap().global::<SideBarInfo>().set_progress_bar(progress);
                            });
                            if e.is_err() {println!("{:?}",e.err())}
                        },
                        None => (),
                    }
                },
                Err(e) => println!("is locked!")
            }
            let gui = gui_copy.clone();
            let e = slint::invoke_from_event_loop(move || {
                let v = gui.clone().unwrap().global::<SearchScreen>().get_loading_ball_position();
                gui.clone().unwrap().global::<SearchScreen>().set_loading_ball_position(!v);
            });
            if e.is_err() {println!("{:?}",e.err())}

            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    });

    gui.global::<SearchScreen>().on_play_track({
        move |_, url, result_type| {
            if result_type == 0 {
                let r = playback_loop.send(Some(url.into()));
            }
        }
    });


    
    gui.run()
}