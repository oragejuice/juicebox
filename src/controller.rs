
use std::io::Error;
use rodio::{Decoder, Sink, Source};
use stream_download::{StreamDownload, storage::temp::TempStorageProvider};

use crate::{stopwatch::{StopWatch, self}, scraper::TrackInfo};


pub struct Controller {
    is_paused: bool,
    pub sink: Sink,
    pub stopwatch: StopWatch,
    pub track_data: Option<TrackInfo>
}

impl Controller {

    pub fn toggle_pause_play(&mut self) -> bool {
        let currently_paused = self.sink.is_paused();
        if currently_paused {
            self.play();
            return false;
        } else  {
            self.pause();
            return true;
        }
    }

    pub fn pause(&mut self) {
        let _ = &self.sink.pause();
        self.is_paused = true;
        self.stopwatch.pause();
    }

    pub fn play(&mut self) {
        let _ = &self.sink.play();
        self.is_paused = false;
        self.stopwatch.start();
    }

    pub fn is_paused(&self) -> bool {
        return self.is_paused
    }

    pub fn play_stream(&mut self, source: Decoder<StreamDownload<TempStorageProvider>>) {
        self.sink.clear();
        println!("song length is: {:?}", source.total_duration());
        let _ = &self.sink.append(source);
        self.play();
    }

    pub fn handle_control_inst(&mut self, instruction: MediaControlIns) -> Result<MediaControlIns, Error> {
        match instruction {
            MediaControlIns::Play => {let _ = self.play();},
            MediaControlIns::Pause =>{let _ = self.pause();},
            MediaControlIns::TogglePausePlay => {let _ = self.toggle_pause_play();},
            MediaControlIns::Skip => todo!(),
            MediaControlIns::Back => todo!(),
        }

        Ok(instruction)
    }
}


pub fn new(sink: Sink) -> Controller{
    
    Controller {
        is_paused: true,
        sink: sink,
        stopwatch: StopWatch::new(),
        track_data: None
}
}

pub enum MediaControlIns {
    Play,
    Pause,
    TogglePausePlay,
    Skip,
    Back,
}