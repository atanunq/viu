use std::env;

pub fn truecolor_available() -> bool {
    if let Ok(value) = env::var("COLORTERM") {
        if value.contains("truecolor") || value.contains("24bit") {
            return true;
        }
    }
    false
}
