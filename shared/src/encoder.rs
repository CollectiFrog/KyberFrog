// SPDX-License-Identifier: AGPL-3.0-or-later

//! Video encoder selection for the generated kyavserver configs (#28-1).
//!
//! The encoder is a **machine** setting ([`crate::UserConf::encoder`]): which
//! hardware encoder exists depends on the GPU, so a show saved on an NVIDIA box
//! must not carry `nvenc` onto an AMD one. The default, [`EncoderChoice::Auto`],
//! picks the hardware encoder of the GPU kyavserver actually works on and falls
//! back to x264 on the CPU.
//!
//! Why it matters: on the latency bench (2026-09-15, Spout 1080p60, 20 Mbps)
//! Spout → Spout took 25.8 ms p50 with x264 — 77 % of it in the encoder — and
//! 3.9 ms with `h264_amf`, which kyavservice feeds with the captured D3D11
//! frames directly (no download, no CPU conversion).
//!
//! Which GPU: txproto creates its D3D11 device on the **default adapter** (DXGI
//! adapter 0) for both Spout and screen capture, and derives the hardware
//! encoder from that device. So only the vendor of adapter 0 decides what can
//! work; a second GPU in the machine is irrelevant.

use serde::{Deserialize, Serialize};

/// The operator's encoder setting.
///
/// Serialises to the lowercase string stored in `kyberfrog.toml`; for every
/// variant but `Auto` that is also the fork's `[kyavserver].encoder` value.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EncoderChoice {
    /// Hardware encoder of the primary GPU when KyberFrog knows it works there
    /// (AMF on AMD, NVENC on NVIDIA), else x264.
    #[default]
    Auto,
    /// Software H.264 on the CPU. Works everywhere, slowest.
    X264,
    /// AMD Advanced Media Framework.
    Amf,
    /// NVIDIA NVENC.
    Nvenc,
    /// Intel Quick Sync Video. Offered on Intel GPUs, never picked by `Auto`:
    /// not validated yet.
    Qsv,
}

impl EncoderChoice {
    /// Every choice, in the order the UI lists them.
    pub const ALL: [EncoderChoice; 5] = [
        EncoderChoice::Auto,
        EncoderChoice::X264,
        EncoderChoice::Amf,
        EncoderChoice::Nvenc,
        EncoderChoice::Qsv,
    ];

    /// The fork's `[kyavserver].encoder` string; `None` for `Auto`, which only
    /// becomes a concrete encoder once the GPU is known ([`resolve`]).
    pub fn kyber_name(self) -> Option<&'static str> {
        match self {
            EncoderChoice::Auto => None,
            EncoderChoice::X264 => Some("x264"),
            EncoderChoice::Amf => Some("amf"),
            EncoderChoice::Nvenc => Some("nvenc"),
            EncoderChoice::Qsv => Some("qsv"),
        }
    }

    /// The GPU vendor a hardware choice needs; `None` for x264 and `Auto`.
    pub fn vendor(self) -> Option<GpuVendor> {
        match self {
            EncoderChoice::Amf => Some(GpuVendor::Amd),
            EncoderChoice::Nvenc => Some(GpuVendor::Nvidia),
            EncoderChoice::Qsv => Some(GpuVendor::Intel),
            EncoderChoice::Auto | EncoderChoice::X264 => None,
        }
    }
}

/// GPU vendors KyberFrog knows a hardware encoder for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum GpuVendor {
    Amd,
    Nvidia,
    Intel,
}

impl GpuVendor {
    /// From a PCI vendor id (DXGI `VendorId`).
    pub fn from_pci_id(id: u32) -> Option<GpuVendor> {
        match id {
            0x1002 => Some(GpuVendor::Amd),
            0x10DE => Some(GpuVendor::Nvidia),
            0x8086 => Some(GpuVendor::Intel),
            _ => None,
        }
    }
}

/// The GPU kyavserver captures and encodes on (DXGI adapter 0 on Windows).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct GpuAdapter {
    /// Marketing name, e.g. "AMD Radeon RX 7800 XT".
    pub name: String,
    /// PCI vendor id.
    pub vendor_id: u32,
}

impl GpuAdapter {
    pub fn vendor(&self) -> Option<GpuVendor> {
        GpuVendor::from_pci_id(self.vendor_id)
    }
}

/// The concrete `[kyavserver].encoder` for `choice` on a machine whose primary
/// GPU is `primary` (`None` when unknown, e.g. off Windows).
///
/// An explicit choice is honoured even if the GPU does not match — the UI only
/// offers available encoders, and an operator who writes one by hand into
/// `kyberfrog.toml` gets exactly that.
pub fn resolve(choice: EncoderChoice, primary: Option<&GpuAdapter>) -> &'static str {
    match choice.kyber_name() {
        Some(name) => name,
        None => match primary.and_then(GpuAdapter::vendor) {
            Some(GpuVendor::Amd) => "amf",
            Some(GpuVendor::Nvidia) => "nvenc",
            // QSV is not validated; unknown or no GPU: the CPU always works.
            Some(GpuVendor::Intel) | None => "x264",
        },
    }
}

/// One entry of the encoder picker.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct EncoderOption {
    pub id: EncoderChoice,
    /// Whether it can work on this machine. `Auto` and x264 always can.
    pub available: bool,
}

/// What the UI needs to render the encoder setting.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct EncoderInfo {
    /// The stored setting.
    pub choice: EncoderChoice,
    /// What it resolves to right now (the value written into configs).
    pub resolved: &'static str,
    /// Name of the primary GPU, if known.
    pub gpu: Option<String>,
    pub options: Vec<EncoderOption>,
}

impl EncoderInfo {
    pub fn new(choice: EncoderChoice, primary: Option<&GpuAdapter>) -> EncoderInfo {
        let vendor = primary.and_then(GpuAdapter::vendor);
        EncoderInfo {
            choice,
            resolved: resolve(choice, primary),
            gpu: primary.map(|g| g.name.clone()),
            options: EncoderChoice::ALL
                .iter()
                .map(|&id| EncoderOption {
                    id,
                    available: id.vendor().is_none() || id.vendor() == vendor,
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gpu(vendor_id: u32) -> GpuAdapter {
        GpuAdapter { name: "GPU".to_string(), vendor_id }
    }

    #[test]
    fn auto_picks_the_hardware_encoder_of_the_primary_gpu() {
        assert_eq!(resolve(EncoderChoice::Auto, Some(&gpu(0x1002))), "amf");
        assert_eq!(resolve(EncoderChoice::Auto, Some(&gpu(0x10DE))), "nvenc");
    }

    #[test]
    fn auto_falls_back_to_x264() {
        // Intel: QSV not validated. Unknown vendor or no GPU info: the CPU works.
        assert_eq!(resolve(EncoderChoice::Auto, Some(&gpu(0x8086))), "x264");
        assert_eq!(resolve(EncoderChoice::Auto, Some(&gpu(0x1414))), "x264");
        assert_eq!(resolve(EncoderChoice::Auto, None), "x264");
    }

    #[test]
    fn an_explicit_choice_is_honoured() {
        assert_eq!(resolve(EncoderChoice::X264, Some(&gpu(0x1002))), "x264");
        assert_eq!(resolve(EncoderChoice::Nvenc, Some(&gpu(0x1002))), "nvenc");
    }

    #[test]
    fn names_match_the_fork_enum_and_serde() {
        // kyavservice VideoEncoder: amf, nvenc, qsv, vaapi, x264 (lowercase).
        for choice in EncoderChoice::ALL {
            let serialized = toml::Value::try_from(choice).unwrap();
            let back: EncoderChoice = serialized.clone().try_into().unwrap();
            assert_eq!(back, choice);
            if let Some(name) = choice.kyber_name() {
                assert_eq!(serialized.as_str(), Some(name));
            }
        }
        assert_eq!(toml::Value::try_from(EncoderChoice::Auto).unwrap().as_str(), Some("auto"));
    }

    #[test]
    fn only_the_primary_gpu_vendor_is_available() {
        let info = EncoderInfo::new(EncoderChoice::Auto, Some(&gpu(0x1002)));
        let available: Vec<_> = info.options.iter().filter(|o| o.available).map(|o| o.id).collect();
        assert_eq!(available, [EncoderChoice::Auto, EncoderChoice::X264, EncoderChoice::Amf]);
        assert_eq!(info.resolved, "amf");

        let cpu_only = EncoderInfo::new(EncoderChoice::Auto, None);
        let available: Vec<_> = cpu_only.options.iter().filter(|o| o.available).map(|o| o.id).collect();
        assert_eq!(available, [EncoderChoice::Auto, EncoderChoice::X264]);
    }
}
