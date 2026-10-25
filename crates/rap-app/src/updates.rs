//! Adapted from Oxyde-GPUI's release checker (Carlton2001, GPL-3.0).
//! Installation requires an explicit click and a verified installer signature.
use crate::{
    app::Rap,
    i18n::Text,
    preferences::{hint, section, setting_row},
    widgets::{button, checkbox, list_box},
};
use gpui::{prelude::*, *};

use rap_core::updates::{LIMIT, Release, parse, valid_repository};

#[derive(Default)]
pub struct Updates {
    checking: bool,
    checked: bool,
    failed: bool,
    available: Option<Release>,
    installing: bool,
}

impl Updates {
    pub fn is_available(&self) -> bool {
        self.available.is_some()
    }
}

fn repository() -> Option<&'static str> {
    Some(option_env!("RAP_UPDATE_REPOSITORY").unwrap_or("Carlton2001/Soliton"))
        .filter(|value| valid_repository(value))
}

impl Rap {
    pub fn update_tooltip(&self) -> Text {
        if self.updates.installing {
            Text::InstallingUpdate
        } else {
            Text::UpdateNotification
        }
    }

    pub fn install_update(&mut self, cx: &mut Context<Self>) {
        if self.updates.installing || self.quitting {
            return;
        }
        let Some(release) = self.updates.available.clone() else {
            return;
        };
        let key = option_env!("RAP_UPDATE_PUBLIC_KEY").unwrap_or("");
        self.updates.installing = true;
        let task = cx.background_executor().spawn(async move {
            if rap_platform::updater::is_portable().map_err(|e| e.to_string())? {
                return Ok(None);
            }
            rap_platform::updater::prepare(&release, key).map(Some)
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                if this.quitting {
                    return;
                }
                match result {
                    Ok(Some(prepared)) => {
                        let launch = cx
                            .background_executor()
                            .spawn(async move { prepared.launch() });
                        cx.spawn(async move |this, cx| {
                            let result = launch.await;
                            let _ = this.update(cx, |this, cx| {
                                this.updates.installing = false;
                                match result {
                                    Ok(()) => this.quit_app(cx),
                                    Err(error) => {
                                        this.install_update_failed(&error.to_string(), cx)
                                    }
                                }
                            });
                        })
                        .detach();
                    }
                    Ok(None) => {
                        this.updates.installing = false;
                        this.report_warning(this.t(Text::PortableUpdate));
                        if let Some(release) = &this.updates.available {
                            cx.open_url(&release.page);
                        }
                        cx.notify();
                    }
                    Err(error) => {
                        if key.trim().is_empty() {
                            this.updates.installing = false;
                            this.report_warning(this.t(Text::UpdateKeyMissing));
                            cx.notify();
                        } else {
                            this.install_update_failed(&error, cx);
                        }
                    }
                }
            });
        })
        .detach();
        cx.notify();
    }

    fn install_update_failed(&mut self, error: &str, cx: &mut Context<Self>) {
        self.updates.installing = false;
        tracing::warn!(%error, "Update installation failed");
        self.report_warning(self.t(Text::UpdateInstallFailed));
        cx.notify();
    }

    pub fn check_for_updates(&mut self, cx: &mut Context<Self>) {
        let Some(repository) = repository() else {
            return;
        };
        if self.updates.checking || self.updates.installing {
            return;
        }
        self.updates.checking = true;
        self.updates.failed = false;
        let query = cx.background_executor().spawn(async move {
            let url =
                format!("https://github.com/{repository}/releases/latest/download/latest.json");
            rap_platform::download_https(&url, LIMIT)
                .map_err(|e| e.to_string())
                .and_then(|bytes| parse(&bytes, env!("CARGO_PKG_VERSION"), repository))
        });
        cx.spawn(async move |this, cx| {
            let result = query.await;
            let _ = this.update(cx, |this, cx| {
                this.updates.checking = false;
                this.updates.checked = true;
                match result {
                    Ok(release) => this.updates.available = release,
                    Err(error) => {
                        tracing::warn!(%error, "Update check failed");
                        this.updates.failed = true;
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    pub fn update_preferences(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme();
        let check = button(
            "check-updates",
            "refresh-cw",
            self.t(if self.updates.checking {
                Text::CheckingUpdates
            } else {
                Text::CheckUpdates
            }),
            theme,
        )
        .when(
            self.updates.checking || self.updates.installing || repository().is_none(),
            |button| button.opacity(0.5),
        )
        .on_click(cx.listener(|this, _, _, cx| this.check_for_updates(cx)));
        let automatic = checkbox(
            "automatic-updates",
            self.settings.check_updates,
            self.t(Text::AutoCheckUpdates),
            theme,
        )
        .on_click(cx.listener(|this, _, _, cx| {
            this.settings.check_updates = !this.settings.check_updates;
            this.save();
            cx.notify();
        }));
        let mut body = div()
            .w_full()
            .flex_shrink_0()
            .flex()
            .flex_col()
            .gap(rems(0.625))
            .child(list_box(
                [
                    setting_row(
                        self.t(Text::Version),
                        None,
                        format!("RAP {}", env!("CARGO_PKG_VERSION")),
                        theme,
                    ),
                    setting_row(
                        self.t(Text::CheckUpdates),
                        Some(self.t(Text::UpdateHint)),
                        check,
                        theme,
                    ),
                    div()
                        .px(crate::widgets::PAGE_INSET)
                        .py(rems(0.625))
                        .child(automatic)
                        .into_any_element(),
                ],
                theme,
            ));
        let status = if repository().is_none() {
            Some(Text::UpdateNotConfigured)
        } else if self.updates.failed {
            Some(Text::UpdateFailed)
        } else if self.updates.available.is_some() {
            Some(Text::UpdateAvailable)
        } else if self.updates.checked && !self.updates.checking {
            Some(Text::UpToDate)
        } else {
            None
        };
        if let Some(status) = status {
            body = body.child(hint(self.t(status), theme));
        }
        if let Some(release) = &self.updates.available {
            body = body.child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(rems(0.75))
                    .child(format!("RAP {}", release.version))
                    .child(
                        button(
                            "open-update",
                            "arrow-down",
                            self.t(if self.updates.installing {
                                Text::InstallingUpdate
                            } else {
                                Text::InstallUpdate
                            }),
                            theme,
                        )
                        .on_click(cx.listener(|this, _, _, cx| this.install_update(cx))),
                    ),
            );
        }
        section(self.t(Text::About), None, body)
    }
}
