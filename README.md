# Free My TV Rust

**An independently developed fork of [Free My TV](https://github.com/simbasjungles/free-my-tv), gradually converting its tooling to Rust.**

Free My TV provides an alternative home screen for Google TV and Android TV, plus apps for LG webOS and Samsung Tizen. This fork keeps the original project as its foundation and develops and verifies Rust replacements for selected build and install tools. The GitHub fork relationship remains in place.

![Free My TV home screen](.github/preview.webp)

## Project status

| Component | Current implementation | Status in this fork |
| --- | --- | --- |
| Android and Google TV app | Java | Original app retained; Rust conversion has not reached the app. |
| LG and Samsung app interface | JavaScript ES5 | Original interface retained. An experimental WebAssembly conversion was reverted after a measured regression. |
| Build and install tools | Python and Rust | Rust CLI implements webOS and Tizen builds and installs, Homebrew metadata, time zone generation, translation checks, and Android photo transfer. Python tools remain available. |
| Rust on webOS | ARMv7 probes | Minimal static and dynamic executables ran on a test TV. This does not establish that a complete Rust app runs there. |

See [Rust conversion status](documentacao/rust.md), [test coverage](documentacao/testes.md), and the [change log](documentacao/alteracoes.md). Results for simulated devices are identified in those documents.

## Get the code

```bash
git clone https://github.com/indexsaulomathe/free-my-tv-rust.git
cd free-my-tv-rust
```

Build the Rust CLI with a [Rust toolchain](https://www.rust-lang.org/tools/install):

```bash
cargo build --release --manifest-path rust/Cargo.toml
rust/target/release/freemytv
```

Run commands from the repository root. The CLI's usage output lists its commands and options. A project-specific `config.properties` is created by the build tools when needed. Set your location and, for recommendations, your own TMDB key there. Keep this file and signing keys out of Git.

## Install

These commands use a local `TV_IP` variable. Set it to your TV's address on your own computer; do not put addresses, credentials, or key files in a public issue.

### Android and Google TV

The Android app still uses the original Java build. It requires JDK 17 or newer, Android SDK command line tools (build-tools 34 and platform android-34), and `adb`.

```bash
./build.sh
TV_IP="<your-tv-ip>"
adb connect "$TV_IP:5555"
adb install -r FreeMyTV.apk
```

The first build may create `config.properties` and ask you to fill it before building again. Enable developer options and debugging on the TV before connecting. On the TV, open **Free My TV → Home Screen → Free My TV** and accept its own debugging prompt to switch home screens. See [Android app documentation](documentacao/app-android.md) for behavior and platform details.

### LG webOS

Install LG's **Developer Mode** app on the TV, sign in, enable **Dev Mode Status** and **Key Server**, and keep the TV and computer on the same network. The first install asks for the passphrase shown by Key Server. The Rust installer verifies the SSH host key.

```bash
rust/target/release/freemytv webos-build
TV_IP="<your-tv-ip>"
rust/target/release/freemytv webos-install "$TV_IP"
```

This installs an LG app; it does not replace LG's system home screen. Open it from the app list or assign a remote number key. Developer Mode must remain active. See [webOS tooling and test results](documentacao/rust.md).

### Samsung Tizen

The Rust CLI builds an **unsigned** `.wgt` by default. A retail TV needs a signed package made with Samsung's Tizen tools and a certificate valid for that TV. After setting up a signing profile, build and sign with the existing Python tool, then install with Rust:

```bash
python3 tizen/build.py --sign "<profile-name>"
TV_IP="<your-tv-ip>"
rust/target/release/freemytv tizen-install "$TV_IP"
```

Enable Developer Mode on the TV and set **Host PC IP** to your computer's local address. The Rust installer has been tested against a simulated Tizen TV; installation on a physical Samsung TV has not been verified for this fork. See [Tizen tooling and test results](documentacao/rust.md).

## Restore or remove

- **Android and Google TV:** On the TV, choose **Free My TV → Home Screen → Original** before uninstalling. The app's watchdog also restores the original home screen after repeated crashes.
- **LG webOS:** Run `rust/target/release/freemytv webos-install "$TV_IP" --uninstall`. LG's home screen stays available while the app is installed.
- **Samsung Tizen:** Run `rust/target/release/freemytv tizen-install "$TV_IP" --uninstall` or remove the app through the TV's Apps settings if the device does not answer the uninstall command.
- **Repository changes:** This fork's changes and their reversals are recorded in the [change log](documentacao/alteracoes.md). No firmware, partition, or TV service changes are required by the Rust compatibility probes.

## Development and checks

```bash
tools/check.sh
tests/run.sh
cargo test --manifest-path rust/Cargo.toml
```

`tools/check.sh` needs the Android build tools, Python 3, and Node.js. Its `web` mode checks translations, LG and Samsung packages, and ES5 compatibility without building Android. The [documentation index](documentacao/README.md) maps the codebase and test procedures.

## Origin and credits

This repository is a fork of **[simbasjungles/free-my-tv](https://github.com/simbasjungles/free-my-tv)**. The original authors created the Android, webOS, and Tizen apps and their Python tooling. This fork adds Rust tooling, tests, compatibility experiments, and documentation. The upstream [freemytv.io](https://freemytv.io) site and upstream release channels belong to the original project; they are not releases of this fork.

The original project's [credits](https://github.com/simbasjungles/free-my-tv#credits) cover its contributors, third-party services, fonts, and icons. This fork is not affiliated with TV manufacturers or streaming services. See [MIT license](LICENSE).
