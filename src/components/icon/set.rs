//! Interface icons from Tabler Icons (MIT, https://tabler.io/icons), stored as SVG
//! path data and tinted at render time.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Icon {
    Home,
    Bell,
    Search,
    Settings,
    Compose,
    Reply,
    Repost,
    Quote,
    Heart,
    HeartFilled,
    More,
    Back,
    Refresh,
    External,
    Photo,
    Close,
    Follow,
    Check,
    ChevronLeft,
    ChevronRight,
    Download,
    Bookmark,
    BookmarkFilled,
    User,
    Drafts,
    ArrowUp,
    Link,
    Copy,
    BellOff,
    Flag,
    Trash,
    Language,
    Ban,
    VolumeOff,
    Hash,

    ArrowDown,
    Shield,
    Plus,
    EyeOff,
    Play,
    Mood,
    Clock,
    Trending,
    Pin,
    Pencil,
    Chat,
    Send,
    Users,
    List,
    Video,
    Columns,
    Pause,
    Volume,
    Maximize,
    Lock,
    Leave,
    Alert,
    Live,
    Filter,
    Article,
    Folder,
    Chart,
    Calendar,
    Computer,
}

impl Icon {
    /// The icon's SVG body: path elements on a 24x24 grid.
    fn body(self) -> (&'static str, bool) {
        match self {
            Icon::Home => (
                r#"<path d="M5 12l-2 0l9 -9l9 9l-2 0" /> <path d="M5 12v7a2 2 0 0 0 2 2h10a2 2 0 0 0 2 -2v-7" /> <path d="M9 21v-6a2 2 0 0 1 2 -2h2a2 2 0 0 1 2 2v6" />"#,
                false,
            ),
            Icon::Bell => (
                r#"<path d="M10 5a2 2 0 1 1 4 0a7 7 0 0 1 4 6v3a4 4 0 0 0 2 3h-16a4 4 0 0 0 2 -3v-3a7 7 0 0 1 4 -6" /> <path d="M9 17v1a3 3 0 0 0 6 0v-1" />"#,
                false,
            ),
            Icon::Search => (
                r#"<path d="M3 10a7 7 0 1 0 14 0a7 7 0 1 0 -14 0" /> <path d="M21 21l-6 -6" />"#,
                false,
            ),
            Icon::Settings => (
                r#"<path d="M10.325 4.317c.426 -1.756 2.924 -1.756 3.35 0a1.724 1.724 0 0 0 2.573 1.066c1.543 -.94 3.31 .826 2.37 2.37a1.724 1.724 0 0 0 1.065 2.572c1.756 .426 1.756 2.924 0 3.35a1.724 1.724 0 0 0 -1.066 2.573c.94 1.543 -.826 3.31 -2.37 2.37a1.724 1.724 0 0 0 -2.572 1.065c-.426 1.756 -2.924 1.756 -3.35 0a1.724 1.724 0 0 0 -2.573 -1.066c-1.543 .94 -3.31 -.826 -2.37 -2.37a1.724 1.724 0 0 0 -1.065 -2.572c-1.756 -.426 -1.756 -2.924 0 -3.35a1.724 1.724 0 0 0 1.066 -2.573c-.94 -1.543 .826 -3.31 2.37 -2.37c1 .608 2.296 .07 2.572 -1.065" /> <path d="M9 12a3 3 0 1 0 6 0a3 3 0 0 0 -6 0" />"#,
                false,
            ),
            Icon::Compose => (
                r#"<path d="M4 20h4l10.5 -10.5a2.828 2.828 0 1 0 -4 -4l-10.5 10.5v4" /> <path d="M13.5 6.5l4 4" />"#,
                false,
            ),
            Icon::Reply => (
                r#"<path d="M3 20l1.3 -3.9c-2.324 -3.437 -1.426 -7.872 2.1 -10.374c3.526 -2.501 8.59 -2.296 11.845 .48c3.255 2.777 3.695 7.266 1.029 10.501c-2.666 3.235 -7.615 4.215 -11.574 2.293l-4.7 1" />"#,
                false,
            ),
            Icon::Repost => (
                r#"<path d="M4 12v-3a3 3 0 0 1 3 -3h13m-3 -3l3 3l-3 3" /> <path d="M20 12v3a3 3 0 0 1 -3 3h-13m3 3l-3 -3l3 -3" />"#,
                false,
            ),
            Icon::Quote => (
                r#"<path d="M10 11h-4a1 1 0 0 1 -1 -1v-3a1 1 0 0 1 1 -1h3a1 1 0 0 1 1 1v6c0 2.667 -1.333 4.333 -4 5" /> <path d="M19 11h-4a1 1 0 0 1 -1 -1v-3a1 1 0 0 1 1 -1h3a1 1 0 0 1 1 1v6c0 2.667 -1.333 4.333 -4 5" />"#,
                false,
            ),
            Icon::Heart => (
                r#"<path d="M19.5 12.572l-7.5 7.428l-7.5 -7.428a5 5 0 1 1 7.5 -6.566a5 5 0 1 1 7.5 6.572" />"#,
                false,
            ),
            Icon::HeartFilled => (
                r#"<path d="M6.979 3.074a6 6 0 0 1 4.988 1.425l.037 .033l.034 -.03a6 6 0 0 1 4.733 -1.44l.246 .036a6 6 0 0 1 3.364 10.008l-.18 .185l-.048 .041l-7.45 7.379a1 1 0 0 1 -1.313 .082l-.094 -.082l-7.493 -7.422a6 6 0 0 1 3.176 -10.215z" />"#,
                true,
            ),
            Icon::More => (
                r#"<path d="M4 12a1 1 0 1 0 2 0a1 1 0 1 0 -2 0" /> <path d="M11 12a1 1 0 1 0 2 0a1 1 0 1 0 -2 0" /> <path d="M18 12a1 1 0 1 0 2 0a1 1 0 1 0 -2 0" />"#,
                false,
            ),
            Icon::Back => (
                r#"<path d="M5 12l14 0" /> <path d="M5 12l6 6" /> <path d="M5 12l6 -6" />"#,
                false,
            ),
            Icon::Refresh => (
                r#"<path d="M20 11a8.1 8.1 0 0 0 -15.5 -2m-.5 -4v4h4" /> <path d="M4 13a8.1 8.1 0 0 0 15.5 2m.5 4v-4h-4" />"#,
                false,
            ),
            Icon::External => (
                r#"<path d="M12 6h-6a2 2 0 0 0 -2 2v10a2 2 0 0 0 2 2h10a2 2 0 0 0 2 -2v-6" /> <path d="M11 13l9 -9" /> <path d="M15 4h5v5" />"#,
                false,
            ),
            Icon::Photo => (
                r#"<path d="M15 8h.01" /> <path d="M3 6a3 3 0 0 1 3 -3h12a3 3 0 0 1 3 3v12a3 3 0 0 1 -3 3h-12a3 3 0 0 1 -3 -3v-12" /> <path d="M3 16l5 -5c.928 -.893 2.072 -.893 3 0l5 5" /> <path d="M14 14l1 -1c.928 -.893 2.072 -.893 3 0l3 3" />"#,
                false,
            ),
            Icon::Close => (
                r#"<path d="M18 6l-12 12" /> <path d="M6 6l12 12" />"#,
                false,
            ),
            Icon::Follow => (
                r#"<path d="M8 7a4 4 0 1 0 8 0a4 4 0 0 0 -8 0" /> <path d="M16 19h6" /> <path d="M19 16v6" /> <path d="M6 21v-2a4 4 0 0 1 4 -4h4" />"#,
                false,
            ),
            Icon::Check => (r#"<path d="M5 12l5 5l10 -10" />"#, false),
            Icon::ChevronLeft => (r#"<path d="M15 6l-6 6l6 6" />"#, false),
            Icon::ChevronRight => (r#"<path d="M9 6l6 6l-6 6" />"#, false),
            Icon::Download => (
                r#"<path d="M4 17v2a2 2 0 0 0 2 2h12a2 2 0 0 0 2 -2v-2" /> <path d="M7 11l5 5l5 -5" /> <path d="M12 4l0 12" />"#,
                false,
            ),
            Icon::Bookmark => (
                r#"<path d="M18 7v14l-6 -4l-6 4v-14a4 4 0 0 1 4 -4h4a4 4 0 0 1 4 4z" />"#,
                false,
            ),
            Icon::BookmarkFilled => (
                r#"<path d="M14 2a5 5 0 0 1 5 5v14a1 1 0 0 1 -1.555 .832l-5.445 -3.63l-5.444 3.63a1 1 0 0 1 -1.55 -.72l-.006 -.112v-14a5 5 0 0 1 5 -5h4z" />"#,
                true,
            ),
            Icon::User => (
                r#"<path d="M8 7a4 4 0 1 0 8 0a4 4 0 0 0 -8 0" /> <path d="M6 21v-2a4 4 0 0 1 4 -4h4a4 4 0 0 1 4 4v2" />"#,
                false,
            ),
            Icon::Drafts => (
                r#"<path d="M14 3v4a1 1 0 0 0 1 1h4" /> <path d="M17 21h-10a2 2 0 0 1 -2 -2v-14a2 2 0 0 1 2 -2h7l5 5v11a2 2 0 0 1 -2 2z" /> <path d="M9 9l1 0" /> <path d="M9 13l6 0" /> <path d="M9 17l6 0" />"#,
                false,
            ),
            Icon::Hash => (
                r#"<path d="M5 9l14 0" /> <path d="M5 15l14 0" /> <path d="M11 4l-4 16" /> <path d="M17 4l-4 16" />"#,
                false,
            ),
            Icon::ArrowDown => (
                r#"<path d="M12 5l0 14" /> <path d="M18 13l-6 6" /> <path d="M6 13l6 6" />"#,
                false,
            ),
            Icon::EyeOff => (
                r#"<path d="M10.585 10.587a2 2 0 0 0 2.829 2.828" /> <path d="M16.681 16.673a8.717 8.717 0 0 1 -4.681 1.327c-3.6 0 -6.6 -2 -9 -6c1.272 -2.12 2.712 -3.678 4.32 -4.674m2.86 -1.146a9.055 9.055 0 0 1 1.82 -.18c3.6 0 6.6 2 9 6c-.666 1.11 -1.379 2.067 -2.138 2.87" /> <path d="M3 3l18 18" />"#,
                false,
            ),
            Icon::Mood => (
                r#"<path d="M3 12a9 9 0 1 0 18 0a9 9 0 1 0 -18 0" /> <path d="M9 10l.01 0" /> <path d="M15 10l.01 0" /> <path d="M9.5 15a3.5 3.5 0 0 0 5 0" />"#,
                false,
            ),
            Icon::Clock => (
                r#"<path d="M3 12a9 9 0 1 0 18 0a9 9 0 0 0 -18 0" /> <path d="M12 7v5l3 3" />"#,
                false,
            ),
            Icon::Trending => (
                r#"<path d="M3 17l6 -6l4 4l8 -8" /> <path d="M14 7l7 0l0 7" />"#,
                false,
            ),
            Icon::Pin => (
                r#"<path d="M15 4.5l-4 4l-4 1.5l-1.5 1.5l7 7l1.5 -1.5l1.5 -4l4 -4" /> <path d="M9 15l-4.5 4.5" /> <path d="M14.5 4l5.5 5.5" />"#,
                false,
            ),
            Icon::Pencil => (
                r#"<path d="M4 20h4l10.5 -10.5a2.828 2.828 0 1 0 -4 -4l-10.5 10.5v4" /> <path d="M13.5 6.5l4 4" />"#,
                false,
            ),
            Icon::Chat => (
                r#"<path d="M3 20l1.3 -3.9c-2.324 -3.437 -1.426 -7.872 2.1 -10.374c3.526 -2.501 8.59 -2.296 11.845 .48c3.255 2.777 3.695 7.266 1.029 10.501c-2.666 3.235 -7.615 4.215 -11.574 2.293l-4.7 1" />"#,
                false,
            ),
            Icon::Send => (
                r#"<path d="M10 14l11 -11" /> <path d="M21 3l-6.5 18a.55 .55 0 0 1 -1 0l-3.5 -7l-7 -3.5a.55 .55 0 0 1 0 -1l18 -6.5" />"#,
                false,
            ),
            Icon::Users => (
                r#"<path d="M5 7a4 4 0 1 0 8 0a4 4 0 1 0 -8 0" /> <path d="M3 21v-2a4 4 0 0 1 4 -4h4a4 4 0 0 1 4 4v2" /> <path d="M16 3.13a4 4 0 0 1 0 7.75" /> <path d="M21 21v-2a4 4 0 0 0 -3 -3.85" />"#,
                false,
            ),
            Icon::List => (
                r#"<path d="M9 6l11 0" /> <path d="M9 12l11 0" /> <path d="M9 18l11 0" /> <path d="M5 6l0 .01" /> <path d="M5 12l0 .01" /> <path d="M5 18l0 .01" />"#,
                false,
            ),
            Icon::Video => (
                r#"<path d="M15 10l4.553 -2.276a1 1 0 0 1 1.447 .894v6.764a1 1 0 0 1 -1.447 .894l-4.553 -2.276v-4z" /> <path d="M3 8a2 2 0 0 1 2 -2h8a2 2 0 0 1 2 2v8a2 2 0 0 1 -2 2h-8a2 2 0 0 1 -2 -2z" />"#,
                false,
            ),
            Icon::Columns => (
                r#"<path d="M4 6a2 2 0 0 1 2 -2h12a2 2 0 0 1 2 2v12a2 2 0 0 1 -2 2h-12a2 2 0 0 1 -2 -2z" /> <path d="M12 4l0 16" />"#,
                false,
            ),
            Icon::Pause => (
                r#"<path d="M6 6a1 1 0 0 1 1 -1h2a1 1 0 0 1 1 1v12a1 1 0 0 1 -1 1h-2a1 1 0 0 1 -1 -1z" /> <path d="M14 6a1 1 0 0 1 1 -1h2a1 1 0 0 1 1 1v12a1 1 0 0 1 -1 1h-2a1 1 0 0 1 -1 -1z" />"#,
                false,
            ),
            Icon::Volume => (
                r#"<path d="M15 8a5 5 0 0 1 0 8" /> <path d="M17.7 5a9 9 0 0 1 0 14" /> <path d="M6 15h-2a1 1 0 0 1 -1 -1v-4a1 1 0 0 1 1 -1h2l3.5 -4.5a.8 .8 0 0 1 1.5 .5v14a.8 .8 0 0 1 -1.5 .5l-3.5 -4.5" />"#,
                false,
            ),
            Icon::Maximize => (
                r#"<path d="M4 8v-2a2 2 0 0 1 2 -2h2" /> <path d="M4 16v2a2 2 0 0 0 2 2h2" /> <path d="M16 4h2a2 2 0 0 1 2 2v2" /> <path d="M16 20h2a2 2 0 0 0 2 -2v-2" />"#,
                false,
            ),
            Icon::Alert => (
                r#"<path d="M12 9v4" /> <path d="M10.363 3.591l-8.106 13.534a1.914 1.914 0 0 0 1.636 2.871h16.214a1.914 1.914 0 0 0 1.636 -2.87l-8.106 -13.536a1.914 1.914 0 0 0 -3.274 0z" /> <path d="M12 16h.01" />"#,
                false,
            ),
            Icon::Live => (
                r#"<path d="M18.364 19.364a9 9 0 1 0 -12.728 0" /> <path d="M15.536 16.536a5 5 0 1 0 -7.072 0" /> <path d="M11 13a1 1 0 1 0 2 0a1 1 0 1 0 -2 0" />"#,
                false,
            ),
            Icon::Article => (
                r#"<path d="M3 6a2 2 0 0 1 2 -2h14a2 2 0 0 1 2 2v12a2 2 0 0 1 -2 2h-14a2 2 0 0 1 -2 -2z" /> <path d="M7 8h10" /> <path d="M7 12h10" /> <path d="M7 16h10" />"#,
                false,
            ),
            Icon::Folder => (
                r#"<path d="M5 4h4l3 3h7a2 2 0 0 1 2 2v8a2 2 0 0 1 -2 2h-14a2 2 0 0 1 -2 -2v-11a2 2 0 0 1 2 -2" />"#,
                false,
            ),
            Icon::Chart => (
                r#"<path d="M3 13a1 1 0 0 1 1 -1h4a1 1 0 0 1 1 1v6a1 1 0 0 1 -1 1h-4a1 1 0 0 1 -1 -1z" /> <path d="M15 9a1 1 0 0 1 1 -1h4a1 1 0 0 1 1 1v10a1 1 0 0 1 -1 1h-4a1 1 0 0 1 -1 -1z" /> <path d="M9 5a1 1 0 0 1 1 -1h4a1 1 0 0 1 1 1v14a1 1 0 0 1 -1 1h-4a1 1 0 0 1 -1 -1z" /> <path d="M4 20h14" />"#,
                false,
            ),
            Icon::Calendar => (
                r#"<path d="M4 7a2 2 0 0 1 2 -2h12a2 2 0 0 1 2 2v12a2 2 0 0 1 -2 2h-12a2 2 0 0 1 -2 -2v-12z" /> <path d="M16 3v4" /> <path d="M8 3v4" /> <path d="M4 11h16" />"#,
                false,
            ),
            Icon::Computer => (
                r#"<path d="M3 5a1 1 0 0 1 1 -1h16a1 1 0 0 1 1 1v10a1 1 0 0 1 -1 1h-16a1 1 0 0 1 -1 -1v-10z" /> <path d="M7 20h10" /> <path d="M9 16v4" /> <path d="M15 16v4" />"#,
                false,
            ),
            Icon::Filter => (
                r#"<path d="M4 4h16v2.172a2 2 0 0 1 -.586 1.414l-4.414 4.414v7l-6 2v-8.5l-4.48 -4.928a2 2 0 0 1 -.52 -1.345v-2.227z" />"#,
                false,
            ),
            Icon::Lock => (
                r#"<path d="M5 13a2 2 0 0 1 2 -2h10a2 2 0 0 1 2 2v6a2 2 0 0 1 -2 2h-10a2 2 0 0 1 -2 -2v-6z" /> <path d="M11 16a1 1 0 1 0 2 0a1 1 0 0 0 -2 0" /> <path d="M8 11v-4a4 4 0 1 1 8 0v4" />"#,
                false,
            ),
            Icon::Leave => (
                r#"<path d="M14 8v-2a2 2 0 0 0 -2 -2h-7a2 2 0 0 0 -2 2v12a2 2 0 0 0 2 2h7a2 2 0 0 0 2 -2v-2" /> <path d="M9 12h12l-3 -3" /> <path d="M18 15l3 -3" />"#,
                false,
            ),
            Icon::Play => (r#"<path d="M7 4v16l13 -8z" />"#, false),
            Icon::Shield => (
                r#"<path d="M12 3a12 12 0 0 0 8.5 3a12 12 0 0 1 -8.5 15a12 12 0 0 1 -8.5 -15a12 12 0 0 0 8.5 -3" />"#,
                false,
            ),
            Icon::Plus => (r#"<path d="M12 5l0 14" /> <path d="M5 12l14 0" />"#, false),
            Icon::ArrowUp => (
                r#"<path d="M12 5l0 14" /> <path d="M18 11l-6 -6" /> <path d="M6 11l6 -6" />"#,
                false,
            ),
            Icon::Link => (
                r#"<path d="M9 15l6 -6" /> <path d="M11 6l.463 -.536a5 5 0 0 1 7.071 7.072l-.534 .464" /> <path d="M13 18l-.397 .534a5.068 5.068 0 0 1 -7.127 0a4.972 4.972 0 0 1 0 -7.071l.524 -.463" />"#,
                false,
            ),
            Icon::Copy => (
                r#"<path d="M7 9.667a2.667 2.667 0 0 1 2.667 -2.667h8.666a2.667 2.667 0 0 1 2.667 2.667v8.666a2.667 2.667 0 0 1 -2.667 2.667h-8.666a2.667 2.667 0 0 1 -2.667 -2.667z" /> <path d="M4.012 16.737a2.005 2.005 0 0 1 -1.012 -1.737v-10c0 -1.1 .9 -2 2 -2h10c.75 0 1.158 .385 1.5 1" />"#,
                false,
            ),
            Icon::BellOff => (
                r#"<path d="M9.346 5.353c.21 -.129 .428 -.246 .654 -.353a2 2 0 1 1 4 0a7 7 0 0 1 4 6v3m-1 3h-13a4 4 0 0 0 2 -3v-3a6.996 6.996 0 0 1 1.273 -3.707" /> <path d="M9 17v1a3 3 0 0 0 6 0v-1" /> <path d="M3 3l18 18" />"#,
                false,
            ),
            Icon::Flag => (
                r#"<path d="M5 5a5 5 0 0 1 7 0a5 5 0 0 0 7 0v9a5 5 0 0 1 -7 0a5 5 0 0 0 -7 0v-9z" /> <path d="M5 21v-7" />"#,
                false,
            ),
            Icon::Trash => (
                r#"<path d="M4 7l16 0" /> <path d="M10 11l0 6" /> <path d="M14 11l0 6" /> <path d="M5 7l1 12a2 2 0 0 0 2 2h8a2 2 0 0 0 2 -2l1 -12" /> <path d="M9 7v-3a1 1 0 0 1 1 -1h4a1 1 0 0 1 1 1v3" />"#,
                false,
            ),
            Icon::Language => (
                r#"<path d="M4 5h7" /> <path d="M9 3v2c0 4.418 -2.239 8 -5 8" /> <path d="M5 9c0 2.144 2.952 3.908 6.7 4" /> <path d="M12 20l4 -9l4 9" /> <path d="M19.1 18h-6.2" />"#,
                false,
            ),
            Icon::Ban => (
                r#"<path d="M3 12a9 9 0 1 0 18 0a9 9 0 1 0 -18 0" /> <path d="M5.7 5.7l12.6 12.6" />"#,
                false,
            ),
            Icon::VolumeOff => (
                r#"<path d="M15 8a5 5 0 0 1 1.912 4.934m-1.377 2.602a5 5 0 0 1 -.535 .464" /> <path d="M17.7 5a9 9 0 0 1 2.362 11.086m-1.676 2.299a9 9 0 0 1 -.686 .615" /> <path d="M9.069 5.054l.431 -.554a.8 .8 0 0 1 1.5 .5v2m0 4v8a.8 .8 0 0 1 -1.5 .5l-3.5 -4.5h-2a1 1 0 0 1 -1 -1v-4a1 1 0 0 1 1 -1h2l1.294 -1.664" /> <path d="M3 3l18 18" />"#,
                false,
            ),
        }
    }

    /// A complete SVG document for the icon, drawn in white so it can be tinted.
    pub fn svg(self) -> String {
        let (body, filled) = self.body();
        let paint = if filled {
            r##"fill="#ffffff" stroke="none""##
        } else {
            r##"fill="none" stroke="#ffffff" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round""##
        };
        format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" {paint}>{body}</svg>"#
        )
    }
}
