//! One owner for everything that plays: a post's video, a live stream and Bandcamp
//! music. Only one of them plays at a time, and each says what happens when you
//! leave the screen it plays on.

use super::App;

/// What happens to something playing when you leave the screen it plays on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum OnLeave {
    Stop,
    /// Paused where it was, to carry on when you come back.
    Pause,
    /// Keeps playing, with its controls in the mini-player.
    Keep,
}

/// What can play.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    Video,
    Live,
    Music,
}

impl Kind {
    /// A post's video pauses while something is opened on top of its screen (`below`
    /// is true: Back returns to it) and stops when you go elsewhere. A live stream
    /// stops, as it can't be paused and caught up. Music keeps playing.
    pub(super) fn on_leave(self, below: bool) -> OnLeave {
        match self {
            Kind::Video if below => OnLeave::Pause,
            Kind::Video | Kind::Live => OnLeave::Stop,
            Kind::Music => OnLeave::Keep,
        }
    }
}

impl App {
    /// Before `kind` starts playing: stops whatever else plays.
    pub(super) fn make_room(&mut self, kind: Kind) {
        match kind {
            Kind::Music => self.video = None,
            Kind::Video | Kind::Live => self.music = None,
        }
    }

    /// After the screen changed: what plays on another screen stops, pauses or keeps
    /// going, as its kind says. A video shown over the whole window carries on.
    pub(super) fn playback_on_enter(&mut self) {
        let Some(v) = &self.video else {
            return;
        };
        if v.expanded || v.route == *self.route() {
            return;
        }
        let kind = if v.player.live() {
            Kind::Live
        } else {
            Kind::Video
        };
        let below = self.routes.contains(&v.route);
        match kind.on_leave(below) {
            OnLeave::Stop => self.video = None,
            OnLeave::Pause => v.player.set_paused(true),
            OnLeave::Keep => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_kind_says_what_leaving_does() {
        assert_eq!(Kind::Video.on_leave(true), OnLeave::Pause);
        assert_eq!(Kind::Video.on_leave(false), OnLeave::Stop);
        assert_eq!(Kind::Live.on_leave(true), OnLeave::Stop);
        assert_eq!(Kind::Music.on_leave(false), OnLeave::Keep);
    }
}
