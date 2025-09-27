use std::fs;

pub struct AppInfo {
    pub name: String,
    pub version: String,
}

pub fn read_app_info() -> AppInfo {
    let cargo = fs::read_to_string("Cargo.toml").unwrap_or_default();
    let mut name = "RSNano".to_string();
    let mut version = "未知版本".to_string();

    for line in cargo.lines() {
        if line.starts_with("name = ") {
            name = line.split('=').nth(1).unwrap().trim().trim_matches('"').to_string();
        } else if line.starts_with("version = ") {
            version = line.split('=').nth(1).unwrap().trim().trim_matches('"').to_string();
        }
    }

    AppInfo { name, version }
}