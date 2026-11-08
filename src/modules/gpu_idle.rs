//! Why a card won't idle: on Linux the video memory often stays at its highest
//! clock with nothing running, and the card draws tens of watts for nothing. The
//! usual cause is the displays: the driver can only drop the memory clock inside
//! the short gap between two screen refreshes, and with several monitors, mixed
//! refresh rates or a very fast panel that gap is too short, so it doesn't try.
//!
//! This reads the connected displays from the kernel (their EDID, the monitor's
//! own description of its preferred mode) and, given a reading of the idle card,
//! names the likely causes. It says *likely*: the refresh rate is the monitor's
//! preferred one, and the one in use can differ.

use std::path::Path;

use super::gpu::Sample;

/// Memory at or above this with nothing running counts as stuck high. Idle
/// memory sits well below it on current NVIDIA, AMD and Intel cards.
pub const STUCK_MEM_MHZ: f64 = 900.0;
/// The card is idle below this much use.
pub const IDLE_UTIL_PCT: f64 = 10.0;
/// A refresh rate at or above this makes the gap between refreshes short.
pub const FAST_HZ: f64 = 119.0;

#[derive(Clone, Debug, PartialEq)]
pub struct Display {
    /// The connector, such as `DP-1` or `HDMI-A-2`.
    pub connector: String,
    pub width: u32,
    pub height: u32,
    /// The refresh rate of the monitor's preferred mode.
    pub hz: f64,
}

/// The monitor's preferred mode from its EDID: the first detailed timing, bytes 54 to 71.
pub fn parse_edid(edid: &[u8]) -> Option<(u32, u32, f64)> {
    if edid.len() < 72 || edid[..8] != [0, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0] {
        return None;
    }
    let d = &edid[54..72];
    let clock_hz = f64::from(u16::from_le_bytes([d[0], d[1]])) * 10_000.0;
    let (ha, hb) = (u32::from(d[2]) | (u32::from(d[4] & 0xf0) << 4), u32::from(d[3]) | (u32::from(d[4] & 0x0f) << 8));
    let (va, vb) = (u32::from(d[5]) | (u32::from(d[7] & 0xf0) << 4), u32::from(d[6]) | (u32::from(d[7] & 0x0f) << 8));
    let total = f64::from((ha + hb) * (va + vb));
    (clock_hz > 0.0 && ha > 0 && va > 0 && total > 0.0).then_some((ha, va, clock_hz / total))
}

/// The displays connected to the card whose sysfs directory is `device`.
pub fn read_displays(drm_root: &Path, device: &Path) -> Vec<Display> {
    let mine = std::fs::canonicalize(device).ok();
    let Ok(dir) = std::fs::read_dir(drm_root) else { return Vec::new() };
    let mut out: Vec<Display> = dir
        .flatten()
        .filter_map(|e| {
            let path = e.path();
            let name = e.file_name().to_string_lossy().into_owned();
            let connector = name.strip_prefix("card")?.split_once('-')?.1.to_owned();
            if std::fs::read_to_string(path.join("status")).ok()?.trim() != "connected" {
                return None;
            }
            if std::fs::canonicalize(path.join("device")).ok() != mine {
                return None;
            }
            let (width, height, hz) = parse_edid(&std::fs::read(path.join("edid")).ok()?)?;
            Some(Display { connector, width, height, hz })
        })
        .collect();
    out.sort_by(|a, b| a.connector.cmp(&b.connector));
    out
}

/// A likely reason the memory clock can't drop.
#[derive(Clone, Debug, PartialEq)]
pub enum Cause {
    /// More than one display is on.
    SeveralDisplays(usize),
    /// The displays refresh at different rates.
    MixedRefresh,
    /// One display refreshes very fast (the rate).
    FastRefresh(f64),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Verdict {
    /// The reading doesn't include the clocks, so nothing can be said.
    Unknown,
    /// Something is using the card; check again when the desktop is quiet.
    Busy,
    /// Idle, with the memory clock where it should be.
    Fine,
    /// Idle, with the memory clock high, and the likely causes (possibly none found).
    Stuck(Vec<Cause>),
}

pub fn diagnose(sample: &Sample, displays: &[Display]) -> Verdict {
    let (Some(util), Some(mem)) = (sample.util_pct, sample.mem_mhz) else { return Verdict::Unknown };
    if util > IDLE_UTIL_PCT {
        return Verdict::Busy;
    }
    if mem < STUCK_MEM_MHZ {
        return Verdict::Fine;
    }
    let mut causes = Vec::new();
    if displays.len() > 1 {
        causes.push(Cause::SeveralDisplays(displays.len()));
        let (lo, hi) = displays.iter().fold((f64::MAX, 0.0_f64), |(lo, hi), d| (lo.min(d.hz), hi.max(d.hz)));
        if hi - lo > 1.0 {
            causes.push(Cause::MixedRefresh);
        }
    }
    if let Some(fast) = displays.iter().map(|d| d.hz).filter(|hz| *hz >= FAST_HZ).reduce(f64::max) {
        causes.push(Cause::FastRefresh(fast));
    }
    Verdict::Stuck(causes)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An EDID whose first timing is `ha`x`va` with the given blanking and pixel clock (10 kHz units).
    fn edid(clock: u16, ha: u32, hb: u32, va: u32, vb: u32) -> Vec<u8> {
        let mut e = vec![0u8; 128];
        e[..8].copy_from_slice(&[0, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0]);
        e[54..56].copy_from_slice(&clock.to_le_bytes());
        e[56] = (ha & 0xff) as u8;
        e[57] = (hb & 0xff) as u8;
        e[58] = (((ha >> 8) << 4) | (hb >> 8)) as u8;
        e[59] = (va & 0xff) as u8;
        e[60] = (vb & 0xff) as u8;
        e[61] = (((va >> 8) << 4) | (vb >> 8)) as u8;
        e
    }

    fn display(name: &str, hz: f64) -> Display {
        Display { connector: name.into(), width: 2560, height: 1440, hz }
    }

    fn idle(mem: f64) -> Sample {
        Sample { util_pct: Some(2.0), mem_mhz: Some(mem), ..Sample::default() }
    }

    #[test]
    fn it_reads_the_preferred_mode_from_an_edid() {
        let (w, h, hz) = parse_edid(&edid(14850, 1920, 280, 1080, 45)).unwrap();
        assert_eq!((w, h), (1920, 1080));
        assert!((hz - 60.0).abs() < 0.01);
        let (w, h, hz) = parse_edid(&edid(35400, 2560, 80, 1440, 33)).unwrap();
        assert_eq!((w, h), (2560, 1440));
        assert!((hz - 91.0).abs() < 1.0, "{hz}");
        assert!(parse_edid(&[0; 128]).is_none(), "no EDID header");
        assert!(parse_edid(&edid(0, 1920, 280, 1080, 45)).is_none(), "no pixel clock");
        assert!(parse_edid(&[1, 2, 3]).is_none());
    }

    #[test]
    fn it_lists_only_connected_displays_of_the_card() {
        let root = std::env::temp_dir().join(format!("voltkit-idle-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let (mine, other) = (root.join("pci/0000:03:00.0"), root.join("pci/0000:0a:00.0"));
        let drm = root.join("drm");
        for (conn, status, device) in [("card1-DP-1", "connected", &mine), ("card1-HDMI-A-1", "disconnected", &mine), ("card2-DP-1", "connected", &other)] {
            let dir = drm.join(conn);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::create_dir_all(device).unwrap();
            std::fs::write(dir.join("status"), status).unwrap();
            std::fs::write(dir.join("edid"), edid(14850, 1920, 280, 1080, 45)).unwrap();
            std::os::unix::fs::symlink(device, dir.join("device")).unwrap();
        }
        std::fs::create_dir_all(drm.join("card1")).unwrap();
        let found = read_displays(&drm, &mine);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].connector, "DP-1");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn it_names_the_likely_causes_of_a_stuck_memory_clock() {
        let verdict = diagnose(&idle(1750.0), &[display("DP-1", 144.0), display("HDMI-A-1", 60.0)]);
        assert_eq!(verdict, Verdict::Stuck(vec![Cause::SeveralDisplays(2), Cause::MixedRefresh, Cause::FastRefresh(144.0)]));
        assert_eq!(diagnose(&idle(1750.0), &[display("DP-1", 60.0)]), Verdict::Stuck(vec![]));
        assert_eq!(diagnose(&idle(1750.0), &[display("DP-1", 165.0)]), Verdict::Stuck(vec![Cause::FastRefresh(165.0)]));
        let same = diagnose(&idle(1750.0), &[display("DP-1", 60.0), display("DP-2", 60.0)]);
        assert_eq!(same, Verdict::Stuck(vec![Cause::SeveralDisplays(2)]));
    }

    #[test]
    fn a_busy_or_quiet_card_is_not_diagnosed() {
        assert_eq!(diagnose(&idle(405.0), &[display("DP-1", 144.0)]), Verdict::Fine);
        let busy = Sample { util_pct: Some(60.0), mem_mhz: Some(1750.0), ..Sample::default() };
        assert_eq!(diagnose(&busy, &[]), Verdict::Busy);
        assert_eq!(diagnose(&Sample::default(), &[]), Verdict::Unknown);
    }
}
