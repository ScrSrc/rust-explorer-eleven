//! Windows shell integration: real file icons & thumbnails, "open with" and
//! the native properties dialog.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender};

pub struct IconRequest {
    pub key: u64,
    pub path: PathBuf,
    pub size: u32,
}

pub struct IconResponse {
    pub key: u64,
    pub image: Option<egui::ColorImage>,
}

/// Starts a background thread that asks the Windows shell for thumbnails and
/// file icons (`IShellItemImageFactory::GetImage`). Falls back to `None` when
/// the shell cannot produce an image, in which case the UI paints a vector icon.
pub fn spawn_icon_worker() -> (Sender<IconRequest>, Receiver<IconResponse>) {
    let (req_tx, req_rx) = std::sync::mpsc::channel::<IconRequest>();
    let (res_tx, res_rx) = std::sync::mpsc::channel::<IconResponse>();

    std::thread::Builder::new()
        .name("shell-icons".into())
        .spawn(move || {
            #[cfg(windows)]
            unsafe {
                use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
                let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            }
            while let Ok(req) = req_rx.recv() {
                #[cfg(windows)]
                let image = shell_image(&req.path, req.size);
                #[cfg(not(windows))]
                let image = None;
                if res_tx.send(IconResponse { key: req.key, image }).is_err() {
                    break;
                }
            }
            #[cfg(windows)]
            unsafe {
                use windows::Win32::System::Com::CoUninitialize;
                CoUninitialize();
            }
        })
        .expect("failed to spawn shell icon worker");

    (req_tx, res_rx)
}

#[cfg(windows)]
fn shell_image(path: &Path, size: u32) -> Option<egui::ColorImage> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::HSTRING;
    use windows::Win32::Foundation::SIZE;
    use windows::Win32::Graphics::Gdi::{
        DeleteDC, DeleteObject, GetDIBits, GetObjectW, SelectObject, BITMAP, BITMAPINFO,
        BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HGDIOBJ,
    };
    use windows::Win32::UI::Shell::{
        IShellItemImageFactory, SHCreateItemFromParsingName, SIIGBF_BIGGERSIZEOK,
        SIIGBF_RESIZETOFIT,
    };

    let wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    let hpath = HSTRING::from_wide(&wide);

    unsafe {
        let factory: IShellItemImageFactory = SHCreateItemFromParsingName(&hpath, None).ok()?;
        let requested = size.clamp(16, 512) as i32;
        let hbitmap = factory
            .GetImage(
                SIZE { cx: requested, cy: requested },
                SIIGBF_RESIZETOFIT | SIIGBF_BIGGERSIZEOK,
            )
            .ok()?;

        // Ask GDI for the real bitmap dimensions (RESIZETOFIT may keep the
        // aspect ratio, so the result is not necessarily square).
        let mut bm = BITMAP::default();
        let got = GetObjectW(
            HGDIOBJ(hbitmap.0),
            std::mem::size_of::<BITMAP>() as i32,
            Some(&mut bm as *mut _ as *mut core::ffi::c_void),
        );
        let width = bm.bmWidth;
        let height = bm.bmHeight.abs();
        if got == 0 || !(1..=2048).contains(&width) || !(1..=2048).contains(&height) {
            let _ = DeleteObject(HGDIOBJ(hbitmap.0));
            return None;
        }

        let mut info = BITMAPINFO::default();
        info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        info.bmiHeader.biWidth = width;
        info.bmiHeader.biHeight = -height; // top-down
        info.bmiHeader.biPlanes = 1;
        info.bmiHeader.biBitCount = 32;
        info.bmiHeader.biCompression = BI_RGB.0;

        let mut buf = vec![0u8; (width * height * 4) as usize];
        let dc = screen_dc();
        let old = SelectObject(dc, HGDIOBJ(hbitmap.0));
        let lines = GetDIBits(
            dc,
            hbitmap,
            0,
            height as u32,
            Some(buf.as_mut_ptr() as *mut core::ffi::c_void),
            &mut info,
            DIB_RGB_COLORS,
        );
        SelectObject(dc, old);
        let _ = DeleteObject(HGDIOBJ(hbitmap.0));
        let _ = DeleteDC(dc);
        if lines == 0 {
            return None;
        }

        // BGRA -> RGBA.
        for px in buf.chunks_exact_mut(4) {
            px.swap(0, 2);
        }
        // Some shell bitmaps come back with an empty alpha channel; treat pure
        // black as transparent in that case.
        let has_alpha = buf.chunks_exact(4).any(|px| px[3] != 0);
        if !has_alpha {
            for px in buf.chunks_exact_mut(4) {
                if px[0] < 8 && px[1] < 8 && px[2] < 8 {
                    px[3] = 0;
                } else {
                    px[3] = 255;
                }
            }
        }

        Some(egui::ColorImage::from_rgba_unmultiplied(
            [width as usize, height as usize],
            &buf,
        ))
    }
}

#[cfg(windows)]
fn screen_dc() -> windows::Win32::Graphics::Gdi::HDC {
    unsafe { windows::Win32::Graphics::Gdi::CreateCompatibleDC(None) }
}

// ---------------------------------------------------------------------------
// Shell actions (raw FFI keeps this independent of `windows` feature churn)
// ---------------------------------------------------------------------------

#[cfg(windows)]
mod ffi {
    #[link(name = "shell32")]
    extern "system" {
        pub fn ShellExecuteW(
            hwnd: *mut core::ffi::c_void,
            lpoperation: *const u16,
            lpfile: *const u16,
            lpparameters: *const u16,
            lpdirectory: *const u16,
            nshowcmd: i32,
        ) -> isize;
        pub fn SHObjectProperties(
            hwnd: *mut core::ffi::c_void,
            shopobjecttype: u32,
            pszobjectname: *const u16,
            pszpropertypage: *const u16,
        ) -> i32;
    }
}

#[cfg(windows)]
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

/// Opens a file or folder with its default application.
#[cfg(windows)]
pub fn open_path(path: &Path) -> bool {
    use std::os::windows::ffi::OsStrExt;
    let file: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let verb = wide("open");
    let result = unsafe {
        ffi::ShellExecuteW(
            std::ptr::null_mut(),
            verb.as_ptr(),
            file.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            1, // SW_SHOWNORMAL
        )
    };
    result > 32
}

#[cfg(not(windows))]
pub fn open_path(_path: &Path) -> bool {
    false
}

/// Shows the native Windows "Properties" sheet for a path.
#[cfg(windows)]
pub fn show_properties(path: &Path) {
    use std::os::windows::ffi::OsStrExt;
    let file: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    const SHOP_FILEPATH: u32 = 2;
    unsafe {
        ffi::SHObjectProperties(std::ptr::null_mut(), SHOP_FILEPATH, file.as_ptr(), std::ptr::null());
    }
}

#[cfg(not(windows))]
pub fn show_properties(_path: &Path) {}

/// Opens the shell t("Открыть с помощью") chooser for a file.
#[cfg(windows)]
pub fn open_with(path: &Path) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let mut cmd = std::process::Command::new("rundll32.exe");
    cmd.arg("shell32.dll,OpenAs_RunDLL")
        .arg(path)
        .creation_flags(CREATE_NO_WINDOW);
    let _ = cmd.spawn();
}

#[cfg(not(windows))]
pub fn open_with(_path: &Path) {}

/// Opens Windows Terminal (or a PowerShell window as a fallback) in a folder.
#[cfg(windows)]
pub fn open_terminal(path: &Path) {
    let mut wt = std::process::Command::new("wt.exe");
    wt.arg("-d").arg(path);
    if wt.spawn().is_ok() {
        return;
    }
    let escaped = path.display().to_string().replace('\'', "''");
    let mut ps = std::process::Command::new("powershell.exe");
    ps.arg("-NoExit")
        .arg("-Command")
        .arg(format!("Set-Location -LiteralPath '{escaped}'"));
    let _ = ps.spawn();
}

#[cfg(not(windows))]
pub fn open_terminal(_path: &Path) {}
