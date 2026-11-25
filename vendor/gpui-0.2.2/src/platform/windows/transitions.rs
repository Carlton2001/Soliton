//! Temporarily paint solid backgrounds during native window-state animations.
use std::time::{Duration, Instant};

use util::ResultExt;
use windows::Win32::{
    Foundation::{HWND, LPARAM, WPARAM},
    Graphics::Gdi::InvalidateRect,
    UI::WindowsAndMessaging::{
        ANIMATIONINFO, IsIconic, IsWindowVisible, KillTimer, PostMessageW, SIZE_MAXIMIZED,
        SIZE_MINIMIZED, SIZE_RESTORED, SPI_GETANIMATION, SetTimer, SystemParametersInfoW,
        WM_SYSCOMMAND,
    },
};

use crate::{WindowBackgroundAppearance, WindowsWindowInner};

pub(super) const OPACITY_TIMER_ID: usize = 2;
// DWM exposes the state change but no completion notification for this animation.
const TRANSITION_MILLIS: u32 = 240;
const FADE_DURATION: Duration = Duration::from_millis(150);
const FADE_FRAME_MILLIS: u32 = 16;
const MAX_FADE_STEP: Duration = Duration::from_millis(32);

pub(super) fn post_window_state_command(hwnd: HWND, command: u32) {
    unsafe {
        PostMessageW(
            Some(hwnd),
            WM_SYSCOMMAND,
            WPARAM(command as usize),
            LPARAM(0),
        )
    }
    .log_err();
}

#[derive(Default)]
pub(super) struct WindowTransition {
    size_state: u32,
    opacity: TransitionOpacity,
}

#[derive(Clone, Copy, Default)]
enum TransitionOpacity {
    #[default]
    Configured,
    OpaqueUntil(Instant),
    Fading {
        elapsed: Duration,
        last_frame: Instant,
    },
}

impl WindowTransition {
    pub(super) fn opacity_override(&self) -> f32 {
        match self.opacity {
            TransitionOpacity::Configured => 0.,
            TransitionOpacity::OpaqueUntil(_) => 1.,
            TransitionOpacity::Fading { elapsed, .. } => {
                let progress = elapsed.as_secs_f32() / FADE_DURATION.as_secs_f32();
                1. - progress * progress * (3. - 2. * progress)
            }
        }
    }
}

impl WindowsWindowInner {
    pub(super) fn start_opaque_transition(&self, hwnd: HWND) -> bool {
        if self.state.borrow().background_appearance != WindowBackgroundAppearance::Blurred
            || !unsafe { IsWindowVisible(hwnd) }.as_bool()
            || !window_animations_enabled()
        {
            self.clear_opaque_transition(hwnd);
            return false;
        }
        let deadline = Instant::now() + Duration::from_millis(u64::from(TRANSITION_MILLIS));
        if unsafe { SetTimer(Some(hwnd), OPACITY_TIMER_ID, TRANSITION_MILLIS, None) } == 0 {
            self.clear_opaque_transition(hwnd);
            return false;
        }

        let previous = std::mem::replace(
            &mut self.state.borrow_mut().transition.opacity,
            TransitionOpacity::OpaqueUntil(deadline),
        );
        let was_translucent = !matches!(previous, TransitionOpacity::OpaqueUntil(_));
        if was_translucent {
            log::trace!("Painting opaque backgrounds for the window transition");
        }
        was_translucent
    }

    pub(super) fn transition_size_changed(&self, hwnd: HWND, size_state: u32) {
        if !matches!(size_state, SIZE_RESTORED | SIZE_MINIMIZED | SIZE_MAXIMIZED) {
            return;
        }
        let previous = std::mem::replace(
            &mut self.state.borrow_mut().transition.size_state,
            size_state,
        );
        if previous != size_state && size_state != SIZE_MINIMIZED {
            // Some shell commands change the window state without sending WM_SYSCOMMAND.
            if self.start_opaque_transition(hwnd) {
                unsafe { InvalidateRect(Some(hwnd), None, false).ok().log_err() };
            }
        }
    }

    pub(super) fn advance_opacity_transition(&self, hwnd: HWND) {
        let now = Instant::now();
        let opacity = self.state.borrow().transition.opacity;
        if matches!(opacity, TransitionOpacity::Configured) {
            return;
        }
        if unsafe { IsIconic(hwnd) }.as_bool() {
            // Keep the opaque surface available for the restore animation.
            self.state.borrow_mut().transition.opacity = TransitionOpacity::OpaqueUntil(now);
            let _ = unsafe { KillTimer(Some(hwnd), OPACITY_TIMER_ID) };
            return;
        }

        match opacity {
            TransitionOpacity::Configured => return,
            TransitionOpacity::OpaqueUntil(deadline) if now < deadline => return,
            TransitionOpacity::OpaqueUntil(_) => {
                // Let the compositor present the first opaque frame before advancing the fade.
                self.present_window_frame();
                if unsafe { SetTimer(Some(hwnd), OPACITY_TIMER_ID, FADE_FRAME_MILLIS, None) } == 0 {
                    self.clear_opaque_transition(hwnd);
                } else {
                    self.state.borrow_mut().transition.opacity = TransitionOpacity::Fading {
                        elapsed: Duration::ZERO,
                        last_frame: Instant::now(),
                    };
                    log::trace!("Fading to the configured background opacity");
                    return;
                }
            }
            TransitionOpacity::Fading {
                elapsed,
                last_frame,
            } => {
                // A delayed frame may stretch the fade, but cannot skip it.
                let elapsed = elapsed + (now - last_frame).min(MAX_FADE_STEP);
                if elapsed >= FADE_DURATION {
                    self.clear_opaque_transition(hwnd);
                } else {
                    self.state.borrow_mut().transition.opacity = TransitionOpacity::Fading {
                        elapsed,
                        last_frame: now,
                    };
                }
            }
        }
        self.present_window_frame();
    }

    fn clear_opaque_transition(&self, hwnd: HWND) {
        let previous = std::mem::take(&mut self.state.borrow_mut().transition.opacity);
        if !matches!(previous, TransitionOpacity::Configured) {
            let _ = unsafe { KillTimer(Some(hwnd), OPACITY_TIMER_ID) };
            log::trace!("Restored the configured background opacity");
        }
    }
}

fn window_animations_enabled() -> bool {
    let mut info = ANIMATIONINFO {
        cbSize: size_of::<ANIMATIONINFO>() as u32,
        iMinAnimate: 0,
    };
    unsafe {
        SystemParametersInfoW(
            SPI_GETANIMATION,
            info.cbSize,
            Some((&mut info as *mut ANIMATIONINFO).cast()),
            Default::default(),
        )
    }
    .is_ok()
        && info.iMinAnimate != 0
}
