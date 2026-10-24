//! Sends photos to an Android/Google TV for the screensaver and My Photos (`tools/photos.py`).
//!
//! In the order given (a folder's in name order), to the app's own folder on the TV, where adb can write; where the
//! TV keeps adb out of it (Android 11, Fire OS 8), through the app itself (`content write` to its PhotoInbox).

use std::fs;
use std::path::Path;
use std::process::{Command, Output, Stdio};

pub const INBOX: &str = "/sdcard/Android/data/io.freemytv/files/photos";
const KINDS: [&str; 6] = [".jpg", ".jpeg", ".png", ".webp", ".heic", ".heif"];

fn is_photo(name: &str) -> bool {
    let lower = name.to_lowercase();
    KINDS.iter().any(|k| lower.ends_with(k))
}

fn sorted_entries(dir: &Path) -> (Vec<String>, Vec<String>) {
    let (mut dirs, mut files) = (Vec::new(), Vec::new());
    if let Ok(rd) = fs::read_dir(dir) {
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if e.path().is_dir() {
                dirs.push(name);
            } else {
                files.push(name);
            }
        }
    }
    dirs.sort();
    files.sort();
    (dirs, files)
}

/// os.walk, top-down: a folder's photos, then each subfolder's; hidden ones skipped. `base` is the folder as
/// given without its trailing slash, so the paths read like Python's `os.path.join(root, f)`.
fn walk(dir: &Path, base: &str, out: &mut Vec<String>) {
    let (dirs, files) = sorted_entries(dir);
    for f in files.iter().filter(|f| !f.starts_with('.') && is_photo(f)) {
        out.push(format!("{base}/{f}"));
    }
    for d in dirs.iter().filter(|d| !d.starts_with('.')) {
        walk(&dir.join(d), &format!("{base}/{d}"), out);
    }
}

/// The photos to send, and a line for each argument that isn't one (`photos`).
pub fn photos(paths: &[String]) -> (Vec<String>, Vec<String>) {
    let (mut out, mut skipped) = (Vec::new(), Vec::new());
    for p in paths {
        let path = Path::new(p);
        if path.is_dir() {
            walk(path, p.trim_end_matches('/'), &mut out);
        } else if path.is_file() && is_photo(p) {
            out.push(p.clone());
        } else {
            skipped.push(format!("Skipped {p}: not a photo or a folder."));
        }
    }
    (out, skipped)
}

/// The name on the TV: numbered, so the app imports them in this order, and without spaces.
pub fn remote_name(i: usize, path: &str) -> String {
    let base = path.rsplit('/').next().unwrap_or(path);
    format!("{i:03}-{}", base.replace(' ', "_"))
}

/// `urllib.parse.quote`: letters, digits, `_.-~` and `/` stay; every other byte is %XX.
pub fn quote(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"_.-~/".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// adb, aimed at one TV once connected, so a phone or another TV on adb doesn't get in the way.
pub struct Adb {
    pub program: String,
    pub serial: Vec<String>,
}

impl Adb {
    pub fn run(&self, args: &[&str]) -> Output {
        self.run_with(args, None)
    }

    pub fn run_with(&self, args: &[&str], stdin: Option<fs::File>) -> Output {
        let mut c = Command::new(&self.program);
        c.args(&self.serial).args(args);
        c.stdin(stdin.map(Stdio::from).unwrap_or_else(Stdio::null));
        c.output().unwrap_or_else(|e| Output {
            status: failed(),
            stdout: Vec::new(),
            stderr: e.to_string().into_bytes(),
        })
    }
}

#[cfg(unix)]
fn failed() -> std::process::ExitStatus {
    use std::os::unix::process::ExitStatusExt;
    std::process::ExitStatus::from_raw(1 << 8)
}

#[cfg(windows)]
fn failed() -> std::process::ExitStatus {
    use std::os::windows::process::ExitStatusExt;
    std::process::ExitStatus::from_raw(1)
}

fn text(o: &Output) -> (String, String) {
    (
        String::from_utf8_lossy(&o.stdout).into_owned(),
        String::from_utf8_lossy(&o.stderr).into_owned(),
    )
}

/// The whole send, printing as the Python prints; returns the exit code.
pub fn send(adb: &mut Adb, mut args: Vec<String>) -> i32 {
    if !args.is_empty()
        && !Path::new(&args[0]).exists()
        && args[0].starts_with(|c: char| c.is_ascii_digit())
    {
        let ip = args.remove(0);
        let target = if ip.contains(':') {
            ip
        } else {
            format!("{ip}:5555")
        };
        let (out, err) = text(&adb.run(&["connect", &target]));
        if !out.contains("connected") {
            println!("Could not connect to {target}: {}", (out + &err).trim());
            return 1;
        }
        adb.serial = vec!["-s".into(), target];
    }
    let (found, skipped) = photos(&args);
    for s in skipped {
        println!("{s}");
    }
    if found.is_empty() {
        println!("No photos to send.");
        return 1;
    }
    let listed = adb.run(&["shell", "pm", "list", "packages", "io.freemytv"]);
    if !listed.status.success() {
        println!(
            "No TV connected: give its IP address first (adb devices lists the connected ones)."
        );
        return 1;
    }
    if !text(&listed)
        .0
        .split_whitespace()
        .any(|w| w == "package:io.freemytv")
    {
        println!("Free My TV is not installed on this TV.");
        return 1;
    }
    // Android 11 (Fire OS 8 too) keeps adb out of other apps' Android/data folders: then through the app itself.
    let direct = adb.run(&["shell", "mkdir", "-p", INBOX]).status.success();
    if !direct {
        // exec-in brings back nothing the TV prints: check once that its Free My TV has the way in.
        let (out, err) = text(&adb.run(&[
            "shell",
            "content",
            "query",
            "--uri",
            "content://io.freemytv.photos/",
        ]));
        if (out + &err).contains("Could not find provider") {
            println!(
                "This TV keeps adb out of the app's folder, and the Free My TV on it is too old for the other way in: \
                 install the new FreeMyTV.apk (see Install), then send them again."
            );
            return 1;
        }
    }
    for (i, path) in found.iter().enumerate() {
        // Numbered so the app imports them in this order.
        let name = remote_name(i, path);
        let r = if direct {
            adb.run(&["push", path, &format!("{INBOX}/{name}")])
        } else {
            // Encoded (a # or ? would cut the name), and with its size, so the app drops a send cut short.
            let size = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
            let uri = format!("content://io.freemytv.photos/{}?size={size}", quote(&name));
            adb.run_with(
                &["exec-in", "content", "write", "--uri", &uri],
                fs::File::open(path).ok(),
            )
        };
        if !r.status.success() {
            let (out, err) = text(&r);
            println!("Could not send {path}: {}", (out + &err).trim());
            return 1;
        }
        println!("Sent {path}");
    }
    println!(
        "{} photo(s) on the TV. Open the home screen or Free My TV's settings to import them.",
        found.len()
    );
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_like_python() {
        assert_eq!(quote("000-a_b.jpg"), "000-a_b.jpg");
        assert_eq!(quote("001-#1?.jpg"), "001-%231%3F.jpg");
        assert_eq!(quote("002-praia ção.jpg"), "002-praia%20%C3%A7%C3%A3o.jpg");
        assert_eq!(quote("a~b/c"), "a~b/c");
    }

    #[test]
    fn names() {
        assert_eq!(
            remote_name(0, "Holidays/beach day.JPG"),
            "000-beach_day.JPG"
        );
        assert_eq!(remote_name(12, "x.png"), "012-x.png");
    }

    #[test]
    fn walks_like_os_walk() {
        let dir = std::env::temp_dir().join(format!("fmt-photos-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        for p in [
            "b.jpg",
            "a.PNG",
            ".hidden.jpg",
            "notes.txt",
            "sub/c.webp",
            ".git/d.jpg",
            "a_sub/e.heic",
        ] {
            let f = dir.join(p);
            fs::create_dir_all(f.parent().unwrap()).unwrap();
            fs::write(f, b"x").unwrap();
        }
        let d = dir.to_string_lossy().into_owned();
        let (found, skipped) = photos(&[
            format!("{d}/"),
            format!("{d}/notes.txt"),
            "missing.jpg".into(),
        ]);
        let rel: Vec<String> = found.iter().map(|f| f[d.len()..].to_string()).collect();
        assert_eq!(rel, ["/a.PNG", "/b.jpg", "/a_sub/e.heic", "/sub/c.webp"]);
        assert_eq!(skipped.len(), 2);
        fs::remove_dir_all(&dir).unwrap();
    }
}
