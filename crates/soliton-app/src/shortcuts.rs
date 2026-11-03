//! Keyboard shortcuts as shown to the user: in control tooltips and in the
//! list of the settings page. The keys themselves are handled in `keys`.
use crate::{
    app::Soliton,
    i18n::Text,
    preferences::{section, setting_row},
    widgets::{button, list_box},
};
use gpui::{prelude::*, *};
use soliton_core::config::Language;

#[derive(Clone, Copy)]
pub enum Shortcut {
    PlayPause,
    Previous,
    Next,
    Seek,
    Volume,
    Mute,
    Shuffle,
    Repeat,
    Search,
    History,
    Sections,
    InterfaceSize,
    FullScreen,
    Back,
    ListMove,
    ListJump,
    ListLetter,
    Open,
    RowMenu,
    RowMove,
    RowSelect,
    RowRemove,
}

/// Every shortcut, in the order of the settings page.
const ALL: [Shortcut; 22] = [
    Shortcut::PlayPause,
    Shortcut::Previous,
    Shortcut::Next,
    Shortcut::Seek,
    Shortcut::Volume,
    Shortcut::Mute,
    Shortcut::Shuffle,
    Shortcut::Repeat,
    Shortcut::Search,
    Shortcut::History,
    Shortcut::Sections,
    Shortcut::InterfaceSize,
    Shortcut::FullScreen,
    Shortcut::Back,
    Shortcut::ListMove,
    Shortcut::ListJump,
    Shortcut::ListLetter,
    Shortcut::Open,
    Shortcut::RowMenu,
    Shortcut::RowMove,
    Shortcut::RowSelect,
    Shortcut::RowRemove,
];

/// Description and keys of a shortcut.
pub fn shortcut(language: Language, shortcut: Shortcut) -> (&'static str, &'static str) {
    let (fr, en) = match shortcut {
        Shortcut::PlayPause => (("Lecture / pause", "Espace"), ("Play / pause", "Space")),
        Shortcut::Previous => (
            ("Morceau précédent", "Ctrl+←"),
            ("Previous track", "Ctrl+←"),
        ),
        Shortcut::Next => (("Morceau suivant", "Ctrl+→"), ("Next track", "Ctrl+→")),
        Shortcut::Seek => (
            ("Reculer / avancer de 10 s", "Ctrl+Maj+← / Ctrl+Maj+→"),
            ("Seek back / forward 10 s", "Ctrl+Shift+← / Ctrl+Shift+→"),
        ),
        Shortcut::Volume => (("Volume", "Ctrl+↑ / Ctrl+↓"), ("Volume", "Ctrl+↑ / Ctrl+↓")),
        Shortcut::Mute => (("Couper / rétablir le son", "M"), ("Mute / unmute", "M")),
        Shortcut::Shuffle => (("Lecture aléatoire", "S"), ("Shuffle", "S")),
        Shortcut::Repeat => (
            ("Répéter : non, la file, le morceau", "R"),
            ("Repeat: off, queue, track", "R"),
        ),
        Shortcut::Search => (("Rechercher", "Ctrl+F"), ("Search", "Ctrl+F")),
        Shortcut::History => (
            ("Page précédente / suivante", "Alt+← / Alt+→"),
            ("Previous / next page", "Alt+← / Alt+→"),
        ),
        Shortcut::Sections => (
            (
                "Artistes, Albums, Playlists, File, Réglages",
                "Ctrl+1 … Ctrl+5",
            ),
            (
                "Artists, Albums, Playlists, Queue, Settings",
                "Ctrl+1 … Ctrl+5",
            ),
        ),
        Shortcut::InterfaceSize => (
            ("Taille de l’interface", "Ctrl++ / Ctrl+- / Ctrl+0"),
            ("Interface size", "Ctrl++ / Ctrl+- / Ctrl+0"),
        ),
        Shortcut::FullScreen => (("Plein écran", "F11"), ("Full screen", "F11")),
        Shortcut::Back => (
            ("Fermer, quitter le plein écran, revenir", "Échap"),
            ("Close, leave full screen, go back", "Esc"),
        ),
        Shortcut::ListMove => (
            ("Parcourir une liste", "Flèches"),
            ("Move through a list", "Arrows"),
        ),
        Shortcut::ListJump => (
            (
                "Début, fin, page d’une liste",
                "Origine / Fin / Pg préc. / Pg suiv.",
            ),
            (
                "Start, end, page of a list",
                "Home / End / Page Up / Page Down",
            ),
        ),
        Shortcut::ListLetter => (
            ("Aller au nom suivant commençant par une lettre", "A … Z"),
            ("Go to the next name starting with a letter", "A … Z"),
        ),
        Shortcut::Open => (
            ("Ouvrir ou lire l’élément", "Entrée"),
            ("Open or play the item", "Enter"),
        ),
        Shortcut::RowMenu => (("Menu du morceau", "Maj+F10"), ("Track menu", "Shift+F10")),
        Shortcut::RowMove => (
            ("Déplacer dans la file ou la playlist", "Alt+↑ / Alt+↓"),
            ("Move in the queue or playlist", "Alt+↑ / Alt+↓"),
        ),
        Shortcut::RowSelect => (
            ("Étendre la sélection", "Maj+↑ / Maj+↓"),
            ("Extend the selection", "Shift+↑ / Shift+↓"),
        ),
        Shortcut::RowRemove => (
            ("Retirer de la file ou de la playlist", "Suppr"),
            ("Remove from the queue or playlist", "Delete"),
        ),
    };
    match language {
        Language::French => fr,
        Language::English => en,
    }
}

impl Soliton {
    /// Keys of a shortcut, as shown beside a control's label.
    pub fn shortcut_keys(&self, key: Shortcut) -> &'static str {
        shortcut(self.settings.language, key).1
    }

    /// Folded by default: its header toggles the list.
    pub fn shortcuts_section(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme();
        let open = self.shortcuts_open;
        let toggle = button(
            "toggle-shortcuts",
            if open { "chevron-up" } else { "chevron-down" },
            self.t(if open { Text::Hide } else { Text::Show }),
            theme,
        )
        .on_click(cx.listener(|this, _, _, cx| {
            this.shortcuts_open = !this.shortcuts_open;
            cx.notify();
        }));
        let title = self.t(Text::KeyboardShortcuts);
        if !open {
            return section(title, Some(toggle.into_any_element()), div());
        }
        let rows = ALL.map(|key| {
            let (label, keys) = shortcut(self.settings.language, key);
            setting_row(
                label,
                None,
                div()
                    .text_color(theme.muted)
                    .font(crate::widgets::numeric_font())
                    .child(keys),
                theme,
            )
        });
        section(
            title,
            Some(toggle.into_any_element()),
            list_box(rows, theme),
        )
    }
}
