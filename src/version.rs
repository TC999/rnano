pub struct AppInfo {
    pub name: &'static str,
    pub version: &'static str,
}

pub fn read_app_info() -> AppInfo {
    AppInfo {
        name: env!("CARGO_PKG_NAME"),
        version: env!("CARGO_PKG_VERSION"),
    }
}