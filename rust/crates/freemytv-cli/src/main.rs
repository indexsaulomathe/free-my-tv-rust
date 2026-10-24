//! `freemytv`: the Rust version of the Python build tools (one subcommand per script).
//!
//!     freemytv webos-build [--generic | --config FILE] [--preview] [--out FILE]   # webos/build.py
//!     freemytv tizen-build [--generic | --config FILE] [--preview] [--out FILE]   # tizen/build.py (unsigned)
//!     freemytv homebrew-repo FILE.ipk [--out FILE]                                # tools/homebrew_repo.py
//!     freemytv make-zones [ZONEINFO_DIR] [--out FILE]                             # tools/make_zones.py
//!     freemytv webos-install IP [--uninstall]                                     # webos/install.py
//!
//! Run it anywhere inside the repository (or pass --root DIR).

use freemytv_core::config::Config;
use freemytv_core::{homebrew, package, webapp, webos_install, zones, Error, Repo};
use std::path::{Path, PathBuf};
use std::{env, fs, process};

const USAGE: &str = "usage: freemytv <webos-build|tizen-build|homebrew-repo|make-zones|webos-install> [options]
  webos-build   [--generic | --config FILE] [--preview] [--out FILE]
  tizen-build   [--generic | --config FILE] [--preview] [--out FILE]
  homebrew-repo FILE.ipk [--out FILE]
  make-zones    [ZONEINFO_DIR] [--out FILE]
  webos-install IP [--uninstall]   installs FreeMyTV-webos.ipk on an LG TV (builds it first if missing)
  --root DIR    the repository (default: found from the current folder)";

struct Args {
    list: Vec<String>,
}

impl Args {
    fn flag(&self, name: &str) -> bool {
        self.list.iter().any(|a| a == name)
    }
    fn value(&self, name: &str) -> Option<String> {
        self.list
            .iter()
            .position(|a| a == name)
            .and_then(|i| self.list.get(i + 1))
            .cloned()
    }
    /// Arguments that are neither flags nor flag values.
    fn positional(&self) -> Vec<String> {
        let mut out = Vec::new();
        let mut skip = false;
        for a in &self.list {
            if skip {
                skip = false;
            } else if matches!(a.as_str(), "--config" | "--out" | "--root") {
                skip = true;
            } else if !a.starts_with("--") {
                out.push(a.clone());
            }
        }
        out
    }
}

fn config(repo: &Repo, args: &Args) -> Result<Config, Error> {
    if args.flag("--generic") {
        return Ok(Config::generic());
    }
    let path = args
        .value("--config")
        .map(PathBuf::from)
        .unwrap_or_else(|| repo.path("config.properties"));
    Config::read(&path, &repo.path("config.example.properties"))
}

fn kb(n: usize) -> usize {
    n / 1024
}

fn build(repo: &Repo, args: &Args, lg: bool) -> Result<(), Error> {
    let cfg = config(repo, args)?;
    let made_by = if lg {
        "webos/build.py"
    } else {
        "tizen/build.py"
    };
    let app = webapp::files(repo, &cfg, made_by)?;
    let preview = if lg { "out/webos" } else { "out/tizen" };
    if args.flag("--preview") {
        webapp::preview(&app, &repo.path(preview))?;
    }
    let built = if lg {
        package::ipk(repo, app, package::now())?
    } else {
        package::wgt(repo, app)?
    };
    let default = if lg {
        "FreeMyTV-webos.ipk"
    } else {
        "FreeMyTV-tizen.wgt"
    };
    let out = args
        .value("--out")
        .map(PathBuf::from)
        .unwrap_or_else(|| repo.path(default));
    fs::write(&out, &built.data)?;
    if args.flag("--preview") {
        println!("Preview: open {preview}/index.html (or serve {preview}) in a browser.");
    }
    let name = out
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    if lg {
        println!(
            "Built {name} ({} {}, {} KB)",
            built.app_id,
            built.version,
            kb(built.data.len())
        );
    } else {
        println!(
            "Built {name} ({} {}, {} KB, not signed)",
            built.app_id,
            built.version,
            kb(built.data.len())
        );
    }
    if cfg.tmdb_key.is_empty() {
        println!("No TMDB key: the picks stay off (everything else works).");
    }
    Ok(())
}

fn homebrew_repo(repo: &Repo, args: &Args) -> Result<(), Error> {
    let pos = args.positional();
    let Some(ipk) = pos.first().filter(|p| p.ends_with(".ipk")) else {
        eprintln!("usage: freemytv homebrew-repo FILE.ipk [--out FILE]");
        process::exit(2);
    };
    let data = fs::read(ipk).map_err(|e| Error(format!("{ipk}: {e}")))?;
    let sha = homebrew::sha256_hex(&data);
    let text = homebrew::repository(repo, &sha, data.len() as u64)?;
    let out = args
        .value("--out")
        .map(PathBuf::from)
        .unwrap_or_else(|| repo.path("docs/webos/apps.json"));
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&out, text)?;
    println!(
        "Wrote {}: {} bytes, sha256 {}…",
        out.display(),
        data.len(),
        &sha[..12]
    );
    Ok(())
}

fn make_zones(repo: &Repo, args: &Args) -> Result<(), Error> {
    let base = args
        .positional()
        .first()
        .cloned()
        .unwrap_or_else(|| "/usr/share/zoneinfo".into());
    let base = Path::new(&base);
    let read = |name: &str| {
        fs::read_to_string(base.join(name))
            .map_err(|e| Error(format!("{}: {e}", base.join(name).display())))
    };
    let map = zones::zones(&read("zone.tab")?, &read("tzdata.zi")?);
    let js = zones::zones_js(&map);
    let out = args
        .value("--out")
        .map(PathBuf::from)
        .unwrap_or_else(|| repo.path("web/zones.js"));
    fs::write(&out, &js)?;
    println!(
        "Wrote {} ({} zones, {} KB)",
        out.display(),
        map.len(),
        kb(js.len())
    );
    Ok(())
}

fn home() -> Result<PathBuf, Error> {
    env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .ok_or(Error("no home folder (HOME)".into()))
}

fn has(tool: &str) -> bool {
    process::Command::new(tool)
        .arg("-V")
        .stdout(process::Stdio::null())
        .stderr(process::Stdio::null())
        .status()
        .is_ok()
}

fn webos_install(repo: &Repo, args: &Args) -> Result<(), Error> {
    let pos = args.positional();
    let Some(ip) = pos.first().filter(|ip| webos_install::valid_ip(ip)) else {
        eprintln!("usage: freemytv webos-install IP [--uninstall]");
        process::exit(2);
    };
    if !has("ssh") || !has("ssh-keygen") {
        return Err(Error("OpenSSH (ssh and ssh-keygen) is needed.".into()));
    }
    let appinfo: serde_json::Value = serde_json::from_slice(&repo.read("webos/appinfo.json")?)
        .map_err(|e| Error(format!("webos/appinfo.json: {e}")))?;
    let app_id = appinfo["id"].as_str().unwrap_or_default().to_string();
    let home = home()?;
    let tv = webos_install::Tv {
        ip: ip.clone(),
        key: webos_install::fetch_key(&home, ip)?,
        known_hosts: webos_install::known_hosts(&home),
    };
    if args.flag("--uninstall") {
        println!("{}", tv.uninstall(&app_id));
        return Ok(());
    }
    let ipk = repo.path("FreeMyTV-webos.ipk");
    if !ipk.exists() {
        build(repo, args, true)?;
    }
    tv.install(&ipk, &app_id)?;
    println!("Installed and open on the TV.");
    println!("Tip: while Free My TV is open, hold a number key on the remote to open it with that key from anywhere.");
    println!("Developer Mode lasts about 40 days: open the Developer Mode app now and then and press EXTEND.");
    Ok(())
}

fn run() -> Result<(), Error> {
    let mut all: Vec<String> = env::args().skip(1).collect();
    if all.is_empty() || matches!(all[0].as_str(), "-h" | "--help" | "help") {
        println!("{USAGE}");
        return Ok(());
    }
    let command = all.remove(0);
    let args = Args { list: all };
    let repo = match args.value("--root") {
        Some(r) => Repo::new(r),
        None => Repo::find(&env::current_dir()?).ok_or(Error(
            "not inside the Free My TV repository (use --root DIR)".into(),
        ))?,
    };
    match command.as_str() {
        "webos-build" => build(&repo, &args, true),
        "tizen-build" => build(&repo, &args, false),
        "homebrew-repo" => homebrew_repo(&repo, &args),
        "make-zones" => make_zones(&repo, &args),
        "webos-install" => webos_install(&repo, &args),
        other => Err(Error(format!("unknown command {other:?}\n{USAGE}"))),
    }
}

fn main() {
    if let Err(e) = run() {
        eprintln!("freemytv: {e}");
        process::exit(1);
    }
}
