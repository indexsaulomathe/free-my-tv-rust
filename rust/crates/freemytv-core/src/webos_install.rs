//! Installs the `.ipk` on an LG TV in Developer Mode (`webos/install.py`).
//!
//! Same flow as the Python: the TV's key from its Key Server (port 9991), unlocked once and kept in
//! `~/.ssh/freemytv_webos_<IP>`, the package copied over SSH (port 9922) and installed with the TV's
//! `com.webos.appInstallService`. Two differences, both for safety (`documentacao/seguranca.md`, #4):
//! the TV's host key is remembered in its own known_hosts on the first connection and checked after
//! that, and the passphrase is asked by `ssh-keygen` itself, so it never shows up on a command line.

use crate::Error;
use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::Duration;

pub const TEMP: &str = "/media/developer/temp/freemytv.ipk";
const SSH_PORT: &str = "9922";
const KEY_SERVER_PORT: u16 = 9991;

/// An IPv4 address written the way the Python accepts it (`^\d{1,3}(\.\d{1,3}){3}$`).
pub fn valid_ip(ip: &str) -> bool {
    let parts: Vec<&str> = ip.split('.').collect();
    parts.len() == 4
        && parts
            .iter()
            .all(|p| (1..=3).contains(&p.len()) && p.bytes().all(|b| b.is_ascii_digit()))
}

/// Where the unlocked key lives: the same file the Python uses, so either tool finds it.
pub fn key_path(home: &Path, ip: &str) -> PathBuf {
    home.join(".ssh")
        .join(format!("freemytv_webos_{}", ip.replace('.', "_")))
}

/// The TVs' host keys, kept apart from the person's own known_hosts.
pub fn known_hosts(home: &Path) -> PathBuf {
    home.join(".ssh").join("freemytv_webos_known_hosts")
}

/// ssh's options: webOS still speaks ssh-rsa; the host key is learned once (`accept-new`) and then checked.
pub fn ssh_options(known_hosts: &Path) -> Vec<String> {
    [
        "-o",
        "StrictHostKeyChecking=accept-new",
        "-o",
        &format!("UserKnownHostsFile={}", known_hosts.display()),
        "-o",
        "LogLevel=ERROR",
        "-o",
        "HostKeyAlgorithms=+ssh-rsa",
        "-o",
        "PubkeyAcceptedKeyTypes=+ssh-rsa",
        "-p",
        SSH_PORT,
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

/// The shell line that sends one Luna message on the TV.
/// luna-send-pub prints nothing without a terminal (-tt), and the installer streams several replies.
/// Older webOS ships BusyBox before 1.30, whose timeout only takes -t SECS.
/// Some (webOS 6) refuse the terminal; there `-i` streams the replies without one, until the timeout.
pub fn luna_command(
    uri: &str,
    payload: &serde_json::Value,
    replies: u32,
    wait: u32,
    tty: bool,
) -> String {
    let timeout =
        format!("if timeout -t 1 true 2>/dev/null; then T='timeout -t {wait}'; else T='timeout {wait}'; fi; ");
    let message = format!(
        "{uri} '{}'",
        serde_json::to_string(payload).unwrap_or_default()
    );
    if tty {
        format!("{timeout}$T luna-send-pub -n {replies} {message}")
    } else {
        format!("{timeout}$T luna-send-pub -i {message} </dev/null")
    }
}

/// Whether the installer's replies say it finished.
pub fn installed(out: &str) -> bool {
    out.contains("\"state\":\"installed\"")
}

/// Whether a Luna reply says yes.
pub fn returned_true(out: &str) -> bool {
    out.contains("\"returnValue\":true")
}

/// The body of a plain HTTP/1.0 response, or why there isn't one.
pub fn http_body(response: &[u8]) -> Result<&[u8], Error> {
    let end = response
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or_else(|| Error("the Key Server's answer had no body".into()))?;
    let status = String::from_utf8_lossy(&response[..end]);
    let code = status
        .lines()
        .next()
        .unwrap_or("")
        .split_whitespace()
        .nth(1)
        .unwrap_or("");
    if code != "200" {
        return Err(Error(format!("the Key Server answered {code}")));
    }
    Ok(&response[end + 4..])
}

/// The locked key from the TV's Key Server (`http://IP:9991/webos_rsa`).
fn download_key(ip: &str) -> Result<Vec<u8>, Error> {
    let addr: SocketAddr = format!("{ip}:{KEY_SERVER_PORT}")
        .parse()
        .map_err(|_| Error(format!("bad address {ip}")))?;
    let timeout = Duration::from_secs(6);
    let mut s = TcpStream::connect_timeout(&addr, timeout)?;
    s.set_read_timeout(Some(timeout))?;
    write!(s, "GET /webos_rsa HTTP/1.0\r\nHost: {ip}\r\n\r\n")?;
    let mut response = Vec::new();
    s.read_to_end(&mut response)?;
    Ok(http_body(&response)?.to_vec())
}

/// The unlocked key, fetched and unlocked the first time.
pub fn fetch_key(home: &Path, ip: &str) -> Result<PathBuf, Error> {
    let path = key_path(home, ip);
    if path.exists() {
        return Ok(path);
    }
    println!("Getting the TV's key from http://{ip}:{KEY_SERVER_PORT}/webos_rsa ...");
    let data = download_key(ip).map_err(|e| {
        Error(format!(
            "Couldn't reach the Key Server ({e}). In the TV's Developer Mode app, turn on Key Server."
        ))
    })?;
    let dir = path.parent().unwrap_or(home);
    fs::create_dir_all(dir)?;
    let tmp = dir.join(format!(".freemytv_webos_{}.tmp", std::process::id()));
    write_private(&tmp, &data)?;
    println!("Type the passphrase shown in Developer Mode on the TV (ssh-keygen asks for it as the old passphrase).");
    // Inherited terminal: ssh-keygen reads the passphrase itself, without echo, never on a command line.
    let ok = Command::new("ssh-keygen")
        .args(["-q", "-p", "-N", "", "-f"])
        .arg(&tmp)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !ok {
        let _ = fs::remove_file(&tmp);
        return Err(Error(
            "That passphrase didn't unlock the key. Check it on the TV (Developer Mode) and try again.".into(),
        ));
    }
    fs::rename(&tmp, &path)?;
    Ok(path)
}

#[cfg(unix)]
fn write_private(path: &Path, data: &[u8]) -> Result<(), Error> {
    use std::os::unix::fs::OpenOptionsExt;
    let mut f = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?;
    f.write_all(data)?;
    Ok(())
}

#[cfg(not(unix))]
fn write_private(path: &Path, data: &[u8]) -> Result<(), Error> {
    fs::write(path, data)?;
    Ok(())
}

/// One SSH session to the TV.
pub struct Tv {
    pub ip: String,
    pub key: PathBuf,
    pub known_hosts: PathBuf,
}

impl Tv {
    pub fn ssh(&self, command: &str, stdin: Option<fs::File>, tty: bool) -> Result<Output, Error> {
        let mut c = Command::new("ssh");
        if tty {
            c.arg("-tt");
        }
        c.args(ssh_options(&self.known_hosts))
            .arg("-i")
            .arg(&self.key)
            .arg(format!("prisoner@{}", self.ip))
            .arg(command);
        c.stdin(match stdin {
            Some(f) => Stdio::from(f),
            None => Stdio::null(),
        });
        c.output().map_err(|e| Error(format!("ssh: {e}")))
    }

    pub fn luna(&self, uri: &str, payload: serde_json::Value, replies: u32, wait: u32) -> String {
        let run = |tty| {
            self.ssh(&luna_command(uri, &payload, replies, wait, tty), None, tty)
                .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
                .unwrap_or_default()
        };
        let out = run(true);
        if out.trim().is_empty() {
            run(false)
        } else {
            out
        }
    }

    pub fn install(&self, ipk: &Path, app_id: &str) -> Result<(), Error> {
        println!("Copying the app to the TV ...");
        let copy = self.ssh(
            &format!("mkdir -p /media/developer/temp && cat > {TEMP}"),
            Some(fs::File::open(ipk)?),
            false,
        )?;
        if !copy.status.success() {
            let err = String::from_utf8_lossy(&copy.stderr);
            let hint = if err.contains("HOST IDENTIFICATION HAS CHANGED") {
                format!(
                    "The TV's host key changed (Developer Mode reinstalled?). If it's the same TV, run: ssh-keygen -R '[{}]:{SSH_PORT}' -f {}",
                    self.ip,
                    self.known_hosts.display()
                )
            } else {
                format!(
                    "Is Developer Mode still on? If the key changed, delete {} and try again.",
                    self.key.display()
                )
            };
            return Err(Error(format!("Couldn't copy it: {}\n{hint}", err.trim())));
        }
        println!("Installing ...");
        let out = self.luna(
            "luna://com.webos.appInstallService/dev/install",
            serde_json::json!({"id": app_id, "ipkUrl": TEMP, "subscribe": true}),
            30,
            60,
        );
        if !installed(&out) {
            return Err(Error(format!(
                "The TV didn't finish installing. It said:\n{}",
                out.trim()
            )));
        }
        let _ = self.ssh(&format!("rm -f {TEMP}"), None, false);
        self.luna(
            "luna://com.webos.applicationManager/launch",
            serde_json::json!({"id": app_id}),
            1,
            20,
        );
        Ok(())
    }

    pub fn uninstall(&self, app_id: &str) -> String {
        let out = self.luna(
            "luna://com.webos.appInstallService/dev/remove",
            serde_json::json!({"id": app_id}),
            3,
            30,
        );
        if returned_true(&out) {
            "Removed.".into()
        } else {
            format!("The TV said: {}", out.trim())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ip_check_matches_python() {
        assert!(valid_ip("192.168.1.50"));
        assert!(valid_ip("999.1.1.1")); // the Python regex takes it too
        assert!(!valid_ip("192.168.1"));
        assert!(!valid_ip("192.168.1.50; rm -rf /"));
        assert!(!valid_ip("tv.local"));
        assert!(!valid_ip("1.2.3.1234"));
    }

    #[test]
    fn key_path_is_the_pythons() {
        let p = key_path(Path::new("/home/x"), "192.168.1.50");
        assert_eq!(p, Path::new("/home/x/.ssh/freemytv_webos_192_168_1_50"));
    }

    #[test]
    fn host_key_is_checked() {
        let o = ssh_options(Path::new("/h/.ssh/freemytv_webos_known_hosts")).join(" ");
        assert!(o.contains("StrictHostKeyChecking=accept-new"));
        assert!(o.contains("UserKnownHostsFile=/h/.ssh/freemytv_webos_known_hosts"));
        assert!(!o.contains("StrictHostKeyChecking=no"));
        assert!(o.ends_with("-p 9922"));
    }

    #[test]
    fn luna_lines_match_python() {
        let p = serde_json::json!({"id": "io.freemytv", "ipkUrl": TEMP, "subscribe": true});
        let tty = luna_command(
            "luna://com.webos.appInstallService/dev/install",
            &p,
            30,
            60,
            true,
        );
        assert_eq!(
            tty,
            "if timeout -t 1 true 2>/dev/null; then T='timeout -t 60'; else T='timeout 60'; fi; \
             $T luna-send-pub -n 30 luna://com.webos.appInstallService/dev/install \
             '{\"id\":\"io.freemytv\",\"ipkUrl\":\"/media/developer/temp/freemytv.ipk\",\"subscribe\":true}'"
        );
        let plain = luna_command("luna://x/y", &serde_json::json!({"id": "a"}), 1, 20, false);
        assert!(plain.ends_with("$T luna-send-pub -i luna://x/y '{\"id\":\"a\"}' </dev/null"));
    }

    #[test]
    fn replies() {
        assert!(installed(
            r#"{"id":"io.freemytv","details":{"state":"installed"}}"#
        ));
        assert!(!installed(r#"{"details":{"state":"installing"}}"#));
        assert!(returned_true(r#"{"returnValue":true}"#));
        assert!(!returned_true(r#"{"returnValue":false}"#));
    }

    #[test]
    fn key_server_answers() {
        assert_eq!(
            http_body(b"HTTP/1.0 200 OK\r\nX: y\r\n\r\nKEY").unwrap(),
            b"KEY"
        );
        assert!(http_body(b"HTTP/1.0 404 Not Found\r\n\r\n").is_err());
        assert!(http_body(b"garbage").is_err());
    }
}
