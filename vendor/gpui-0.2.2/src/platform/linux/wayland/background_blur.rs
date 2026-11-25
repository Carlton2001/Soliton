use wayland_client::protocol::wl_surface::WlSurface;
use wayland_protocols::ext::background_effect::v1::client::ext_background_effect_surface_v1::ExtBackgroundEffectSurfaceV1;
use wayland_protocols_plasma::blur::client::org_kde_kwin_blur::OrgKdeKwinBlur;

use crate::{Bounds, Globals};

#[derive(Default)]
pub(super) enum BackgroundBlur {
    #[default]
    Disabled,
    Wayland(ExtBackgroundEffectSurfaceV1, Bounds<i32>),
    Kde(OrgKdeKwinBlur),
}

impl BackgroundBlur {
    pub(super) fn update(
        &mut self,
        globals: &Globals,
        surface: &WlSurface,
        area: Option<Bounds<i32>>,
    ) {
        let Some(area) = area else {
            self.clear(globals, surface);
            return;
        };

        if let Some(manager) = &globals.background_effect_manager {
            let effect = match self {
                Self::Wayland(_, previous) if *previous == area => return,
                Self::Wayland(effect, _) => effect.clone(),
                _ => manager.get_background_effect(surface, &globals.qh, ()),
            };
            let region = globals.compositor.create_region(&globals.qh, ());
            region.add(
                area.origin.x,
                area.origin.y,
                area.size.width,
                area.size.height,
            );
            effect.set_blur_region(Some(&region));
            // The protocol copies the region and applies it on the next surface commit.
            region.destroy();
            *self = Self::Wayland(effect, area);
        } else if let Some(manager) = globals.legacy_blur_manager() {
            if !matches!(self, Self::Kde(_)) {
                let blur = manager.create(surface, &globals.qh, ());
                blur.commit();
                *self = Self::Kde(blur);
            }
        }
    }

    pub(super) fn clear(&mut self, globals: &Globals, surface: &WlSurface) {
        match std::mem::take(self) {
            Self::Wayland(effect, _) => effect.destroy(),
            Self::Kde(blur) => {
                if let Some(manager) = globals.legacy_blur_manager() {
                    manager.unset(surface);
                }
                blur.release();
            }
            Self::Disabled => {}
        }
    }
}
