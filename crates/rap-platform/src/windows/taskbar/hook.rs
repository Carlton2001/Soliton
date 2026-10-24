//! The subclass is installed/removed on the HWND's owning thread. Its Arc is
//! retained by the window until explicit detach or WM_NCDESTROY, whichever first.
use super::{Shared, command};
use crate::DesktopEvent;
use anyhow::Result;
use crossbeam_channel::Sender;
use std::sync::{Arc, atomic::Ordering};
use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        Graphics::Dwm::*,
        UI::{Shell::*, WindowsAndMessaging::*},
    },
    core::w,
};

const SUBCLASS: usize = 0x52415054;

pub(super) fn install(window: HWND, events: Sender<DesktopEvent>) -> Result<Arc<Shared>> {
    let shared = Arc::new(Shared {
        frame: Default::default(),
        events,
        generation: 1.into(),
        enabled: true.into(),
        toolbar_ready: false.into(),
        thumbnails: 0.into(),
        created: unsafe { RegisterWindowMessageW(w!("TaskbarButtonCreated")) },
        detach: unsafe { RegisterWindowMessageW(w!("RAP.Taskbar.Detach")) },
    });
    anyhow::ensure!(
        shared.created != 0 && shared.detach != 0,
        "Cannot register taskbar messages"
    );
    let raw = Arc::into_raw(shared.clone());
    unsafe {
        if !SetWindowSubclass(window, Some(procedure), SUBCLASS, raw as usize).as_bool() {
            drop(Arc::from_raw(raw));
            anyhow::bail!("Cannot attach taskbar controls to the window");
        }
    }
    // Keep the main window's native DWM surface available for Peek. Only the
    // separate thumbnail proxy opts into iconic bitmap rendering.
    Ok(shared)
}

/// May be called from the native worker at shutdown; the removal executes on UI.
pub(super) fn detach(window: HWND, shared: &Shared) {
    unsafe {
        SendMessageTimeoutW(
            window,
            shared.detach,
            WPARAM(0),
            LPARAM(0),
            SMTO_ABORTIFHUNG | SMTO_BLOCK,
            500,
            None,
        );
    }
}

unsafe fn remove(window: HWND, raw: *const Shared) {
    unsafe {
        if RemoveWindowSubclass(window, Some(procedure), SUBCLASS).as_bool() {
            let shared = Arc::from_raw(raw);
            shared.enabled.store(false, Ordering::Release);
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
    // SAFETY: one strong reference belongs to this subclass, until remove.
    let shared = unsafe { &*(reference as *const Shared) };
    if message == shared.detach {
        unsafe {
            remove(window, reference as *const Shared);
        }
        return LRESULT(0);
    }
    if message == WM_NCDESTROY {
        unsafe {
            remove(window, reference as *const Shared);
        }
        return unsafe { DefSubclassProc(window, message, wparam, lparam) };
    }
    if message == shared.created {
        shared.generation.fetch_add(1, Ordering::AcqRel);
        shared.toolbar_ready.store(false, Ordering::Release);
    }
    if message == WM_COMMAND
        && ((wparam.0 >> 16) as u32) == THBN_CLICKED
        && let Some(command) = command(wparam.0 as u32 & 0xffff)
    {
        let _ = shared.events.try_send(DesktopEvent::Media(command));
        return LRESULT(0);
    }
    unsafe { DefSubclassProc(window, message, wparam, lparam) }
}

pub(super) fn thumbnail(window: HWND, shared: &Shared, lparam: LPARAM) -> LRESULT {
    let frame = shared.frame.try_lock().ok().and_then(|frame| frame.clone());
    if let Some(frame) = frame {
        let size = (
            ((lparam.0 as u32 >> 16) & 0xffff) as i32,
            (lparam.0 as u32 & 0xffff) as i32,
        );
        // A bounded GDI scale/copy only: decoding and layout are already done.
        if let Ok(bitmap) = frame.bitmap(size.0, size.1) {
            unsafe {
                if DwmSetIconicThumbnail(window, bitmap.bitmap, 0).is_ok() {
                    shared.thumbnails.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
    }
    LRESULT(0)
}
