
use std::sync::Mutex;

struct Controller {
    paused: bool,
    sink: rodio::Sink
}

pub async fn testing(v: Mutex<Vec<String>>) {
    v.lock().unwrap().push("1".to_string());
    println!("v {:?}", v);
}