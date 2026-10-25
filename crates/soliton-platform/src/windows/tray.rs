//! The hidden top-level owner receives Shell callbacks and Explorer restarts.
use super::app_icons::Icon;
use crate::{DesktopEvent, MediaCommand, TransportState};
use anyhow::Result;
use crossbeam_channel::Sender;
use soliton_core::config::{AppIcon, Language};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM},
        System::LibraryLoader::GetModuleHandleW,
        UI::{Shell::*, WindowsAndMessaging::*},
    },
    core::{HSTRING, w},
};

const CALLBACK: u32 = WM_APP + 6;
// Shell's SDK defines NIN_KEYSELECT as NIN_SELECT | NINF_KEY.
const NIN_KEYSELECT: u32 = NIN_SELECT | NINF_KEY;
const CLASS: windows::core::PCWSTR = w!("Soliton.DesktopIntegration");
const SHOW: usize = 1;
const TOGGLE: usize = 2;
const PREVIOUS: usize = 3;
const NEXT: usize = 4;
const QUIT: usize = 5;

pub(super) struct Tray {
    window: HWND,
    icon: Icon,
    palette: AppIcon,
    // Box address stays stable for GWLP_USERDATA until DestroyWindow returns.
    state: Box<State>,
}

struct State {
    events: Sender<DesktopEvent>,
    ready: Arc<AtomicBool>,
    restart: u32,
    data: NOTIFYICONDATAW,
    language: Language,
    playing: bool,
}

impl Tray {
    pub fn new(
        events: Sender<DesktopEvent>,
        ready: Arc<AtomicBool>,
        language: Language,
        palette: AppIcon,
    ) -> Result<Self> {
        unsafe {
            let instance = GetModuleHandleW(None)?;
            let class = WNDCLASSW {
                lpfnWndProc: Some(procedure),
                hInstance: instance.into(),
                lpszClassName: CLASS,
                ..Default::default()
            };
            anyhow::ensure!(
                RegisterClassW(&class) != 0,
                "Cannot register tray window class"
            );
            let window = match CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                CLASS,
                w!("Soliton notification owner"),
                WINDOW_STYLE::default(),
                0,
                0,
                0,
                0,
                None,
                None,
                Some(instance.into()),
                None,
            ) {
                Ok(window) => window,
                Err(error) => {
                    let _ = UnregisterClassW(CLASS, Some(instance.into()));
                    return Err(error.into());
                }
            };
            let icon = match Icon::load(palette, GetSystemMetrics(SM_CXSMICON)) {
                Ok(icon) => icon,
                Err(error) => {
                    let _ = DestroyWindow(window);
                    let _ = UnregisterClassW(CLASS, Some(instance.into()));
                    return Err(error);
                }
            };
            let mut data = NOTIFYICONDATAW {
                cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
                hWnd: window,
                uID: 1,
                uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP | NIF_SHOWTIP,
                uCallbackMessage: CALLBACK,
                hIcon: icon.0,
                ..Default::default()
            };
            for (dest, value) in data.szTip.iter_mut().zip("Soliton".encode_utf16()) {
                *dest = value;
            }
            data.Anonymous.uVersion = NOTIFYICON_VERSION_4;
            let state = Box::new(State {
                events,
                ready,
                restart: RegisterWindowMessageW(w!("TaskbarCreated")),
                data,
                language,
                playing: false,
            });
            let mut tray = Self {
                window,
                icon,
                palette,
                state,
            };
            SetWindowLongPtrW(
                window,
                GWLP_USERDATA,
                (&mut *tray.state as *mut State) as isize,
            );
            tray.state.install();
            Ok(tray)
        }
    }

    pub fn update(&mut self, state: TransportState, language: Language) {
        self.state.language = language;
        self.state.playing = state == TransportState::Playing;
    }

    pub fn set_icon(&mut self, palette: AppIcon) -> Result<()> {
        if self.palette == palette {
            return Ok(());
        }
        self.palette = palette;
        let icon = Icon::load(palette, unsafe { GetSystemMetrics(SM_CXSMICON) })?;
        let mut data = self.state.data;
        data.hIcon = icon.0;
        let installed = unsafe { Shell_NotifyIconW(NIM_MODIFY, &data) }.as_bool();
        // Retain the latest choice for TaskbarCreated even if Explorer is
        // temporarily unavailable and could not replace its displayed copy.
        self.state.data = data;
        self.icon = icon;
        if !installed {
            self.state.ready.store(false, Ordering::Release);
            let _ = self.state.events.try_send(DesktopEvent::Show);
            anyhow::bail!("Cannot update Windows notification icon");
        }
        Ok(())
    }
}

impl State {
    fn install(&self) {
        let installed = unsafe {
            (Shell_NotifyIconW(NIM_ADD, &self.data).as_bool()
                || Shell_NotifyIconW(NIM_MODIFY, &self.data).as_bool())
                && Shell_NotifyIconW(NIM_SETVERSION, &self.data).as_bool()
        };
        self.ready.store(installed, Ordering::Release);
        if !installed {
            let _ = self.events.try_send(DesktopEvent::Warning(
                "Windows notification area unavailable".into(),
            ));
            let _ = self.events.try_send(DesktopEvent::Show);
        }
    }

    fn menu(&self, window: HWND) -> Result<()> {
        unsafe {
            let menu = CreatePopupMenu()?;
            // Always destroy the HMENU, including an early AppendMenu failure.
            let result = (|| -> Result<()> {
                let french = self.language == Language::French;
                let labels = [
                    (
                        SHOW,
                        if french {
                            "Afficher Soliton"
                        } else {
                            "Show Soliton"
                        },
                    ),
                    (
                        TOGGLE,
                        if self.playing {
                            "Pause"
                        } else if french {
                            "Lire"
                        } else {
                            "Play"
                        },
                    ),
                    (PREVIOUS, if french { "Précédente" } else { "Previous" }),
                    (NEXT, if french { "Suivante" } else { "Next" }),
                    (QUIT, if french { "Quitter" } else { "Quit" }),
                ];
                for (id, label) in labels {
                    AppendMenuW(menu, MF_STRING, id, &HSTRING::from(label))?;
                }
                SetMenuDefaultItem(menu, SHOW as u32, 0)?;
                let mut point = POINT::default();
                GetCursorPos(&mut point)?;
                let _ = SetForegroundWindow(window);
                let choice = TrackPopupMenu(
                    menu,
                    TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON,
                    point.x,
                    point.y,
                    Some(0),
                    window,
                    None,
                )
                .0 as usize;
                if let Some(event) = menu_event(choice) {
                    let _ = self.events.try_send(event);
                }
                let _ = PostMessageW(Some(window), WM_NULL, WPARAM(0), LPARAM(0));
                Ok(())
            })();
            let _ = DestroyMenu(menu);
            result
        }
    }
}

fn menu_event(id: usize) -> Option<DesktopEvent> {
    match id {
        SHOW => Some(DesktopEvent::Show),
        TOGGLE => Some(DesktopEvent::Media(MediaCommand::Toggle)),
        PREVIOUS => Some(DesktopEvent::Media(MediaCommand::Previous)),
        NEXT => Some(DesktopEvent::Media(MediaCommand::Next)),
        QUIT => Some(DesktopEvent::Quit),
        _ => None,
    }
}

unsafe extern "system" fn procedure(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // SAFETY: set after creation and cleared before the owner's Box is freed.
    let pointer = unsafe { GetWindowLongPtrW(window, GWLP_USERDATA) } as *const State;
    if let Some(state) = unsafe { pointer.as_ref() } {
        if message == WM_COMMAND {
            if let Some(event) = menu_event(wparam.0 & 0xffff) {
                let _ = state.events.try_send(event);
            }
            return LRESULT(0);
        }
        if message == state.restart && state.restart != 0 {
            state.ready.store(false, Ordering::Release);
            state.install();
            return LRESULT(0);
        }
        if message == CALLBACK {
            match lparam.0 as u32 & 0xffff {
                NIN_SELECT | NIN_KEYSELECT => {
                    let _ = state.events.try_send(DesktopEvent::Show);
                }
                WM_CONTEXTMENU => {
                    if let Err(error) = state.menu(window) {
                        let _ = state
                            .events
                            .try_send(DesktopEvent::Warning(format!("Tray menu: {error:#}")));
                    }
                }
                _ => {}
            }
            return LRESULT(0);
        }
    }
    unsafe { DefWindowProcW(window, message, wparam, lparam) }
}

impl Drop for Tray {
    fn drop(&mut self) {
        self.state.ready.store(false, Ordering::Release);
        unsafe {
            let _ = Shell_NotifyIconW(NIM_DELETE, &self.state.data);
            SetWindowLongPtrW(self.window, GWLP_USERDATA, 0);
            let _ = DestroyWindow(self.window);
            if let Ok(instance) = GetModuleHandleW(None) {
                let _ = UnregisterClassW(CLASS, Some(instance.into()));
            }
        }
    }
}
