//! The packages the TVs install: the LG `.ipk` (`webos/build.py`) and the Samsung `.wgt` (`tizen/build.py`, before
//! signing; signing stays with Samsung's `tizen` command).

use crate::json::{dumps, Style};
use crate::webapp::Files;
use crate::{Error, Repo};
use flate2::write::GzEncoder;
use flate2::Compression;
use regex::Regex;
use serde_json::{json, Value};
use std::io::{Cursor, Write};
use std::time::{SystemTime, UNIX_EPOCH};

/// What a build made.
#[derive(Debug)]
pub struct Built {
    pub app_id: String,
    pub version: String,
    pub data: Vec<u8>,
}

pub fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// A `.tar.gz` of (name, contents or None for a folder), owned by root, as LG's ares-package makes them.
pub fn tar_gz(entries: &[(String, Option<&[u8]>)], mtime: u64) -> Result<Vec<u8>, Error> {
    let gz = GzEncoder::new(Vec::new(), Compression::best());
    let mut tar = tar::Builder::new(gz);
    for (name, data) in entries {
        let mut h = tar::Header::new_ustar();
        h.set_path(name)?;
        h.set_mtime(mtime);
        h.set_uid(0);
        h.set_gid(0);
        match data {
            None => {
                h.set_entry_type(tar::EntryType::Directory);
                h.set_mode(0o755);
                h.set_size(0);
                h.set_cksum();
                tar.append(&h, std::io::empty())?;
            }
            Some(bytes) => {
                h.set_entry_type(tar::EntryType::Regular);
                h.set_mode(0o644);
                h.set_size(bytes.len() as u64);
                h.set_cksum();
                tar.append(&h, *bytes)?;
            }
        }
    }
    Ok(tar.into_inner()?.finish()?)
}

/// An `ar` archive (the `.deb`/`.ipk` container): a 60-byte header per member, members aligned to 2 bytes.
pub fn ar(members: &[(&str, &[u8])], mtime: u64) -> Vec<u8> {
    let mut out = b"!<arch>\n".to_vec();
    for (name, data) in members {
        let header = format!("{:<16}{:<12}{:<6}{:<6}{:<8}{:<10}`\n", name, mtime, 0, 0, "100644", data.len());
        out.extend_from_slice(header.as_bytes());
        out.extend_from_slice(data);
        if data.len() % 2 == 1 {
            out.push(b'\n');
        }
    }
    out
}

/// `FreeMyTV-webos.ipk`: `debian-binary`, `control.tar.gz` and `data.tar.gz` with the app in
/// `usr/palm/applications/<id>/`. `app` is [`crate::webapp::files`]; the webOS manifest and icons are added here.
pub fn ipk(repo: &Repo, mut app: Files, mtime: u64) -> Result<Built, Error> {
    let info: Value = serde_json::from_str(&repo.read_string("webos/appinfo.json")?).map_err(|e| Error(format!("appinfo.json: {e}")))?;
    let app_id = info["id"].as_str().ok_or(Error("appinfo.json has no id".into()))?.to_string();
    let version = info["version"].as_str().ok_or(Error("appinfo.json has no version".into()))?.to_string();
    for name in ["appinfo.json", "icon.png", "largeIcon.png"] {
        app.insert(name.into(), repo.read(&format!("webos/{name}"))?);
    }

    let base = format!("usr/palm/applications/{app_id}");
    let mut entries: Vec<(String, Option<&[u8]>)> =
        ["usr", "usr/palm", "usr/palm/applications", base.as_str()].iter().map(|d| (d.to_string(), None)).collect();
    let mut dirs = std::collections::HashSet::new();
    for (path, content) in &app {
        if let Some((parent, _)) = path.rsplit_once('/') {
            if dirs.insert(parent.to_string()) {
                entries.push((format!("{base}/{parent}"), None));
            }
        }
        entries.push((format!("{base}/{path}"), Some(content)));
    }
    let package = dumps(&json!({"id": app_id, "version": version, "app": app_id}), Style { indent: Some(2), ..Style::DEFAULT });
    entries.push(("usr/palm/packages".into(), None));
    entries.push((format!("usr/palm/packages/{app_id}"), None));
    entries.push((format!("usr/palm/packages/{app_id}/packageinfo.json"), Some(package.as_bytes())));

    let size: usize = app.values().map(Vec::len).sum::<usize>() + package.len();
    let control = format!(
        "Package: {app_id}\nVersion: {version}\nSection: misc\nPriority: optional\nArchitecture: all\nInstalled-Size: {size}\n\
         Maintainer: Free My TV <https://freemytv.io>\nDescription: A fast, ad-free home screen.\n\
         webOS-Package-Format-Version: 2\nwebOS-Packager-Version: 1.0.0\n"
    );
    let control_tar = tar_gz(&[("control".into(), Some(control.as_bytes()))], mtime)?;
    let data_tar = tar_gz(&entries, mtime)?;
    let data = ar(&[("debian-binary", b"2.0\n"), ("control.tar.gz", &control_tar), ("data.tar.gz", &data_tar)], mtime);
    Ok(Built { app_id, version, data })
}

/// `FreeMyTV-tizen.wgt`, not signed: a zip of the app with `tizen/config.xml` and `tizen/icon.png`.
pub fn wgt(repo: &Repo, mut app: Files) -> Result<Built, Error> {
    let manifest = repo.read_string("tizen/config.xml")?;
    let find = |re: &str| Regex::new(re).unwrap().captures(&manifest).map(|c| c[1].to_string());
    let app_id = find(r#"<tizen:application id="([^"]+)""#).ok_or(Error("config.xml has no tizen:application id".into()))?;
    let version = find(r#"<widget [^>]*version="([^"]+)""#).ok_or(Error("config.xml has no version".into()))?;
    for name in ["config.xml", "icon.png"] {
        app.insert(name.into(), repo.read(&format!("tizen/{name}"))?);
    }
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for (path, content) in &app {
        zip.start_file(path.as_str(), options).map_err(|e| Error(e.to_string()))?;
        zip.write_all(content)?;
    }
    let data = zip.finish().map_err(|e| Error(e.to_string()))?.into_inner();
    Ok(Built { app_id, version, data })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ar_layout() {
        let data = ar(&[("a", b"odd"), ("bb", b"even")], 7);
        assert_eq!(&data[..8], b"!<arch>\n");
        assert_eq!(data.len(), 8 + 60 + 3 + 1 + 60 + 4);
        assert_eq!(&data[8..68], format!("{:<16}{:<12}{:<6}{:<6}{:<8}{:<10}`\n", "a", 7, 0, 0, "100644", 3).as_bytes());
    }

    #[test]
    fn tar_gz_roundtrip() {
        let data = tar_gz(&[("d".into(), None), ("d/f".into(), Some(b"hi"))], 1).unwrap();
        let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(&data[..]));
        let entries: Vec<(String, u32, u64)> = archive
            .entries()
            .unwrap()
            .map(|e| {
                let e = e.unwrap();
                let h = e.header();
                (e.path().unwrap().to_string_lossy().into_owned(), h.mode().unwrap(), h.uid().unwrap())
            })
            .collect();
        assert_eq!(entries, vec![("d".into(), 0o755, 0), ("d/f".into(), 0o644, 0)]);
    }
}
