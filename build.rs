// build.rs
fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        // تضمين ملف الـ Manifest لضمان عمل واجهات ويندوز الحديثة بدون أخطاء
        println!("cargo:rustc-link-lib=dylib=comctl32");
    }
}
