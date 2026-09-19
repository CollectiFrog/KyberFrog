// SPDX-License-Identifier: AGPL-3.0-or-later

//! Generation of per-instance `kyber_config.toml` files.
//!
//! A transmitter's config is the operator's [`Directory::defaults`] with the
//! transmitter-specific values layered on top:
//!
//! * `[kycontroller].port` — the control-plane port.
//! * `[kycontroller].auth.basic` — a transparent default login is injected when
//!   the operator declared no auth, so the transmitter is reachable (kycontroller
//!   has no anonymous mode). Any operator-provided auth is left untouched.
//! * `[kycontroller].tray` — defaulted to `false`: instances are managed from
//!   the KyberFrog Server tray, so they don't each show their own icon.
//! * `[kyavserver].spout_sender` — set for [`Source::Spout`], removed otherwise.
//! * `[kyavserver].camera_device` — set for [`Source::Camera`], removed otherwise.
//! * `[kyavserver].grab_backend` — on Linux, **always** written from the machine's
//!   [`crate::UserConf::screen_backend`]; the fork's own default is `nvfbc`, so an
//!   absent key silently breaks capture on every non-NVIDIA machine.
//! * `[kyavserver].encoder` — **always** written from the machine's resolved
//!   encoder ([`crate::encoder::resolve`]): which hardware encoder works depends
//!   on the GPU, so a value inherited from a (portable) setup is overridden.
//!
//! [`Directory::defaults`]: crate::Directory::defaults
//! [`Source::Spout`]: crate::Source::Spout

use toml::Value;

use crate::{ScreenBackend, Source, Transmitter, DEFAULT_AUTH_PASSWORD, DEFAULT_AUTH_USERNAME};

#[derive(Debug, thiserror::Error)]
pub enum GenError {
    #[error("config key `{0}` exists in defaults but is not a table")]
    NotATable(&'static str),
    #[error("failed to serialize config: {0}")]
    Serialize(#[from] toml::ser::Error),
}

/// Render the `kyber_config.toml` contents for `tx`, layering its values on a
/// clone of `defaults`.
///
/// `screen_backend` is the machine's Linux capture backend
/// ([`crate::UserConf::screen_backend`]); pass `None` off Linux, where the fork
/// does not compile the key in. `encoder` is the machine's resolved encoder
/// (`x264`, `amf`, `nvenc`, …, see [`crate::encoder::resolve`]).
pub fn render_config(
    tx: &Transmitter,
    defaults: &toml::Table,
    screen_backend: Option<ScreenBackend>,
    encoder: &str,
) -> Result<String, GenError> {
    let mut root = defaults.clone();

    // [kycontroller]: port + transparent default auth
    {
        let kyc = table_mut(&mut root, "kycontroller")?;
        kyc.insert("port".to_string(), Value::Integer(i64::from(tx.port)));

        // kycontroller refuses every connection without a valid login, so an
        // instance with no auth configured would be unreachable. If the operator
        // declared nothing, inject the transparent default basic login.
        if !kyc.contains_key("auth") {
            kyc.insert("auth".to_string(), default_auth());
        }

        // Instances are started, stopped and restarted from the KyberFrog Server
        // tray, so suppress each kycontroller's own tray icon (otherwise every
        // instance clutters the Windows notification area). The operator can
        // force it back on with `tray = true` in [defaults.kycontroller].
        kyc.entry("tray")
            .or_insert_with(|| Value::Boolean(false));
    }

    // [kyavserver]: encoder + source pinning
    {
        let kya = table_mut(&mut root, "kyavserver")?;
        // The encoder is a machine setting, like the capture backend: a setup
        // saved on another machine must not force an encoder this GPU lacks.
        let inherited = kya.insert("encoder".to_string(), Value::String(encoder.to_string()));
        if let Some(old) = inherited.as_ref().and_then(Value::as_str).filter(|old| *old != encoder) {
            log::warn!(
                "Transmitter {:?}: ignoring encoder {old:?} from the setup defaults, \
                 the machine setting resolves to {encoder:?}",
                tx.name
            );
        }

        // Linux screen capture: always write the backend, never let the fork
        // fall back to its own default. That default is `NvFbc`, so an absent
        // key means every non-NVIDIA machine gets a transmitter that starts
        // cleanly and captures nothing. A pinned camera clears it below — the
        // fork routes through lavd and ignores grab_backend there.
        match screen_backend {
            Some(backend) => {
                kya.insert(
                    "grab_backend".to_string(),
                    Value::String(backend.as_str().to_string()),
                );
            }
            // Off Linux the key is meaningless; drop anything inherited so a
            // config written on Linux and reloaded on Windows stays clean.
            None => {
                kya.remove("grab_backend");
            }
        }

        match &tx.source {
            Source::Spout { sender } => {
                kya.insert("spout_sender".to_string(), Value::String(sender.clone()));
                kya.remove("camera_device");
                kya.remove("all_sources");
            }
            Source::Screen {} => {
                // A plain screen grabber must not be pinned to a Spout sender.
                // Which display is captured is decided client-side (kyclient
                // `--display-idx`), not here. The fork default (no `all_sources`)
                // scopes this instance to physical monitors only — Spout senders
                // are not exposed.
                kya.remove("spout_sender");
                kya.remove("camera_device");
                kya.remove("all_sources");
            }
            Source::Camera { device } => {
                // Pin the instance to one capture device (fork lavd iosys) —
                // DirectShow name on Windows, /dev/videoN on Linux; same
                // mechanism as the Spout pin, same device-name CRC.
                kya.insert("camera_device".to_string(), Value::String(device.clone()));
                kya.remove("spout_sender");
                kya.remove("all_sources");
                // A camera takes priority over the grab backend in the fork;
                // leaving the key in would only be misleading.
                kya.remove("grab_backend");
            }
            Source::All {} => {
                // Expose every source (all monitors + all Spout senders). The
                // fork reads `all_sources` to widen the capture scope; no Spout
                // pin so clients pick freely.
                kya.remove("spout_sender");
                kya.remove("camera_device");
                kya.insert("all_sources".to_string(), Value::Boolean(true));
            }
        }
    }

    let header = format!(
        "# Generated by KyberFrog Server — do not edit by hand.\n\
         # Transmitter: {} (port {})\n\
         # Source: {}\n\n",
        tx.name,
        tx.port,
        tx.source.label(),
    );

    Ok(header + &toml::to_string_pretty(&root)?)
}

/// Borrow (creating if absent) a top-level table inside `root`.
fn table_mut<'a>(
    root: &'a mut toml::Table,
    key: &'static str,
) -> Result<&'a mut toml::Table, GenError> {
    root.entry(key)
        .or_insert_with(|| Value::Table(toml::Table::new()))
        .as_table_mut()
        .ok_or(GenError::NotATable(key))
}

/// The `auth` table holding the transparent default basic login (password is
/// stored as the hex SHA-256 of the plaintext, matching kycontroller).
fn default_auth() -> Value {
    let mut login = toml::Table::new();
    login.insert(
        "username".to_string(),
        Value::String(DEFAULT_AUTH_USERNAME.to_string()),
    );
    login.insert(
        "password".to_string(),
        Value::String(sha256_hex(DEFAULT_AUTH_PASSWORD)),
    );

    let mut basic = toml::Table::new();
    basic.insert(
        "logins".to_string(),
        Value::Array(vec![Value::Table(login)]),
    );

    let mut auth = toml::Table::new();
    auth.insert("basic".to_string(), Value::Table(basic));
    Value::Table(auth)
}

/// Hex-encoded SHA-256 of `plaintext` (kycontroller's basic-auth password form).
fn sha256_hex(plaintext: &str) -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(plaintext.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tx_spout() -> Transmitter {
        Transmitter {
            name: "stage-left".to_string(),
            port: 8080,
            source: Source::Spout {
                sender: "Spout Sender".to_string(),
            },
        }
    }

    fn tx_screen() -> Transmitter {
        Transmitter {
            name: "stage-right".to_string(),
            port: 8081,
            source: Source::Screen {},
        }
    }

    fn tx_all() -> Transmitter {
        Transmitter {
            name: "everything".to_string(),
            port: 8082,
            source: Source::All {},
        }
    }

    fn tx_camera() -> Transmitter {
        Transmitter {
            name: "webcam".to_string(),
            port: 8083,
            source: Source::Camera {
                device: "Integrated Camera".to_string(),
            },
        }
    }

    #[test]
    fn spout_pins_sender_and_sets_port() {
        let out = render_config(&tx_spout(), &toml::Table::new(), None, "x264").unwrap();
        let parsed: toml::Table = out.parse().unwrap();
        let kya = parsed["kyavserver"].as_table().unwrap();
        assert_eq!(kya["spout_sender"].as_str(), Some("Spout Sender"));
        assert_eq!(kya["encoder"].as_str(), Some("x264"));
        let kyc = parsed["kycontroller"].as_table().unwrap();
        assert_eq!(kyc["port"].as_integer(), Some(8080));
    }

    #[test]
    fn screen_drops_inherited_spout_sender() {
        let mut defaults = toml::Table::new();
        let mut kya = toml::Table::new();
        kya.insert(
            "spout_sender".to_string(),
            Value::String("Leftover".to_string()),
        );
        defaults.insert("kyavserver".to_string(), Value::Table(kya));

        let out = render_config(&tx_screen(), &defaults, None, "x264").unwrap();
        let parsed: toml::Table = out.parse().unwrap();
        let kya = parsed["kyavserver"].as_table().unwrap();
        assert!(kya.get("spout_sender").is_none());
    }

    #[test]
    fn all_sets_all_sources_and_drops_inherited_spout() {
        // A "Tout envoyer" transmitter must set all_sources and never keep a
        // pinned Spout inherited from defaults.
        let mut defaults = toml::Table::new();
        let mut kya = toml::Table::new();
        kya.insert("spout_sender".to_string(), Value::String("Leftover".to_string()));
        defaults.insert("kyavserver".to_string(), Value::Table(kya));

        let out = render_config(&tx_all(), &defaults, None, "x264").unwrap();
        let parsed: toml::Table = out.parse().unwrap();
        let kya = parsed["kyavserver"].as_table().unwrap();
        assert_eq!(kya["all_sources"].as_bool(), Some(true));
        assert!(kya.get("spout_sender").is_none());
    }

    #[test]
    fn camera_pins_device_and_drops_inherited_keys() {
        // A camera transmitter must pin camera_device and never keep a Spout
        // pin or an all_sources flag inherited from defaults.
        let mut defaults = toml::Table::new();
        let mut kya = toml::Table::new();
        kya.insert("spout_sender".to_string(), Value::String("Leftover".to_string()));
        kya.insert("all_sources".to_string(), Value::Boolean(true));
        defaults.insert("kyavserver".to_string(), Value::Table(kya));

        let out = render_config(&tx_camera(), &defaults, None, "x264").unwrap();
        let parsed: toml::Table = out.parse().unwrap();
        let kya = parsed["kyavserver"].as_table().unwrap();
        assert_eq!(kya["camera_device"].as_str(), Some("Integrated Camera"));
        assert!(kya.get("spout_sender").is_none());
        assert!(kya.get("all_sources").is_none());
    }

    #[test]
    fn screen_drops_inherited_camera_device() {
        let mut defaults = toml::Table::new();
        let mut kya = toml::Table::new();
        kya.insert("camera_device".to_string(), Value::String("Leftover".to_string()));
        defaults.insert("kyavserver".to_string(), Value::Table(kya));

        let out = render_config(&tx_screen(), &defaults, None, "x264").unwrap();
        let parsed: toml::Table = out.parse().unwrap();
        let kya = parsed["kyavserver"].as_table().unwrap();
        assert!(kya.get("camera_device").is_none());
    }

    #[test]
    fn screen_has_no_all_sources_flag() {
        // Screen is the fork default (monitors only) — it must not emit
        // all_sources, and must drop any inherited one.
        let mut defaults = toml::Table::new();
        let mut kya = toml::Table::new();
        kya.insert("all_sources".to_string(), Value::Boolean(true));
        defaults.insert("kyavserver".to_string(), Value::Table(kya));

        let out = render_config(&tx_screen(), &defaults, None, "x264").unwrap();
        let parsed: toml::Table = out.parse().unwrap();
        let kya = parsed["kyavserver"].as_table().unwrap();
        assert!(kya.get("all_sources").is_none());
    }

    #[test]
    fn linux_screen_always_gets_an_explicit_grab_backend() {
        // THE Linux regression guard. The fork defaults grab_backend to nvfbc,
        // so an absent key means a transmitter that starts fine and captures
        // nothing on every non-NVIDIA machine.
        let out = render_config(
            &tx_screen(),
            &toml::Table::new(),
            Some(ScreenBackend::Xcb),
            "x264",
        )
        .unwrap();
        let parsed: toml::Table = out.parse().unwrap();
        assert_eq!(
            parsed["kyavserver"]["grab_backend"].as_str(),
            Some("xcb"),
            "un transmetteur écran Linux doit toujours porter son backend"
        );
    }

    #[test]
    fn machine_backend_overrides_an_inherited_one() {
        // The operator's [defaults.kyavserver] may carry a stale backend from
        // another machine; the machine setting wins.
        let mut defaults = toml::Table::new();
        let mut kya = toml::Table::new();
        kya.insert("grab_backend".to_string(), Value::String("nvfbc".to_string()));
        defaults.insert("kyavserver".to_string(), Value::Table(kya));

        let out = render_config(&tx_screen(), &defaults, Some(ScreenBackend::Wlroots), "x264").unwrap();
        let parsed: toml::Table = out.parse().unwrap();
        assert_eq!(
            parsed["kyavserver"]["grab_backend"].as_str(),
            Some("wlroots")
        );
    }

    #[test]
    fn off_linux_drops_an_inherited_grab_backend() {
        // A config authored on Linux and reloaded on Windows must not keep a
        // key the fork does not compile in there.
        let mut defaults = toml::Table::new();
        let mut kya = toml::Table::new();
        kya.insert("grab_backend".to_string(), Value::String("drm".to_string()));
        defaults.insert("kyavserver".to_string(), Value::Table(kya));

        let out = render_config(&tx_screen(), &defaults, None, "x264").unwrap();
        let parsed: toml::Table = out.parse().unwrap();
        assert!(parsed["kyavserver"].get("grab_backend").is_none());
    }

    #[test]
    fn camera_clears_the_grab_backend_even_on_linux() {
        // The fork routes a pinned camera through lavd and ignores the grab
        // backend; keeping the key would only mislead whoever reads the config.
        let out = render_config(&tx_camera(), &toml::Table::new(), Some(ScreenBackend::Xcb), "x264")
            .unwrap();
        let parsed: toml::Table = out.parse().unwrap();
        let kya = parsed["kyavserver"].as_table().unwrap();
        assert!(kya.get("grab_backend").is_none());
        assert!(kya.get("camera_device").is_some());
    }

    #[test]
    fn machine_encoder_overrides_an_inherited_one() {
        // Setups written before 0.6.0 carry `encoder = "x264"` (the example
        // config did); the machine setting must win or they never get the GPU.
        let mut defaults = toml::Table::new();
        let mut kya = toml::Table::new();
        kya.insert("encoder".to_string(), Value::String("x264".to_string()));
        defaults.insert("kyavserver".to_string(), Value::Table(kya));

        let out = render_config(&tx_spout(), &defaults, None, "amf").unwrap();
        let parsed: toml::Table = out.parse().unwrap();
        assert_eq!(parsed["kyavserver"]["encoder"].as_str(), Some("amf"));
    }

    #[test]
    fn encoder_is_always_written() {
        let out = render_config(&tx_screen(), &toml::Table::new(), None, "nvenc").unwrap();
        let parsed: toml::Table = out.parse().unwrap();
        assert_eq!(parsed["kyavserver"]["encoder"].as_str(), Some("nvenc"));
    }
}
