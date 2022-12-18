use std::path::PathBuf;

pub fn open_tape_dialog() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter("TAPE", &["tap"])
        .set_title("Open TAP file")
        .pick_file()
}

pub fn save_tape_dialog() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter("TAPE", &["tap"])
        .set_title("Create a new TAP file")
        .save_file()
}

#[cfg(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "netbsd",
    target_os = "openbsd"
))]
pub mod posix_menus;
#[cfg(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "netbsd",
    target_os = "openbsd"
))]
pub use posix_menus as menus;

#[cfg(not(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "netbsd",
    target_os = "openbsd"
)))]
pub mod native_menus;
#[cfg(not(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "netbsd",
    target_os = "openbsd"
)))]
pub use native_menus as menus;

#[cfg(not(windows))]
pub fn set_dpi_awareness() -> Result<(), String> { Ok(()) }

#[cfg(windows)]
pub fn set_dpi_awareness() -> Result<(), String> {
    use winapi::{shared::winerror::{E_INVALIDARG, S_OK},
                 um::shellscalingapi::{GetProcessDpiAwareness, SetProcessDpiAwareness, PROCESS_DPI_UNAWARE,
                                       PROCESS_PER_MONITOR_DPI_AWARE}};

    match unsafe { SetProcessDpiAwareness(PROCESS_PER_MONITOR_DPI_AWARE) } {
        S_OK => Ok(()),
        E_INVALIDARG => Err("Could not set DPI awareness.".into()),
        _ => {
            let mut awareness = PROCESS_DPI_UNAWARE;
            match unsafe { GetProcessDpiAwareness(std::ptr::null_mut(), &mut awareness) } {
                S_OK if awareness == PROCESS_PER_MONITOR_DPI_AWARE => Ok(()),
                _ => Err("Please disable DPI awareness override in program properties.".into()),
            }
        },
    }
}

#[macro_export]
macro_rules! total_ticks_of {
    ($spectrum:ident, $ula:ty) => {
        $spectrum.ula.current_frame() *
            <$ula as Video>::VideoFrame::FRAME_TSTATES_COUNT as u64 +
            $spectrum.ula.current_tstate() as u64
    };
}

#[macro_export]
macro_rules! measure_ticks_start {
    ($time:ident, $dur:ident, $ticks:ident, $spectrum:ident, $ula:ty) => {
        let mut $time = std::time::Instant::now();
        let mut $ticks = total_ticks_of!($spectrum, $ula);
        let mut $dur = std::time::Duration::ZERO;
    };
}

#[macro_export]
macro_rules! measure_ticks {
    ($time:ident, $dur:ident, $ticks:ident, $spectrum:ident, $ula:ty) => {
        {
            const SECOND: std::time::Duration = std::time::Duration::from_secs(1);
            let time_end = std::time::Instant::now();
            $dur += time_end.duration_since($time);
            $time = time_end;
            if $dur >= SECOND {
                let ticks_end = total_ticks_of!($spectrum, $ula);
                let delta_ticks = ticks_end - $ticks;
                $ticks = ticks_end;
                println!("CPU MHz: {:10.04}",
                    delta_ticks as f64 / $dur.as_secs_f64() / 1.0e6);
                $dur = std::time::Duration::ZERO;
            }
        }
    };
}
