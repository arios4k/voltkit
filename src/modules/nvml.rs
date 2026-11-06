//! A small binding to NVIDIA's management library (NVML), loaded at run time so
//! Voltkit starts, and says why, on machines without the NVIDIA driver. It reads
//! sensors and, as root, changes power limits, fans, clock offsets and
//! temperature thresholds: the same calls `nvidia-smi` makes.
//!
//! Every call reports the driver's own verdict, so what a card supports is
//! learned by asking the driver rather than guessed from its model name.

use std::ffi::{CStr, CString, c_char, c_int, c_uint, c_ulonglong, c_void};
use std::fmt;

use libloading::Library;

use super::gpu::Sample;

/// NVML's return codes that matter here.
const SUCCESS: c_int = 0;
const NOT_SUPPORTED: c_int = 3;
const NO_PERMISSION: c_int = 4;
const FUNCTION_NOT_FOUND: c_int = 13;

#[derive(Debug, Clone, PartialEq)]
pub enum NvmlError {
    /// No NVIDIA management library on this computer.
    Missing,
    /// This card or driver doesn't offer the call.
    Unsupported,
    /// The driver is older than the call.
    TooOld,
    /// The call needs administrator rights.
    Permission,
    Other(String),
}

impl fmt::Display for NvmlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use crate::i18n::tr;
        match self {
            NvmlError::Missing => f.write_str(tr("NVIDIA's management library isn't installed.")),
            NvmlError::Unsupported => f.write_str(tr("The driver doesn't allow this on this card.")),
            NvmlError::TooOld => f.write_str(tr("The installed NVIDIA driver is too old for this.")),
            NvmlError::Permission => f.write_str(tr("This needs administrator permission.")),
            NvmlError::Other(m) => f.write_str(m),
        }
    }
}

impl std::error::Error for NvmlError {}

pub type Result<T> = std::result::Result<T, NvmlError>;

/// Which temperature NVML sets or reads (`nvmlTemperatureThresholds_t`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Threshold {
    /// The temperature the card's own fan control aims for.
    AcousticTarget = 5,
}

#[repr(C)]
#[derive(Default)]
struct Utilization {
    gpu: c_uint,
    memory: c_uint,
}

#[repr(C)]
#[derive(Default)]
struct MemoryInfo {
    total: c_ulonglong,
    free: c_ulonglong,
    used: c_ulonglong,
}

type Device = *mut c_void;

pub struct Nvml {
    lib: Library,
}

impl Nvml {
    pub fn open() -> Result<Self> {
        let lib = ["libnvidia-ml.so.1", "libnvidia-ml.so"]
            .into_iter()
            // SAFETY: loading NVIDIA's own library, whose initializers are plain.
            .find_map(|name| unsafe { Library::new(name).ok() })
            .ok_or(NvmlError::Missing)?;
        let nvml = Nvml { lib };
        // SAFETY: nvmlInit_v2 takes no arguments.
        let code = unsafe { nvml.func::<unsafe extern "C" fn() -> c_int>(b"nvmlInit_v2\0")?() };
        nvml.check(code)?;
        Ok(nvml)
    }

    fn func<T: Copy>(&self, name: &[u8]) -> Result<T> {
        // SAFETY: each caller names the symbol with the signature NVML documents.
        unsafe { self.lib.get::<T>(name).map(|s| *s).map_err(|_| NvmlError::TooOld) }
    }

    fn check(&self, code: c_int) -> Result<()> {
        match code {
            SUCCESS => Ok(()),
            NOT_SUPPORTED => Err(NvmlError::Unsupported),
            NO_PERMISSION => Err(NvmlError::Permission),
            FUNCTION_NOT_FOUND => Err(NvmlError::TooOld),
            other => {
                let text = self
                    .func::<unsafe extern "C" fn(c_int) -> *const c_char>(b"nvmlErrorString\0")
                    .ok()
                    // SAFETY: nvmlErrorString returns a static C string.
                    .map(|f| unsafe { CStr::from_ptr(f(other)).to_string_lossy().into_owned() })
                    .unwrap_or_else(|| format!("NVML error {other}"));
                Err(NvmlError::Other(text))
            }
        }
    }

    /// The card at a PCI address such as `0000:2d:00.0`.
    pub fn device(&self, address: &str) -> Result<Device> {
        let id = CString::new(address.to_ascii_uppercase()).map_err(|_| NvmlError::Other("bad address".into()))?;
        let mut device: Device = std::ptr::null_mut();
        let f = self.func::<unsafe extern "C" fn(*const c_char, *mut Device) -> c_int>(b"nvmlDeviceGetHandleByPciBusId_v2\0")?;
        // SAFETY: `id` is a valid C string and `device` a valid out pointer.
        self.check(unsafe { f(id.as_ptr(), &mut device) })?;
        Ok(device)
    }

    fn get_uint(&self, name: &[u8], device: Device) -> Result<u32> {
        let f = self.func::<unsafe extern "C" fn(Device, *mut c_uint) -> c_int>(name)?;
        let mut v: c_uint = 0;
        // SAFETY: valid handle and out pointer.
        self.check(unsafe { f(device, &mut v) })?;
        Ok(v)
    }

    fn get_uint_with(&self, name: &[u8], device: Device, arg: c_uint) -> Result<u32> {
        let f = self.func::<unsafe extern "C" fn(Device, c_uint, *mut c_uint) -> c_int>(name)?;
        let mut v: c_uint = 0;
        // SAFETY: valid handle and out pointer.
        self.check(unsafe { f(device, arg, &mut v) })?;
        Ok(v)
    }

    // ---- Reading

    /// A reading of the sensors; what the card doesn't report stays `None`.
    #[allow(clippy::field_reassign_with_default)]
    pub fn sample(&self, address: &str) -> Result<Sample> {
        let d = self.device(address)?;
        let mut s = Sample::default();
        s.temp_c = self.get_uint_with(b"nvmlDeviceGetTemperature\0", d, 0).ok().map(f64::from);
        s.power_w = self.get_uint(b"nvmlDeviceGetPowerUsage\0", d).ok().map(|mw| f64::from(mw) / 1000.0);
        s.limit_w = self.get_uint(b"nvmlDeviceGetPowerManagementLimit\0", d).ok().map(|mw| f64::from(mw) / 1000.0);
        // Some cards answer the range with nonsense; it only counts when plausible
        // and the current limit is readable too.
        if let Ok((min, max)) = self.power_limit_range(address)
            && max > min
            && max <= 1500.0
            && s.limit_w.is_some()
        {
            s.limit_min_w = Some(min);
            s.limit_max_w = Some(max);
        }
        s.core_mhz = self.get_uint_with(b"nvmlDeviceGetClockInfo\0", d, 0).ok().map(f64::from);
        s.mem_mhz = self.get_uint_with(b"nvmlDeviceGetClockInfo\0", d, 2).ok().map(f64::from);
        s.fan_pct = self.get_uint_with(b"nvmlDeviceGetFanSpeed_v2\0", d, 0).ok().map(f64::from);
        if let Ok(f) = self.func::<unsafe extern "C" fn(Device, *mut Utilization) -> c_int>(b"nvmlDeviceGetUtilizationRates\0") {
            let mut u = Utilization::default();
            // SAFETY: valid handle and out pointer.
            if unsafe { f(d, &mut u) } == SUCCESS {
                s.util_pct = Some(f64::from(u.gpu));
            }
        }
        if let Ok(f) = self.func::<unsafe extern "C" fn(Device, *mut MemoryInfo) -> c_int>(b"nvmlDeviceGetMemoryInfo\0") {
            let mut m = MemoryInfo::default();
            // SAFETY: valid handle and out pointer.
            if unsafe { f(d, &mut m) } == SUCCESS {
                s.vram_used_mb = Some(m.used as f64 / (1024.0 * 1024.0));
                s.vram_total_mb = Some(m.total as f64 / (1024.0 * 1024.0));
            }
        }
        Ok(s)
    }

    /// The adjustable power limit range in watts.
    pub fn power_limit_range(&self, address: &str) -> Result<(f64, f64)> {
        let d = self.device(address)?;
        let f = self.func::<unsafe extern "C" fn(Device, *mut c_uint, *mut c_uint) -> c_int>(b"nvmlDeviceGetPowerManagementLimitConstraints\0")?;
        let (mut min, mut max): (c_uint, c_uint) = (0, 0);
        // SAFETY: valid handle and out pointers.
        self.check(unsafe { f(d, &mut min, &mut max) })?;
        Ok((f64::from(min) / 1000.0, f64::from(max) / 1000.0))
    }

    /// The power limit now, in watts.
    pub fn power_limit(&self, address: &str) -> Result<f64> {
        self.get_uint(b"nvmlDeviceGetPowerManagementLimit\0", self.device(address)?).map(|mw| f64::from(mw) / 1000.0)
    }

    pub fn fan_count(&self, address: &str) -> Result<u32> {
        self.get_uint(b"nvmlDeviceGetNumFans\0", self.device(address)?)
    }

    /// The lowest and highest fan speed the driver accepts, in percent.
    pub fn fan_range(&self, address: &str) -> Result<(u32, u32)> {
        let d = self.device(address)?;
        let f = self.func::<unsafe extern "C" fn(Device, *mut c_uint, *mut c_uint) -> c_int>(b"nvmlDeviceGetMinMaxFanSpeed\0")?;
        let (mut min, mut max): (c_uint, c_uint) = (0, 0);
        // SAFETY: valid handle and out pointers.
        self.check(unsafe { f(d, &mut min, &mut max) })?;
        Ok((min, max))
    }

    pub fn clock_offset(&self, address: &str, memory: bool) -> Result<i32> {
        let d = self.device(address)?;
        let name: &[u8] = if memory { b"nvmlDeviceGetMemClkVfOffset\0" } else { b"nvmlDeviceGetGpcClkVfOffset\0" };
        let f = self.func::<unsafe extern "C" fn(Device, *mut c_int) -> c_int>(name)?;
        let mut v: c_int = 0;
        // SAFETY: valid handle and out pointer.
        self.check(unsafe { f(d, &mut v) })?;
        Ok(v)
    }

    pub fn threshold(&self, address: &str, which: Threshold) -> Result<u32> {
        self.get_uint_with(b"nvmlDeviceGetTemperatureThreshold\0", self.device(address)?, which as c_uint)
    }

    // ---- Changing (as root)

    pub fn set_power_limit(&self, address: &str, watts: f64) -> Result<()> {
        let d = self.device(address)?;
        let f = self.func::<unsafe extern "C" fn(Device, c_uint) -> c_int>(b"nvmlDeviceSetPowerManagementLimit\0")?;
        // SAFETY: valid handle.
        self.check(unsafe { f(d, (watts * 1000.0).round() as c_uint) })
    }

    pub fn set_fan_speed(&self, address: &str, fan: u32, percent: u32) -> Result<()> {
        let d = self.device(address)?;
        let f = self.func::<unsafe extern "C" fn(Device, c_uint, c_uint) -> c_int>(b"nvmlDeviceSetFanSpeed_v2\0")?;
        // SAFETY: valid handle.
        self.check(unsafe { f(d, fan, percent) })
    }

    /// Gives the fan back to the card's own control.
    pub fn set_fan_auto(&self, address: &str, fan: u32) -> Result<()> {
        let d = self.device(address)?;
        let f = self.func::<unsafe extern "C" fn(Device, c_uint) -> c_int>(b"nvmlDeviceSetDefaultFanSpeed_v2\0")?;
        // SAFETY: valid handle.
        self.check(unsafe { f(d, fan) })
    }

    pub fn set_clock_offset(&self, address: &str, memory: bool, mhz: i32) -> Result<()> {
        let d = self.device(address)?;
        let name: &[u8] = if memory { b"nvmlDeviceSetMemClkVfOffset\0" } else { b"nvmlDeviceSetGpcClkVfOffset\0" };
        let f = self.func::<unsafe extern "C" fn(Device, c_int) -> c_int>(name)?;
        // SAFETY: valid handle.
        self.check(unsafe { f(d, mhz) })
    }

    pub fn set_threshold(&self, address: &str, which: Threshold, celsius: i32) -> Result<()> {
        let d = self.device(address)?;
        let f = self.func::<unsafe extern "C" fn(Device, c_uint, *mut c_int) -> c_int>(b"nvmlDeviceSetTemperatureThreshold\0")?;
        let mut value: c_int = celsius;
        // SAFETY: valid handle and in/out pointer.
        self.check(unsafe { f(d, which as c_uint, &mut value) })
    }
}

impl Drop for Nvml {
    fn drop(&mut self) {
        if let Ok(f) = self.func::<unsafe extern "C" fn() -> c_int>(b"nvmlShutdown\0") {
            // SAFETY: nvmlShutdown takes no arguments.
            unsafe { f() };
        }
    }
}
