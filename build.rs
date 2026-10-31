//! Windows resource embedding: gives the executable an application icon so it
//! looks right on the taskbar and can be pinned with the Explorer artwork.

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/app.ico");
        // `api-ms-win...` delays: not needed; a bare icon resource only.
        if let Err(err) = res.compile() {
            panic!("failed to compile Windows resources: {err}");
        }
    }
}
