use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::sync::Mutex;

// 全域 log 檔案鎖，避免多執行緒寫入衝突
lazy_static::lazy_static! {
    static ref LOG_MUTEX: Mutex<()> = Mutex::new(());
}

/// 將訊息寫入 log 檔案 (logs/emulator.log)
pub fn log_to_file(msg: &str) {
    let _lock = LOG_MUTEX.lock().unwrap();
    let log_dir = "logs";
    let log_path = format!("{}/emulator.log", log_dir);
    if !Path::new(log_dir).exists() {
        let _ = std::fs::create_dir_all(log_dir);
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .unwrap();
    let _ = writeln!(file, "{}", msg);
}
