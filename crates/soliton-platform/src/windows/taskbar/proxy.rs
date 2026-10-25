//! A hidden tab supplies the music thumbnail; Peek uses the main DWM surface.
//! Created, messaged and destroyed on the desktop worker, never the GPUI thread.
use super::super::app_icons::IconPair;
use super::{Shared, command, hook};
use crate::DesktopEvent;
use anyhow::Result;
use soliton_core::config::AppIcon;
use std::sync::{Arc, atomic::Ordering};
use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        Graphics::Dwm::*,
        UI::{Shell::*, WindowsAndMessaging::*},
    },
    core::w,
};

const SUBCLASS: usize = 0x52415050;

pub(super) struct Proxy {
    window: HWND,
    // Stable address borrowed by the subclass until DestroyWindow returns.
    state: Box<State>,
    registered: bool,
    icons: IconPair,
    palette: AppIcon,
}

struct State {
    main: HWND,
    shared: Arc<Shared>,
}

impl Proxy {
    pub fn new(main: HWND, shared: Arc<Shared>) -> Result<Self> {
        let icons = IconPair::load(AppIcon::default())?;
        let window = unsafe {
            CreateWindowExW(
                WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                w!("STATIC"),
                w!("Soliton"),
                WS_POPUP | WS_CAPTION | WS_SYSMENU,
                0,
                0,
                640,
                360,
                None,
                None,
                None,
                None,
            )?
        };
        let proxy = Self {
            window,
            state: Box::new(State { main, shared }),
            registered: false,
            icons,
            palette: AppIcon::default(),
        };
        unsafe {
            anyhow::ensure!(
                SetWindowSubclass(
                    window,
                    Some(procedure),
                    SUBCLASS,
                    (&*proxy.state as *const State) as usize,
                )
                .as_bool(),
                "Cannot attach the taskbar thumbnail proxy"
            );
        }
        proxy.icons.apply(window)?;
        let enabled = 1_i32;
        for attribute in [DWMWA_FORCE_ICONIC_REPRESENTATION, DWMWA_HAS_ICONIC_BITMAP] {
            unsafe {
                DwmSetWindowAttribute(
                    window,
                    attribute,
                    (&enabled as *const i32).cast(),
                    std::mem::size_of_val(&enabled) as u32,
                )?;
            }
        }
        Ok(proxy)
    }

    pub fn window(&self) -> HWND {
        self.window
    }

    pub fn set_icon(&mut self, palette: AppIcon) -> Result<()> {
        if self.palette == palette {
            return Ok(());
        }
        // A failed palette is reported once; a different choice can retry.
        self.palette = palette;
        let icons = IconPair::load(palette)?;
        icons.apply(self.window)?;
        self.icons = icons;
        Ok(())
    }

    pub fn register(&mut self, api: &ITaskbarList4) -> Result<()> {
        if self.registered {
            return Ok(());
        }
        unsafe {
            api.RegisterTab(self.window, self.state.main)?;
            // Set the source before exposing the thumbnail in the flyout.
            // The proxy has a custom thumbnail; the main window keeps native Peek.
            let result = (|| -> Result<()> {
                api.SetTabProperties(self.window, STPF_USEAPPPEEKALWAYS)?;
                api.SetTabOrder(self.window, HWND::default())?;
                api.SetTabActive(self.window, self.state.main, 0)?;
                Ok(())
            })();
            if result.is_err() {
                let _ = api.UnregisterTab(self.window);
            }
            result?;
        }
        self.registered = true;
        Ok(())
    }

    pub fn unregister(&mut self, api: &ITaskbarList4) {
        if self.registered {
            unsafe {
                let _ = api.UnregisterTab(self.window);
            }
            self.registered = false;
        }
    }
}

impl Drop for Proxy {
    fn drop(&mut self) {
        unsafe {
            // The state stays alive for all destruction callbacks.
            let _ = DestroyWindow(self.window);
        }
    }
}

unsafe extern "system" fn procedure(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _: usize,
    reference: usize,
) -> LRESULT {
    // SAFETY: Proxy owns the Box until DestroyWindow finishes on this thread.
    let state = unsafe { &*(reference as *const State) };
    if message == WM_NCDESTROY {
        unsafe {
            let _ = RemoveWindowSubclass(window, Some(procedure), SUBCLASS);
            return DefSubclassProc(window, message, wparam, lparam);
        }
    }
    if state.shared.enabled.load(Ordering::Acquire) {
        if message == WM_DWMSENDICONICTHUMBNAIL {
            return hook::thumbnail(window, &state.shared, lparam);
        }
        if message == WM_COMMAND
            && ((wparam.0 >> 16) as u32) == THBN_CLICKED
            && let Some(command) = command(wparam.0 as u32 & 0xffff)
        {
            let _ = state.shared.events.try_send(DesktopEvent::Media(command));
            return LRESULT(0);
        }
        if (message == WM_ACTIVATE && (wparam.0 & 0xffff) != WA_INACTIVE as usize)
            || (message == WM_SYSCOMMAND && (wparam.0 & 0xfff0) == SC_RESTORE as usize)
        {
            let _ = state.shared.events.try_send(DesktopEvent::Show);
            return LRESULT(0);
        }
        if message == WM_CLOSE
            || (message == WM_SYSCOMMAND && (wparam.0 & 0xfff0) == SC_CLOSE as usize)
        {
            // Use Soliton's normal close policy, including hide-to-tray/confirmation.
            unsafe {
                let _ = PostMessageW(Some(state.main), WM_CLOSE, WPARAM(0), LPARAM(0));
            }
            return LRESULT(0);
        }
    }
    // The proxy is never a user-facing window, including during shutdown.
    if matches!(message, WM_CLOSE | WM_SYSCOMMAND) {
        return LRESULT(0);
    }
    unsafe { DefSubclassProc(window, message, wparam, lparam) }
}

#[cfg(test)]
#[path = "../tests/taskbar_proxy.rs"]
mod tests;
