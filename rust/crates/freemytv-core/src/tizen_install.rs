//! Installs the `.wgt` on a Samsung TV in Developer Mode, speaking SDB itself (`tizen/install.py`).
//!
//! SDB is Samsung's ADB on port 26101: a 24-byte header whose checksum is a byte sum, streams opened with OPEN, and
//! every WRTE answered with OKAY before the next one. The package goes to the folder the TV names in its
//! capabilities and is installed with `0 vd_appinstall`; `0 was_execute` opens it.

use crate::Error;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::io::{ErrorKind, Read, Write};
use std::net::{TcpStream, ToSocketAddrs, UdpSocket};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub const PORT: u16 = 26101;
/// What this side announces; sdbd keeps each packet within both sides' maximum.
pub const MAX_DATA: u32 = 4096;
/// The largest packet taken from the TV: sdbd keeps to the maximum both sides announced, so anything far above it is
/// a broken or impostor device, not a reason to allocate gigabytes.
pub const MAX_PACKET: usize = 1 << 20;
/// The most a command's output may hold (an install's log is a few KB).
pub const MAX_OUTPUT: usize = 16 << 20;
/// Removing it on the TV itself.
pub const ON_TV: &str = "Apps, Settings, Free My TV, Delete";

type Cmd = [u8; 4];

/// One packet: the header (command, two arguments, length, byte-sum checksum, magic) and its data.
pub fn packet(cmd: &Cmd, arg0: u32, arg1: u32, data: &[u8]) -> Vec<u8> {
    let c = u32::from_le_bytes(*cmd);
    let sum = data.iter().fold(0u32, |s, &b| s.wrapping_add(b as u32));
    let mut out = Vec::with_capacity(24 + data.len());
    for v in [c, arg0, arg1, data.len() as u32, sum, c ^ 0xFFFF_FFFF] {
        out.extend_from_slice(&v.to_le_bytes());
    }
    out.extend_from_slice(data);
    out
}

/// A `sync:` request: four letters and a little-endian length or value.
pub fn sync_request(id: &[u8; 4], n: u32) -> Vec<u8> {
    let mut out = id.to_vec();
    out.extend_from_slice(&n.to_le_bytes());
    out
}

/// The SDB connection. Generic over the transport so tests can stand in for the TV.
pub struct Sdb<S: Read + Write> {
    sock: S,
    next_id: u32,
    okay: HashMap<u32, u32>,
    inbox: HashMap<u32, Vec<u8>>,
    closed: HashSet<u32>,
    pub max: u32,
    pub banner: String,
    set_timeout: fn(&S, Option<Duration>),
    timed_out: bool,
}

pub fn tcp_timeout(s: &TcpStream, d: Option<Duration>) {
    let _ = s.set_read_timeout(d);
}

fn closed_err() -> Error {
    Error("the TV closed the connection".into())
}

impl Sdb<TcpStream> {
    pub fn connect(ip: &str) -> Result<Sdb<TcpStream>, Error> {
        let addr = (ip, PORT)
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| Error(format!("bad address {ip}")))?;
        let sock = TcpStream::connect_timeout(&addr, Duration::from_secs(15))?;
        sock.set_read_timeout(Some(Duration::from_secs(15)))?;
        Sdb::handshake(sock, tcp_timeout)
    }
}

impl<S: Read + Write> Sdb<S> {
    pub fn handshake(sock: S, set_timeout: fn(&S, Option<Duration>)) -> Result<Sdb<S>, Error> {
        let mut sdb = Sdb {
            sock,
            next_id: 1,
            okay: HashMap::new(),
            inbox: HashMap::new(),
            closed: HashSet::new(),
            max: MAX_DATA,
            banner: String::new(),
            set_timeout,
            timed_out: false,
        };
        sdb.send(b"CNXN", 0x0100_0000, MAX_DATA, b"host::\0")?;
        let (cmd, _, max_data, data) = sdb.read()?;
        if &cmd != b"CNXN" {
            return Err(Error(format!(
                "the TV answered {:?} instead of connecting",
                String::from_utf8_lossy(&cmd)
            )));
        }
        // Each sync chunk carries an 8-byte header: a maximum that can't hold one is a broken device.
        if max_data <= 8 {
            return Err(Error(format!(
                "the TV announced packets of {max_data} bytes"
            )));
        }
        sdb.max = max_data.min(MAX_DATA);
        sdb.banner = String::from_utf8_lossy(&data)
            .trim_end_matches('\0')
            .to_string();
        Ok(sdb)
    }

    fn send(&mut self, cmd: &Cmd, arg0: u32, arg1: u32, data: &[u8]) -> Result<(), Error> {
        self.sock.write_all(&packet(cmd, arg0, arg1, data))?;
        Ok(())
    }

    fn recv(&mut self, n: usize) -> Result<Vec<u8>, Error> {
        let mut buf = vec![0; n];
        let mut got = 0;
        while got < n {
            match self.sock.read(&mut buf[got..]) {
                Ok(0) => return Err(closed_err()),
                Ok(k) => got += k,
                Err(e) if e.kind() == ErrorKind::Interrupted => {}
                Err(e) => {
                    self.timed_out =
                        matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut);
                    return Err(e.into());
                }
            }
        }
        Ok(buf)
    }

    fn read(&mut self) -> Result<(Cmd, u32, u32, Vec<u8>), Error> {
        let h = self.recv(24)?;
        let word = |i: usize| u32::from_le_bytes([h[i], h[i + 1], h[i + 2], h[i + 3]]);
        let cmd = [h[0], h[1], h[2], h[3]];
        let n = word(12) as usize;
        if n > MAX_PACKET {
            return Err(Error(format!(
                "the TV sent a {n}-byte packet (more than {MAX_PACKET})"
            )));
        }
        let data = if n > 0 { self.recv(n)? } else { Vec::new() };
        Ok((cmd, word(4), word(8), data))
    }

    /// Reads one packet and files it under its stream: OKAY, data (answered with OKAY) or close.
    fn pump(&mut self) -> Result<(), Error> {
        let (cmd, remote, local, data) = self.read()?;
        match &cmd {
            b"OKAY" => {
                self.okay.insert(local, remote);
            }
            b"WRTE" => {
                let inbox = self.inbox.entry(local).or_default();
                if inbox.len() + data.len() > MAX_OUTPUT {
                    return Err(Error(format!(
                        "the TV sent more than {MAX_OUTPUT} bytes on one stream"
                    )));
                }
                inbox.extend_from_slice(&data);
                self.send(b"OKAY", local, remote, b"")?;
            }
            b"CLSE" => {
                self.closed.insert(local);
            }
            _ => {}
        }
        Ok(())
    }

    pub fn open(&mut self, service: &str) -> Result<(u32, u32), Error> {
        let local = self.next_id;
        self.next_id += 1;
        let mut name = service.as_bytes().to_vec();
        name.push(0);
        self.send(b"OPEN", local, 0, &name)?;
        loop {
            if let Some(remote) = self.okay.remove(&local) {
                return Ok((local, remote));
            }
            if self.closed.contains(&local) {
                return Err(Error(format!("the TV refused {service}")));
            }
            self.pump()?;
        }
    }

    pub fn write(&mut self, stream: (u32, u32), data: &[u8]) -> Result<(), Error> {
        let (local, remote) = stream;
        self.send(b"WRTE", local, remote, data)?;
        loop {
            if self.okay.remove(&local).is_some() {
                return Ok(());
            }
            if self.closed.contains(&local) {
                return Err(Error("the TV closed the stream".into()));
            }
            self.pump()?;
        }
    }

    /// The stream's bytes until `done` says so, the TV closes it, or time runs out.
    pub fn read_until(
        &mut self,
        stream: (u32, u32),
        done: impl Fn(&[u8]) -> bool,
        seconds: u64,
    ) -> Result<Vec<u8>, Error> {
        let local = stream.0;
        let end = Instant::now() + Duration::from_secs(seconds);
        while Instant::now() < end
            && !self.closed.contains(&local)
            && !done(self.inbox.get(&local).map(Vec::as_slice).unwrap_or(b""))
        {
            let left = end
                .saturating_duration_since(Instant::now())
                .max(Duration::from_millis(100));
            (self.set_timeout)(&self.sock, Some(left));
            self.timed_out = false;
            match self.pump() {
                Ok(()) => {}
                Err(_) if self.timed_out => break,
                Err(e) => {
                    (self.set_timeout)(&self.sock, Some(Duration::from_secs(15)));
                    return Err(e);
                }
            }
        }
        (self.set_timeout)(&self.sock, Some(Duration::from_secs(15)));
        Ok(self.inbox.remove(&local).unwrap_or_default())
    }

    pub fn shell(
        &mut self,
        command: &str,
        until: Option<&Regex>,
        seconds: u64,
    ) -> Result<String, Error> {
        let stream = self.open(&format!("shell:{command}"))?;
        let out = self.read_until(
            stream,
            |t| until.is_some_and(|re| re.is_match(&String::from_utf8_lossy(t))),
            seconds,
        )?;
        Ok(String::from_utf8_lossy(&out).into_owned())
    }

    pub fn capabilities(&mut self) -> Result<HashMap<String, String>, Error> {
        let stream = self.open("capability:")?;
        let text = String::from_utf8_lossy(&self.read_until(stream, |_| false, 5)?).into_owned();
        Ok(parse_capabilities(&text))
    }

    /// Copies data to path on the TV (sync SEND), then checks the TV's answer.
    pub fn push(&mut self, path: &str, data: &[u8], mtime: u32) -> Result<(), Error> {
        let stream = self.open("sync:")?;
        let spec = format!("{path},{}", 0o100644);
        let mut send = sync_request(b"SEND", spec.len() as u32);
        send.extend_from_slice(spec.as_bytes());
        self.write(stream, &send)?;
        let step = (self.max - 8) as usize;
        for part in data.chunks(step) {
            let mut chunk = sync_request(b"DATA", part.len() as u32);
            chunk.extend_from_slice(part);
            self.write(stream, &chunk)?;
        }
        self.write(stream, &sync_request(b"DONE", mtime))?;
        let answer = self.read_until(stream, |t| t.len() >= 8, 60)?;
        if !answer.starts_with(b"OKAY") {
            let why = answer
                .get(8..)
                .map(String::from_utf8_lossy)
                .unwrap_or_default();
            return Err(Error(format!("the TV didn't take the file: {why}")));
        }
        self.send(b"WRTE", stream.0, stream.1, &sync_request(b"QUIT", 0))?;
        Ok(())
    }
}

pub fn parse_capabilities(text: &str) -> HashMap<String, String> {
    text.lines()
        .filter_map(|l| l.split_once(':'))
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

/// The TV's model from its banner (`device::<model>:...`), or the whole banner.
pub fn model(banner: &str) -> &str {
    let parts: Vec<&str> = banner.split(':').collect();
    if parts.len() >= 3 {
        parts[2]
    } else {
        banner
    }
}

/// The app id and package from the manifest's `<tizen:application>`.
pub fn ids(manifest: &str) -> Result<(String, String), Error> {
    let bad = || Error("config.xml has no <tizen:application id=... package=...>".into());
    let tag = Regex::new(r"<tizen:application[^>]*>")
        .unwrap()
        .find(manifest)
        .ok_or_else(bad)?
        .as_str();
    let attr = |name: &str| {
        Regex::new(&format!(r#"\b{name}="([^"]+)""#))
            .unwrap()
            .captures(tag)
            .map(|c| c[1].to_string())
            .ok_or_else(bad)
    };
    Ok((attr("id")?, attr("package")?))
}

/// Where the package goes on the TV: the folder its capabilities name.
pub fn remote_path(caps: &HashMap<String, String>) -> String {
    let dir = caps
        .get("sdk_toolpath")
        .map(String::as_str)
        .unwrap_or("/home/owner/share/tmp/sdk_tools");
    format!("{}/FreeMyTV-tizen.wgt", dir.trim_end_matches('/'))
}

/// This computer's address on the TV's network (what Host PC IP must be).
pub fn this_computer(ip: &str) -> String {
    UdpSocket::bind("0.0.0.0:0")
        .and_then(|s| {
            s.connect((ip, PORT))?;
            s.local_addr()
        })
        .map(|a| a.ip().to_string())
        .unwrap_or_else(|_| "this computer's IP".into())
}

pub fn now() -> u32 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as u32)
        .unwrap_or(0)
}

/// The installer's answer: Ok, or what to tell the person.
pub fn install_result(out: &str) -> Result<(), Error> {
    if out.contains("install completed") {
        return Ok(());
    }
    let mut msg = format!("The TV didn't finish installing. It said:\n{}", out.trim());
    if out.contains("different certificate") {
        msg += &format!(
            "\nThe Free My TV on the TV was signed with another certificate, and the TV only updates it with the \
             same one. Remove it on the TV ({ON_TV}), then run this again."
        );
    }
    Err(Error(msg))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::thread;

    #[test]
    fn header_matches_python_struct_pack() {
        // struct.pack('<6I', CNXN, 0x01000000, 4096, 7, sum(b'host::\0'), CNXN ^ 0xFFFFFFFF) + b'host::\0'
        let p = packet(b"CNXN", 0x0100_0000, 4096, b"host::\0");
        let expected: Vec<u8> = [
            0x43, 0x4e, 0x58, 0x4e, 0x00, 0x00, 0x00, 0x01, 0x00, 0x10, 0x00, 0x00, 0x07, 0x00,
            0x00, 0x00, 0x32, 0x02, 0x00, 0x00, 0xbc, 0xb1, 0xa7, 0xb1,
        ]
        .into_iter()
        .chain(*b"host::\0")
        .collect();
        assert_eq!(p, expected);
    }

    #[test]
    fn manifest_ids_and_paths() {
        let xml = r#"<widget><tizen:application id="FreeMyTVio.FreeMyTV" package="FreeMyTVio" required_version="3.0"/></widget>"#;
        assert_eq!(
            ids(xml).unwrap(),
            ("FreeMyTVio.FreeMyTV".into(), "FreeMyTVio".into())
        );
        assert!(ids("<widget/>").is_err());
        let mut caps =
            parse_capabilities("platform_version:5.0\nsdk_toolpath:/opt/usr/apps/tmp/\nbad line\n");
        assert_eq!(caps["platform_version"], "5.0");
        assert_eq!(remote_path(&caps), "/opt/usr/apps/tmp/FreeMyTV-tizen.wgt");
        caps.clear();
        assert_eq!(
            remote_path(&caps),
            "/home/owner/share/tmp/sdk_tools/FreeMyTV-tizen.wgt"
        );
        assert_eq!(model("device::UN50MU6100:x"), "UN50MU6100");
        assert_eq!(model("odd"), "odd");
    }

    #[test]
    fn install_answers() {
        assert!(install_result("... install completed\nspend time 3").is_ok());
        assert!(install_result("cmd_ret:0").is_err()); // cmd_ret:0 comes after failures too
        let e = install_result("signed with a different certificate").unwrap_err();
        assert!(e.0.contains("Remove it on the TV"));
    }

    /// A transport that answers with fixed bytes and swallows what is sent.
    struct Canned(std::io::Cursor<Vec<u8>>);
    impl Read for Canned {
        fn read(&mut self, b: &mut [u8]) -> std::io::Result<usize> {
            self.0.read(b)
        }
    }
    impl Write for Canned {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    fn header(cmd: &Cmd, arg0: u32, arg1: u32, len: u32) -> Vec<u8> {
        let c = u32::from_le_bytes(*cmd);
        [c, arg0, arg1, len, 0, c ^ 0xFFFF_FFFF]
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect()
    }

    #[test]
    fn refuses_what_an_impostor_could_send() {
        // A 4 GB packet: refused before allocating.
        let huge = header(b"CNXN", 0x0100_0000, 4096, u32::MAX);
        let e = Sdb::handshake(Canned(std::io::Cursor::new(huge)), |_, _| {})
            .err()
            .unwrap();
        assert!(e.0.contains("byte packet"), "{}", e.0);
        // A maximum too small for a sync chunk's header.
        let tiny = packet(b"CNXN", 0x0100_0000, 8, b"device::x:\0");
        let e = Sdb::handshake(Canned(std::io::Cursor::new(tiny)), |_, _| {})
            .err()
            .unwrap();
        assert!(e.0.contains("announced packets of 8 bytes"), "{}", e.0);
        // Endless output on one stream.
        let mut stream = packet(b"CNXN", 0x0100_0000, 4096, b"device::x:\0");
        stream.extend(packet(b"OKAY", 9, 1, b""));
        let chunk = vec![b'x'; MAX_PACKET];
        for _ in 0..(MAX_OUTPUT / MAX_PACKET + 1) {
            stream.extend(packet(b"WRTE", 9, 1, &chunk));
        }
        let mut sdb = Sdb::handshake(Canned(std::io::Cursor::new(stream)), |_, _| {}).unwrap();
        let e = sdb.shell("x", None, 5).err().unwrap();
        assert!(e.0.contains("more than"), "{}", e.0);
    }

    /// A stand-in sdbd: answers CNXN, OPEN with OKAY, takes the pushed file and runs one shell command.
    fn fake_tv(listener: TcpListener) -> thread::JoinHandle<Vec<u8>> {
        thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            let read = |s: &mut TcpStream| -> Option<(Cmd, u32, u32, Vec<u8>)> {
                let mut h = [0u8; 24];
                s.read_exact(&mut h).ok()?;
                let w = |i: usize| u32::from_le_bytes([h[i], h[i + 1], h[i + 2], h[i + 3]]);
                let mut d = vec![0; w(12) as usize];
                s.read_exact(&mut d).ok()?;
                Some(([h[0], h[1], h[2], h[3]], w(4), w(8), d))
            };
            let mut file = Vec::new();
            let mut sync = Vec::new();
            while let Some((cmd, a0, a1, data)) = read(&mut s) {
                match &cmd {
                    b"CNXN" => s
                        .write_all(&packet(
                            b"CNXN",
                            0x0100_0000,
                            1024,
                            b"device::UN55RU7100:\0",
                        ))
                        .unwrap(),
                    b"OPEN" => {
                        let service = String::from_utf8_lossy(&data)
                            .trim_end_matches('\0')
                            .to_string();
                        let remote = 100 + a0;
                        s.write_all(&packet(b"OKAY", remote, a0, b"")).unwrap();
                        let reply: &[u8] = match service.as_str() {
                            "capability:" => b"platform_version:5.0\nsdk_toolpath:/tmp/sdk\n",
                            s if s.starts_with("shell:0 vd_appinstall") => {
                                b"install completed\nspend time 2\n"
                            }
                            _ => b"",
                        };
                        if !reply.is_empty() {
                            s.write_all(&packet(b"WRTE", remote, a0, reply)).unwrap();
                            s.write_all(&packet(b"CLSE", remote, a0, b"")).unwrap();
                        }
                    }
                    b"WRTE" => {
                        s.write_all(&packet(b"OKAY", a1, a0, b"")).unwrap();
                        sync.extend_from_slice(&data);
                        if sync.len() >= 8 && &sync[sync.len() - 8..sync.len() - 4] == b"DONE" {
                            // Parse SEND, DATA... from the stream and answer OKAY.
                            let mut i = 0;
                            while i + 8 <= sync.len() {
                                let id = &sync[i..i + 4];
                                let n = u32::from_le_bytes(sync[i + 4..i + 8].try_into().unwrap())
                                    as usize;
                                if id == b"DATA" {
                                    file.extend_from_slice(&sync[i + 8..i + 8 + n]);
                                }
                                i += 8 + if id == b"DONE" { 0 } else { n };
                            }
                            s.write_all(&packet(b"WRTE", a1, a0, &sync_request(b"OKAY", 0)))
                                .unwrap();
                            sync.clear();
                        }
                    }
                    _ => {}
                }
            }
            file
        })
    }

    #[test]
    fn talks_to_a_stand_in_tv() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let tv = fake_tv(listener);
        let sock = TcpStream::connect(("127.0.0.1", port)).unwrap();
        sock.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        let mut sdb = Sdb::handshake(sock, tcp_timeout).unwrap();
        assert_eq!(sdb.max, 1024);
        assert_eq!(model(&sdb.banner), "UN55RU7100");
        let caps = sdb.capabilities().unwrap();
        assert_eq!(remote_path(&caps), "/tmp/sdk/FreeMyTV-tizen.wgt");
        let data: Vec<u8> = (0..5000u32).map(|i| (i % 251) as u8).collect();
        sdb.push(&remote_path(&caps), &data, 1).unwrap();
        let out = sdb
            .shell(
                "0 vd_appinstall FreeMyTVio /tmp/sdk/FreeMyTV-tizen.wgt",
                Some(&Regex::new("spend time").unwrap()),
                5,
            )
            .unwrap();
        assert!(install_result(&out).is_ok());
        drop(sdb);
        assert_eq!(tv.join().unwrap(), data);
    }
}
