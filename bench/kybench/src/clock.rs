//! QPC microseconds (the clock txproto, kyproto and VLC use on Windows) and a
//! pacer that sleeps on a high-resolution waitable timer, then spins for the
//! last stretch. Spinning, not yielding: with a Kyber pipeline running,
//! `SwitchToThread` handed the core away and the probe detected frames
//! 0.6–0.9 ms late (observed, step 2 dry run).

use std::sync::OnceLock;

use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::Performance::{QueryPerformanceCounter, QueryPerformanceFrequency};
use windows::Win32::System::Threading::{
    CreateWaitableTimerExW, GetCurrentThread, SetThreadPriority, SetWaitableTimer,
    WaitForSingleObject, CREATE_WAITABLE_TIMER_HIGH_RESOLUTION, INFINITE,
    THREAD_PRIORITY_TIME_CRITICAL,
};

const TIMER_ALL_ACCESS: u32 = 0x001F_0003;
/// Below this remaining time the pacer stops sleeping and spins instead.
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
    /// Also raises the calling thread to `THREAD_PRIORITY_TIME_CRITICAL` (normal
    /// priority class): the instrument must not be preempted by the threads
    /// the measured pipeline wakes up on the very frame it is waiting for
    /// (observed: 9 % of detections 0.6–0.9 ms late with kyavserver reading
    /// the same sender). `--priority normal` leaves the thread alone.
    pub fn new() -> windows::core::Result<Self> {
        let timer = unsafe {
            CreateWaitableTimerExW(None, PCWSTR::null(), CREATE_WAITABLE_TIMER_HIGH_RESOLUTION,
                                   TIMER_ALL_ACCESS)?
        };
        if std::env::args().skip(1).collect::<Vec<_>>().windows(2)
            .all(|w| !(w[0] == "--priority" && w[1] == "normal"))
        {
            unsafe {
                let _ = SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_TIME_CRITICAL);
            }
        }
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
                std::hint::spin_loop();
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
