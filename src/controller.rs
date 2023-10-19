
use std::io::Cursor;

use rodio::{Decoder, Sink};

pub struct Controller {
    is_paused: bool,
    pub sink: Sink,
    queue: Vec<String>
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
        &self.sink.pause();
        self.is_paused = true;
    }

    pub fn play(&mut self) {
        &self.sink.play();
        self.is_paused = false;
    }

    pub fn play_file(&mut self, source: Decoder<Cursor<Vec<u8>>>) {
        self.sink.clear();
        let _ = &self.sink.append(source);
        self.play();
    }
}


pub fn new(sink: Sink) -> Controller{
    
    Controller {
        is_paused: true,
        sink: sink,
        queue: Vec::new()
    }
}