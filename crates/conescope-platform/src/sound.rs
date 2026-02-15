use std::process::Command;

#[cfg(target_os = "macos")]
pub fn play_system_sound(name: &str) {
    let path = format!("/System/Library/Sounds/{name}.aiff");
    std::thread::spawn(move || {
        let _ = Command::new("afplay").arg(&path).output();
    });
}

#[cfg(not(target_os = "macos"))]
pub fn play_system_sound(_name: &str) {}

pub fn play_status_sound(status: &str) {
    match status {
        "question" => play_system_sound("Purr"),
        "waiting" => play_system_sound("Tink"),
        "finished" => play_system_sound("Pop"),
        "stopped" => play_system_sound("Basso"),
        _ => {}
    }
}
