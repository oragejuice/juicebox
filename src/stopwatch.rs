use std::time::{Duration, SystemTime};



pub struct StopWatch {
    elapsed_time: Duration,
    last_measured: SystemTime,
    paused: bool
}

impl StopWatch {

    pub fn new() -> StopWatch {
        StopWatch {
            elapsed_time: Duration::new(0,0),
            last_measured: SystemTime::now(),
            paused: false
        }
    }

    pub fn start(&mut self) {
        self.paused = false;
        self.last_measured = SystemTime::now();
    }

    pub fn get_total_elapsed(&mut self) -> Duration {
        if self.paused {
            return self.elapsed_time;
        } else {
            let ret = self.elapsed_time + self.last_measured.elapsed().unwrap();
            self.last_measured = SystemTime::now();
            return ret;
        }
    }

    pub fn pause(&mut self) {
        self.paused = true;
    }

    pub fn is_paused(&self) -> bool {
        return self.paused;
    }
}
