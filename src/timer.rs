

struct Timer {
    elapsed_time: Duration,
    last_measured: SystemTime,
    paused: bool
}

impl Timer {

    pub fn new() -> Timer {
        Timer {
            elapsed_time: Duration::new(0,0),
            last_measured: std::time::SystemTime::now(),
            paused: false
        }
    }

    pub fn start(&mut self) {
        self.paused = false;
    }

    pub fn get_total_elapsed(&mut self) -> Duration {

    }
}
