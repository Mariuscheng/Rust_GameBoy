use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

/// 將訊息寫入 log 檔案 (logs/emulator.log)
pub fn log_to_file(msg: &str) {
    let log_dir = "logs";
    let log_path = format!("{}/emulator.log", log_dir);
    if !Path::new(log_dir).exists() {
        let _ = std::fs::create_dir_all(log_dir);
    }
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&log_path) {
        let _ = writeln!(file, "{}", msg);
    }
}
