//! Failures the interface explains in its own language. Producers attach a
//! [`Failure`] to their error; the interface translates its kind and shows the
//! detail (a path, a device name, a system message) unchanged.

/// What went wrong, without any text: the interface owns the wording.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Not classified: the detail is the whole message.
    Other,
    TrackUnavailable,
    /// The file could not be opened or its audio could not be decoded.
    UnreadableAudio,
    /// Channels, sample rate or format changes beyond what Soliton plays.
    UnsupportedAudio,
    /// Isolated corrupted packets were skipped; playback went on.
    SkippedAudio,
    OutputDisconnected,
    OutputUnavailable,
    OutputAmbiguous,
    NoOutputDevice,
    /// Network discovery or the selected receiver is not reachable.
    NetworkUnavailable,
    /// The receiver reported a failure, given as detail when it has one.
    NetworkPlaybackFailed,
    /// A command queue between Soliton's own threads is full or closed.
    Busy,
    SourceInaccessible,
    /// The source did not answer in time; it may only be slow, so its state is kept.
    SourceNotResponding,
    SourceOverlapsStorage,
    SourceOverlapsSource,
    SourceNotDirectory,
    /// A source folder holds no audio file; its entries are kept as unavailable.
    NoAudioFound,
    FolderUnreadable,
    /// Tags of a file could not be read; its previous entry, if any, is kept.
    UnreadableFile,
    CoverIgnored,
    PathNotUnicode,
    PlaylistName,
    /// The playlist or entry changed meanwhile, for instance from another view.
    PlaylistGone,
    SettingsNotSaved,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Failure {
    pub kind: Kind,
    /// Shown as is; empty when the kind says everything.
    pub detail: String,
}

impl Failure {
    pub fn new(kind: Kind) -> Self {
        Self::with(kind, "")
    }

    pub fn with(kind: Kind, detail: impl Into<String>) -> Self {
        Self {
            kind,
            detail: detail.into(),
        }
    }

    /// Names what failed (a path, a device) before the detail.
    pub fn about(mut self, subject: impl std::fmt::Display) -> Self {
        self.detail = if self.detail.is_empty() {
            subject.to_string()
        } else {
            format!("{subject}: {}", self.detail)
        };
        self
    }

    /// The failure attached to `error`, otherwise its whole message, unclassified.
    pub fn of(error: &anyhow::Error) -> Self {
        Self::wrap(Kind::Other, error)
    }

    /// The failure attached to `error`, otherwise its message classified as `kind`.
    pub fn wrap(kind: Kind, error: &anyhow::Error) -> Self {
        error
            .chain()
            .find_map(|cause| cause.downcast_ref::<Self>())
            .cloned()
            .unwrap_or_else(|| Self::with(kind, format!("{error:#}")))
    }
}

/// For logs and command-line tools; the interface translates instead.
impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (self.kind, self.detail.is_empty()) {
            (Kind::Other, _) => f.write_str(&self.detail),
            (kind, true) => write!(f, "{kind:?}"),
            (kind, false) => write!(f, "{kind:?}: {}", self.detail),
        }
    }
}

impl std::error::Error for Failure {}

#[cfg(test)]
#[path = "tests/failure.rs"]
mod tests;
