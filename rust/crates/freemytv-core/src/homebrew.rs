//! `docs/webos/apps.json`, the webOS Homebrew Channel repository (`tools/homebrew_repo.py`).

use crate::json::{dumps, Style};
use crate::{Error, Repo};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

pub const ICON: &str = "https://freemytv.io/assets/icon-512.png";
pub const SOURCE: &str = "https://github.com/simbasjungles/free-my-tv";
const DESCRIPTION: &str = "A fast, ad-free home screen: your apps on a glass shelf, the weather and prices, day and night.";

pub fn sha256_hex(data: &[u8]) -> String {
    Sha256::digest(data).iter().map(|b| format!("{b:02x}")).collect()
}

/// The repository's JSON (with its final newline) for the version in `webos/appinfo.json`.
pub fn repository(repo: &Repo, sha256: &str, size: u64) -> Result<String, Error> {
    let app: Value = serde_json::from_str(&repo.read_string("webos/appinfo.json")?).map_err(|e| Error(e.to_string()))?;
    let version = app["version"].as_str().unwrap_or_default();
    let manifest = json!({
        "id": app["id"],
        "version": version,
        "type": "web",
        "title": app["title"],
        "appDescription": DESCRIPTION,
        "iconUri": ICON,
        "sourceUrl": SOURCE,
        "rootRequired": false,
        "ipkUrl": format!("{SOURCE}/releases/download/v{version}/FreeMyTV-webos.ipk"),
        "ipkHash": {"sha256": sha256},
        "ipkSize": size,
    });
    let doc = json!({
        "paging": {"page": 1, "count": 1, "maxPage": 1, "itemsTotal": 1, "prevUrl": null, "nextUrl": null},
        "packages": [{
            "id": app["id"],
            "title": app["title"],
            "iconUri": ICON,
            "shortDescription": DESCRIPTION,
            "pool": "main",
            "manifest": manifest,
        }],
    });
    Ok(dumps(&doc, Style { indent: Some(2), ..Style::DEFAULT }) + "\n")
}

#[cfg(test)]
mod tests {
    #[test]
    fn sha() {
        assert_eq!(super::sha256_hex(b"hello"), "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824");
    }
}
