fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        let mut res = winres::WindowsResource::new();
        // هذا السطر يربط الأيقونة icon_2.ico بالبرنامج
        res.set_icon("icon_2.ico");
        res.compile().unwrap();
    }
}