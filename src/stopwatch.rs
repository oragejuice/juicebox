

struct StopWatch {
    elapsed_time: Duration,
    last_measured: SystemTime,
    paused: bool
}

impl StopWatch {

    pub fn new() -> Timer {
        Timer {
            elapsed_time: Duration::new(0,0),
            last_measured: std::time::SystemTime::now(),
            paused: false
        }
    }

    pub fn start(&mut self) {
        self.paused = false;
        self.last_measured = std::time::SystemTime::now();
    }

    pub fn get_total_elapsed(&mut self) -> Duration {
        if self.paused {
            return self.elapsed_time;
        } else {
            let ret = self.elapsed_time + last_measured.elapsed_time();
            self.last_measured = std::time::SystemTime::now();
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
