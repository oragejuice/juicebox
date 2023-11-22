
mod controller;
mod scraper;
mod storage;
mod stopwatch;

use std::{rc::Rc, sync::{Arc, atomic::AtomicBool}, path::Path, time::Duration};

use controller::MediaControlIns;
use slint::{ModelRc, VecModel, Image, SharedString};
use tokio::sync::{mpsc, Mutex};
use rodio::{OutputStream, Sink};

slint::include_modules!();

// This starts a task that runs forever that we can send messages to do specific tasks, offloading all the computions from the gui thread
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

    let is_loading_song: AtomicBool = AtomicBool::new(false);


    //let is_loading_song_playback = is_loading_song.clone();
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
                    playback_controller.lock().await.play_stream(info.file);
                    playback_controller.lock().await.stopwatch.reset();
                    
                    let playing_data = controller::Playing {
                        name: info.name.clone(),
                        artist: info.artist.clone(),
                        total_length: Duration::from_secs(info.track_length.try_into().unwrap()),
                    };   
                    playback_controller.lock().await.track_data = Some(playing_data);   


                    let queue_tracks: Vec<String> = playback_controller.lock().await.queue.iter().map(|(name, url)| name.to_owned()).collect();

                    let gui_copy = playback_gui_weak.clone();
                    let _ = slint::invoke_from_event_loop(move || {
                        gui_copy.unwrap().set_song_title(info.name.into());
                        let img_result = Image::load_from_path(Path::new(info.image.as_str()));
                        match img_result {
                            Ok(img) => gui_copy.unwrap().set_cover_art(img),
                            Err(e) => println!("{:?}",e)
                        }
                        let queue_model = ModelRc::from(Rc::new(VecModel::from(queue_tracks.iter().map(|s| SharedString::from(s)).collect::<Vec<SharedString>>())).clone());
                        gui_copy.unwrap().global::<SideBarInfo>().set_queue(queue_model);
                        
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
                        gui.global::<SearchScreen>().set_showing_album(false);
                        let filtered_results: Vec<scraper::SearchResultType> = res.into_iter().flatten().collect();
                        let widgets_data: Vec<SearchData> = filtered_results.iter()
                            .map(|r| {
                                match r {
                                    scraper::SearchResultType::Artist {url, name, image: _, image_path } => {
                                        let img_result = Image::load_from_path(Path::new(image_path.as_str()));
                                        SearchData { artist: "Artist".into(), name: name.into(), url: url.into(), result_type: 2, image: img_result.unwrap() }
                                    },
                                    scraper::SearchResultType::Album {url,name,artist_name,image: _, image_path } => {
                                        let img_result = Image::load_from_path(Path::new(image_path.as_str()));
                                        SearchData { artist: artist_name.into(), name: name.into(), url: url.into(), result_type: 1, image: img_result.unwrap() }
                                    },
                                    scraper::SearchResultType::Song {url,name,artist_name,image: _, image_path } => {
                                        let img_result = Image::load_from_path(Path::new(image_path.as_str()));
                                        SearchData { artist: artist_name.into(), name: name.into(), url: url.into(), result_type: 0, image: img_result.unwrap() }
                                    },
                                    scraper::SearchResultType::Label{url,name, image: _, image_path } => {
                                        let img_result = Image::load_from_path(Path::new(image_path.as_str()));
                                        SearchData { artist: "Label".into(), name: name.into(), url: url.into(), result_type: 2, image: img_result.unwrap()}
                                    },
                                }
                            }).collect();
                        let widgets_rc = ModelRc::from(Rc::new(VecModel::from(widgets_data)).clone());
                        gui.global::<SearchScreen>().set_search_results(widgets_rc);
                        gui.global::<SearchScreen>().set_is_loading(false);

                    });
                },
                Err(_) => ()
            }
        }
    );

    let album_gui_weak = gui.as_weak();
    let album_loop: mpsc::UnboundedSender<Option<String>> = juiceloop!(
        Option<String>,
        query,
        results,
        scraper::get_album_info(query.as_str()),
        {
            match results {
                Ok(res) => {
                    let (tracks, image, artist, album) = res;
                    let results: Vec<AlbumTrackData> = tracks.iter()
                        .map(|(url, name, duration)| {
                            let seconds = duration.as_secs();
                            let minutes = seconds / 60;
                            let remaining_seconds = seconds % 60;

                            AlbumTrackData{
                                name: name.into(),
                                url: url.into(),
                                time: format!("{:02}:{:02}", minutes, remaining_seconds).into()
                            }
                        })
                        .collect::<Vec<AlbumTrackData>>();


                    let gui_copy = album_gui_weak.clone();
                    let _ = slint::invoke_from_event_loop(move || {
                        let gui = gui_copy.unwrap();
                        let results_rc = ModelRc::from(Rc::new(VecModel::from(results)).clone());
                        let img_result = Image::load_from_path(Path::new(image.as_str()));
                        match img_result {
                            Ok(img) => {
                                gui.global::<SearchScreen>().set_showing_album(true);
                                gui.global::<AlbumScreen>().set_album_tracks(results_rc);
                                gui.global::<AlbumScreen>().set_album_name(album.into());
                                gui.global::<AlbumScreen>().set_album_artist(artist.into());
                                gui.global::<AlbumScreen>().set_album_art(img);
                            },
                            Err(e) => {println!("{:?}", e)}
                        }


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
            let _ = search_loop.send(Some(query.to_string()));

        }
    });

    // This tasks is "special" in that rather than waiting for an instruction we just run it every 500ms
    let gui_loop_weak = gui.as_weak();
    let loop_controller = controller.clone();
    tokio::spawn(async move {
        loop {
            match loop_controller.try_lock() {
                Ok(mut con) => {

                    //if we have finished playing a song, and a song is in the queue
                    //GAWD DAYUM is this code messy asf (i deleted it all now xD)
                    if con.sink.empty() && !con.queue.is_empty() {
                        let (name, url) = con.queue.pop_front().unwrap();
                        let gui_copy = gui_loop_weak.clone();
                        //is_loading_song = true;
                        //I should probably move playback_loop into an arc so i can access it here directly, but this hack will work for now
                        let _ = slint::invoke_from_event_loop(move || {
                            let gui = gui_copy.unwrap();
                            //0 = track
                            gui.global::<SearchScreen>().invoke_play_track(name.clone().into(), url.into(), 0);
                            println!("played next song in queue {}", name);
                            //dbg!(&con.queue);
                        });
                        dbg!(&con.queue);
                    }

                    //update the progress bar on the song
                    let elapsed = con.stopwatch.get_total_elapsed().clone();
                    match &mut con.track_data {
                        Some(td) => {
                            let gui_copy = gui_loop_weak.clone();
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
                Err(_) => println!("controller is locked")
            }
            let gui_copy = gui_loop_weak.clone();
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

            match result_type {
                0 => {let _ = playback_loop.send(Some(url.into()));},
                1 => {let _ = album_loop.send(Some(url.into()));},
                _ => ()
            }
        }
    });

    let queue_controller = controller.clone();
    let gui_queue_button_weak = gui.as_weak();
    gui.global::<SearchScreen>().on_add_to_queue(move |name, _url| {
        let controller = queue_controller.clone();
        let url = _url.to_string();
        let gui_copy = gui_queue_button_weak.clone();
        // so like maybe at some point i should reduce delay by using the track-info directly,to be fair this way is much much easier.
        // by using the side-effect of caching songs we essentally pre-do all the html requests when adding a song to queue,
        // thus reducing delay between playing songs in the queue
        tokio::spawn(async move {
            let track_info = scraper::get_track_info(url.clone().into()).await;
            match track_info {
                Ok(_) => {
                    controller.lock().await.queue.push_back((name.into(), url.into()));
                    let queue_tracks: Vec<String> = controller.lock().await.queue.iter().map(|(name, url)| name.to_owned()).collect();
                    let _ = slint::invoke_from_event_loop(move || {
                        let queue_model = ModelRc::from(Rc::new(VecModel::from(queue_tracks.iter().map(|s| SharedString::from(s)).collect::<Vec<SharedString>>())).clone());
                        gui_copy.unwrap().global::<SideBarInfo>().set_queue(queue_model);
                    });
                },
                Err(e) => println!("Failed to load song to add to queue"),
            }
        });
    });


    
    gui.run()
}