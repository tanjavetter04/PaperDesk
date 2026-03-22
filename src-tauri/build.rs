//! Downloads a tinymist release for the current `TARGET` into `resources/bin/`, so the app can ship
//! preview without requiring a separate user install. Override with `TINYMIST_SKIP_BUNDLE=1` (e.g.
//! sandboxed CI) if you provide the binary another way.

use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

/// Keep in sync with the GitHub release used for prebuilt binaries.
const TINYMIST_RELEASE_TAG: &str = "v0.14.10";

fn fetch_release_bytes(url: &str) -> Result<Vec<u8>, ureq::Error> {
    let ua = concat!(
        "PaperDesk/",
        env!("CARGO_PKG_VERSION"),
        " (tinymist release fetch)"
    );
    let agent = ureq::builder().user_agent(ua).build();
    let mut last_err = None;
    for attempt in 0..6 {
        if attempt > 0 {
            std::thread::sleep(std::time::Duration::from_secs(2));
            println!("cargo:warning=tinymist download retry {attempt}/5 for {url}…");
        }
        let mut req = agent.get(url);
        if let Ok(token) = std::env::var("GITHUB_TOKEN") {
            let token = token.trim();
            if !token.is_empty() {
                req = req.set("Authorization", &format!("Bearer {token}"));
            }
        }
        match req.call() {
            Ok(resp) => {
                let mut body = Vec::new();
                resp.into_reader().read_to_end(&mut body)?;
                return Ok(body);
            }
            Err(e) => last_err = Some(e),
        }
    }
    Err(last_err.expect("at least one attempt"))
}

fn main() {
    tauri_build::build();

    println!("cargo:rerun-if-env-changed=TINYMIST_SKIP_BUNDLE");
    println!("cargo:rerun-if-env-changed=TARGET");

    if std::env::var("TINYMIST_SKIP_BUNDLE").ok().as_deref() == Some("1") {
        println!("cargo:warning=tinymist bundle skipped (TINYMIST_SKIP_BUNDLE=1)");
        return;
    }

    let target = std::env::var("TARGET").expect("Cargo sets TARGET");
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let bin_dir = manifest_dir.join("resources/bin");
    let is_windows = target.contains("windows");
    let exe_name = if is_windows {
        "tinymist.exe"
    } else {
        "tinymist"
    };
    let dest = bin_dir.join(exe_name);
    let stamp_path = bin_dir.join(".tinymist-version");

    let stamp_ok = fs::read_to_string(&stamp_path)
        .ok()
        .map(|s| s.trim() == TINYMIST_RELEASE_TAG)
        .unwrap_or(false);

    if dest.is_file() && stamp_ok {
        return;
    }

    fs::create_dir_all(&bin_dir).expect("create resources/bin");

    // Linux returns ETXTBSY if we truncate `dest` while a tinymist process still runs from that
    // path (common during `tauri dev` rebuilds). Write a temp file and rename into place instead.
    let dest_part = bin_dir.join(if is_windows {
        ".tinymist-download.exe.part"
    } else {
        ".tinymist-download.part"
    });

    let ext = if is_windows { "zip" } else { "tar.gz" };
    let url = format!(
        "https://github.com/Myriad-Dreamin/tinymist/releases/download/{TINYMIST_RELEASE_TAG}/tinymist-{target}.{ext}"
    );

    println!("cargo:warning=fetching tinymist {TINYMIST_RELEASE_TAG} for {target}…");

    let body = fetch_release_bytes(&url).unwrap_or_else(|e| {
        panic!(
            "tinymist download failed (GET {url}: {e}).\n\
             Install tinymist on PATH, place it at {},\n\
             or set TINYMIST_SKIP_BUNDLE=1 if you vendor the binary yourself.\n\
             On GitHub Actions, ensure GITHUB_TOKEN is in the environment for this build.",
            dest.display()
        );
    });

    let _ = fs::remove_file(&dest_part);
    if is_windows {
        extract_zip(&body, &dest_part);
    } else {
        extract_tar_gz(&body, &dest_part);
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&dest_part)
            .expect("tinymist metadata")
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&dest_part, perms).expect("chmod tinymist");
    }

    #[cfg(unix)]
    fs::rename(&dest_part, &dest).unwrap_or_else(|e| {
        panic!(
            "could not install tinymist to {} ({e}).\n\
             Stop any running PaperDesk / tinymist using that binary, then rebuild.\n\
             Or set TINYMIST_SKIP_BUNDLE=1 and install tinymist on PATH.",
            dest.display()
        );
    });
    #[cfg(not(unix))]
    {
        let _ = fs::remove_file(&dest);
        fs::rename(&dest_part, &dest).unwrap_or_else(|e| {
            panic!(
                "could not install tinymist to {} ({e}).\n\
                 Close PaperDesk / tinymist if the file is locked, then rebuild.\n\
                 Or set TINYMIST_SKIP_BUNDLE=1 and install tinymist on PATH.",
                dest.display()
            );
        });
    }

    fs::write(&stamp_path, TINYMIST_RELEASE_TAG).expect("write tinymist version stamp");
}

fn extract_tar_gz(bytes: &[u8], dest: &Path) {
    let decoder = flate2::read::GzDecoder::new(bytes);
    let mut archive = tar::Archive::new(decoder);
    let mut found = false;
    for entry in archive.entries().expect("tar entries") {
        let mut entry = entry.expect("tar entry");
        let path = entry.path().expect("tar path");
        if path.file_name().and_then(|n| n.to_str()) == Some("tinymist") {
            let mut out = fs::File::create(dest).expect("create tinymist");
            std::io::copy(&mut entry, &mut out).expect("copy tinymist");
            found = true;
            break;
        }
    }
    if !found {
        panic!("tinymist tarball did not contain a `tinymist` binary");
    }
}

fn extract_zip(bytes: &[u8], dest: &Path) {
    let reader = Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(reader).expect("zip archive");
    let mut found = false;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).expect("zip index");
        if file.name() == "tinymist.exe" {
            let mut out = fs::File::create(dest).expect("create tinymist.exe");
            std::io::copy(&mut file, &mut out).expect("copy tinymist.exe");
            found = true;
            break;
        }
    }
    if !found {
        panic!("tinymist zip did not contain tinymist.exe");
    }
}
