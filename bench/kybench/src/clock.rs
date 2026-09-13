//! QPC microseconds (the clock txproto, kyproto and VLC use on Windows) and a
//! pacer that sleeps on a high-resolution waitable timer, then yields for the
//! last stretch.

use std::sync::OnceLock;

use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::Performance::{QueryPerformanceCounter, QueryPerformanceFrequency};
use windows::Win32::System::Threading::{
    CreateWaitableTimerExW, SetWaitableTimer, WaitForSingleObject,
    CREATE_WAITABLE_TIMER_HIGH_RESOLUTION, INFINITE,
};

const TIMER_ALL_ACCESS: u32 = 0x001F_0003;
/// Below this remaining time the pacer stops sleeping and yields instead.
const SPIN_US: i64 = 1_500;

fn frequency() -> i64 {
    static FREQ: OnceLock<i64> = OnceLock::new();
    *FREQ.get_or_init(|| {
        let mut f = 0i64;
        unsafe { QueryPerformanceFrequency(&mut f).expect("QueryPerformanceFrequency") };
        f
    })
}

pub fn now_us() -> i64 {
    let mut c = 0i64;
    unsafe { QueryPerformanceCounter(&mut c).expect("QueryPerformanceCounter") };
    (c as i128 * 1_000_000 / frequency() as i128) as i64
}

pub struct Pacer {
    timer: HANDLE,
}

impl Pacer {
    pub fn new() -> windows::core::Result<Self> {
        let timer = unsafe {
            CreateWaitableTimerExW(None, PCWSTR::null(), CREATE_WAITABLE_TIMER_HIGH_RESOLUTION,
                                   TIMER_ALL_ACCESS)?
        };
        Ok(Self { timer })
    }

    pub fn sleep_until(&self, target_us: i64) {
        loop {
            let remaining = target_us - now_us();
            if remaining <= 0 {
                return;
            }
            if remaining > SPIN_US {
                // Relative due time, in 100 ns (negative = relative).
                let due = -(remaining - SPIN_US + 500) * 10;
                unsafe {
                    if SetWaitableTimer(self.timer, &due, 0, None, None, false).is_ok() {
                        WaitForSingleObject(self.timer, INFINITE);
                    }
                }
            } else {
                std::thread::yield_now();
            }
        }
    }
}

impl Drop for Pacer {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.timer);
        }
    }
}
