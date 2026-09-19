// SPDX-License-Identifier: AGPL-3.0-or-later

//! The graphical-session variables handed to every child, and read to pick the
//! Linux capture backend (`ScreenBackendChoice::Auto`).
//!
//! On Linux the packaged KyberFrog is a systemd *user* service started at
//! login, which can come up before the desktop has pushed `DISPLAY` /
//! `WAYLAND_DISPLAY` into the systemd user environment. Its own environment
//! then lacks them for good — and so would every kyavserver and kyclient it
//! spawns: no window, no capture. So the variables are gathered again at every
//! child start: from our own environment when the session was there when we
//! started, else from `systemctl --user show-environment`, which by then holds
//! what the desktop imported.
//!
//! Off Linux there is nothing to gather.

use std::collections::HashMap;

/// Everything a child needs to reach the graphical session.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
const SESSION_VARS: [&str; 5] = [
    "DISPLAY",
    "WAYLAND_DISPLAY",
    "XAUTHORITY",
    "XDG_SESSION_TYPE",
    "XDG_CURRENT_DESKTOP",
];

/// The session variables to hand a child about to start.
pub fn env() -> HashMap<String, String> {
    imp::env()
}

/// Pick the session variables: `own` (our environment) when it knows the
/// display, else what `systemd` reports. Called lazily, `systemd` only runs
/// when needed.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn pick(
    own: impl Fn(&str) -> Option<String>,
    systemd: impl FnOnce() -> HashMap<String, String>,
) -> HashMap<String, String> {
    let from_own = |name: &&str| own(name).filter(|v| !v.is_empty()).map(|v| (name.to_string(), v));
    let own_vars: HashMap<_, _> = SESSION_VARS.iter().filter_map(from_own).collect();
    if own_vars.contains_key("DISPLAY") || own_vars.contains_key("WAYLAND_DISPLAY") {
        return own_vars;
    }
    let systemd = systemd();
    SESSION_VARS
        .iter()
        .filter_map(|name| {
            systemd
                .get(*name)
                .filter(|v| !v.is_empty())
                .map(|v| (name.to_string(), v.clone()))
        })
        .collect()
}

/// Parse `systemctl --user show-environment` (`NAME=value` lines).
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn parse_environment(text: &str) -> HashMap<String, String> {
    text.lines()
        .filter_map(|line| line.split_once('='))
        .map(|(name, value)| (name.to_string(), value.to_string()))
        .collect()
}

#[cfg(target_os = "linux")]
mod imp {
    use std::collections::HashMap;

    use log::warn;

    pub fn env() -> HashMap<String, String> {
        super::pick(|name| std::env::var(name).ok(), systemd_user_environment)
    }

    fn systemd_user_environment() -> HashMap<String, String> {
        match std::process::Command::new("systemctl")
            .args(["--user", "show-environment"])
            .output()
        {
            Ok(out) if out.status.success() => {
                super::parse_environment(&String::from_utf8_lossy(&out.stdout))
            }
            Ok(out) => {
                warn!("systemctl --user show-environment failed: {}", out.status);
                HashMap::new()
            }
            Err(err) => {
                warn!("systemctl --user show-environment: {err}");
                HashMap::new()
            }
        }
    }
}

#[cfg(not(target_os = "linux"))]
mod imp {
    use std::collections::HashMap;

    pub fn env() -> HashMap<String, String> {
        HashMap::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    #[test]
    fn own_environment_wins_when_it_knows_the_display() {
        let own = vars(&[("DISPLAY", ":0"), ("XDG_SESSION_TYPE", "x11"), ("HOME", "/home/vj")]);
        let got = pick(|k| own.get(k).cloned(), || panic!("systemd must not be asked"));
        assert_eq!(got, vars(&[("DISPLAY", ":0"), ("XDG_SESSION_TYPE", "x11")]));
    }

    #[test]
    fn started_before_the_desktop_falls_back_to_systemd() {
        // The race this module exists for: the service came up with no display.
        let got = pick(
            |_| None,
            || parse_environment("HOME=/home/vj\nDISPLAY=:0\nXAUTHORITY=/home/vj/.Xauthority\nXDG_SESSION_TYPE=x11\n"),
        );
        assert_eq!(
            got,
            vars(&[
                ("DISPLAY", ":0"),
                ("XAUTHORITY", "/home/vj/.Xauthority"),
                ("XDG_SESSION_TYPE", "x11"),
            ])
        );
    }

    #[test]
    fn no_session_anywhere_gives_nothing() {
        // Headless box: Auto then resolves to drm.
        assert!(pick(|_| None, HashMap::new).is_empty());
    }

    #[test]
    fn empty_values_count_as_absent() {
        let got = pick(
            |k| (k == "DISPLAY").then(String::new),
            || vars(&[("WAYLAND_DISPLAY", "wayland-0")]),
        );
        assert_eq!(got, vars(&[("WAYLAND_DISPLAY", "wayland-0")]));
    }
}
