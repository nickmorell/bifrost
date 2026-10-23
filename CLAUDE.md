# BiFrost: rules for working in this repo

BiFrost is a cross-platform (Windows, macOS, Linux) Bluesky desktop client in Rust with
iced. These rules come from the project owner and apply to every change, by people and
by Claude.

## Principles

- **Black boxes.** Each top-level module under `src/` does one job and exposes a small
  public surface. Callers use that surface and never reach into internals.
- **Shared flows through traits.** Where several things do the same job (API clients,
  auth backends, integrations, OS notification backends), there is one trait with
  default methods for the shared flow, and each implementation only supplies what
  differs. Don't copy a flow into a second place.
- **Thin app.** The app shell holds state and routes messages; logic lives in the black
  boxes it calls.
- **Components deal only with themselves.** A UI component takes plain data and returns
  UI plus messages. It never fetches, stores, or knows which screen it is on.
- **Theme is a black box.** Colours, sizes and fonts come from theme tokens only, never
  literals in views. Each theme is its own file. Dark, light and "match system" must
  all keep working.
- **Clearly typed, only what is necessary.** Typed errors, no `unwrap`/`expect`/`panic`
  outside tests, no dead code, no speculative features or options nobody asked for.
- **Same experience on all three OSes.** No fix that only works on one machine or one
  OS. OS differences live behind one function or trait with a version per OS
  (`#[cfg(target_os = ...)]`), never scattered through feature code.
- **Testable in isolation.** Every module has tests beside it (`#[cfg(test)] mod tests`)
  that use fakes for the network, keyring and OS.

## Where things go

| Path | What |
|---|---|
| `src/model` | BiFrost's data types (posts, profiles, feeds, lists, chats, preferences, deck columns, drafts, articles, collections, kept posts with their search language, top links and your stats, your privacy requests, the ATstore directory) and typed ids (`Did`, `AtUri`); plain data, no I/O, uses no other module. `api`, `storage`, `translate` and `moderation` build on it |
| `src/net` | Moving bytes over HTTP: the `Transport` trait (plain `Request` in, status, headers and body out, never more than the request's size cap), `Http` (reqwest) and `shared()`, the one pool every request uses except video playback, which runs on its own runtime. `socket` opens websockets (Streamplace's live chat). Tests use `net::fake::Fake`. Uses no other module |
| `src/auth` | Login, token refresh, the account list (`AuthBackend`, `AccountManager`); tokens are kept through `storage::secrets`. `xrpc::Xrpc` is the HTTP client atrium sends Bluesky and chat calls through, over `net` |
| `src/storage` | What BiFrost keeps on this computer: `settings`, `drafts`, `places` (where you were reading in each feed), `archive` (posts kept per account for local search and offline reading), `collections` (your private folders of posts), `atstore` (the ATstore directory as last read, in the cache folder), `secrets` (OS credential store, `SecretStore`) and `image_cache`. JSON files are written in the background through `later::Later` |
| `src/platform` | Every Windows/macOS/Linux difference, one function with a version per OS: `paths` (config and cache folders), `fonts`, `locale` (the system language), `clipboard`, `notify` (OS notifications behind the `Notifier` trait), `app_icon` (the app icon's pixels for the window and tray) and `tray` (the tray icon behind the `Tray` trait; `tray::system()` is `None` where there is no tray, and on Linux the icon runs on its own GTK thread). Uses no other module and no iced |
| `src/api` | XRPC calls returning `model` types; atrium types stay in here. One error type, `ApiError` (`friendly()` words it for people). Every other HTTP client implements `client::ApiClient` (base URL, headers, error wording) over a `net::Transport`. `streamplace` is the Streamplace node (live streams, past streams, the chat websocket). `posting` publishes a post or a thread of them. `privacy` reads and writes your Discover opt-out and logged-out visibility. `web::github` is BiFrost's own repository (the opt-in update check and the prefilled "Report a problem" bug form). `web::atstore` is the ATstore directory (atstore.fyi), read through its public XRPC calls. `articles` reads long-form articles (Standard.site, WhiteWind) straight from their authors' servers, found through the PLC directory or `did:web`. `service/` has one trait per feature (`ListsApi`, `ChatApi`, ...) that `Client` implements, so features can be tested with fakes |
| `src/components` | Reusable widgets, one folder each (`button/`, `avatar/`, `menu/`, `popover/`, ...), re-exported flat from `components`; the larger ones (`post_card`, `media`, `video_player`) are used through their module. Generic over the message type, take plain data (props structs, child elements, messages), use only `theme` and iced; each folder has `iced_test` tests |
| `src/features` | One folder per feature (`alerts/` (what pops up as an OS notification), `lists/`, `chat/`, `composer/`, `thread/`, `notifications/`, `settings/`, `profile/`, `search/`, `timeline/`, `deck/`, `login/`, `people/`, `feeds/`, `moderation/`, `drafts/` (kept posts, and scheduled ones sent while BiFrost runs), `live/` (Streamplace streams and their chat), `translate/` (translations under posts and the Translation settings), `prefs/` (your Bluesky preferences, changed at once and saved one at a time), `actions/` (like, repost, follow, block, mute, delete, pin and bookmark, shown at once and put back if Bluesky refuses, and who can reply to your posts), `articles/` (the long-form reader, Standard.site subscriptions and the Articles screen), `collections/` (private folders of posts kept on this computer), `privacy/` (the Discover and logged-out switches in Settings), `stats/` (your numbers for the last month, worked out from your kept posts), `atstore/` (browsing the ATstore directory of AT Protocol apps, and an app's page)): its state, messages (`Msg`), update and views. Talks to Bluesky only through its `api::service` trait, reports what the app must do as `Event`s, and its views return elements of its own `Msg`. A feature whose views must embed something only the app draws (a chat message carrying a post) instead has generic views over the app's message type, with a `Host` trait the app implements and a `lift` from `Msg`. Uses `api`, `components`, `media`, `model`, `moderation` and `theme`; never `app` or another feature. Tests beside it with a fake service |
| `src/app` | The iced app shell. `mod.rs` builds the `App` and routes each group of `Message`s to the file that handles it (`account`, `prefs`, `nav`, `window`, `keys`, `pictures`, `viewer`, `popups`, `actions`, `compose`, `interaction`); `message.rs` has `Message` and `Route`; `screens/` lays out the window and picks the screen; `cache.rs` keeps the shared post caches in step. `playback.rs` owns everything that plays (video, live streams, Bandcamp music): one at a time, and what each does when you leave its screen. `util.rs`, `post/`, `media.rs` and `video.rs` turn app state into component inputs, and `app/<feature>.rs` passes a feature its messages, acts on its events and places its views |
| `src/theme` | Theme tokens (`tokens.rs`), one file per palette (`dark.rs`, `light.rs`), "Match system" (`system.rs`) and style functions (`style/`). No globals: views get a `Look` from the app, styles read the palette from the iced theme. Uses only iced, and `platform::fonts` for the system font's name |
| `src/media` | Media files on this computer: `image` prepares images for upload (upright, no metadata, under the size limit), `files` gets a post's images and video from file dialogs, the clipboard or dropped files behind the `Attachments` trait (`Local` is the real one). Uses `api` and `platform`, no iced |
| `src/video` | Video playback (HLS, MPEG-TS, audio) and `music` (an MP3 song, as Bandcamp streams it); downloads through an `ApiClient` in `net.rs` |
| `src/integrations` | The `Integration` trait and each optional integration (off by default): the external media sources that turn links into cards (`bandcamp` also reads a release's tracks so the app can play them), `streamplace`, whose switch adds the Live screen, and `atstore`, whose switch adds the ATstore screen |
| `src/moderation.rs` | One job |
| `src/translate` | `Translator`: the translation services and their API keys, kept in a `SecretStore` and read off the window's thread; implements `api::service::TranslateApi` |
| `assets/` | Public assets only (fonts, icons, licences), embedded at build time. `assets/brand` holds the app icon and mark (see `docs/brand.md`); `build.rs` puts the icon into the Windows program |
| `tests/architecture.rs` | The module rules below, as tests |

New code goes where this table says it lives. Ask before moving existing code to a
different module.

## Rules the tests enforce

`tests/architecture.rs` fails `cargo test` when:

- a module uses another top-level module it is not allowed to (`ALLOWED`), or anything
  outside `app` uses `crate::app`;
- a feature uses another feature (`crate::features::<other>`); what features share
  lives in `features/mod.rs`;
- the theme uses anything in `platform` but `platform::fonts`;
- code outside `app`, `features`, `components`, `theme` and `video` uses iced;
- a file is over 400 lines. Files that were already over are listed in `OVERSIZED` and
  may only shrink. When you shrink one, the test fails until you lower its number to
  match; when it drops under 400, remove it from the list. Never raise a number to
  make a test pass: split the file.

Adding a module or a dependency between modules is a design decision. Update
`ALLOWED` and this file in the same change and say why in the commit message.

## Before calling a change done

```
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
```

All three must pass. Lint settings are in `Cargo.toml` under `[lints]`. Don't silence a
lint to get green: fix the code. When a lint really doesn't fit one spot, use
`#[expect(clippy::..., reason = "...")]` on the smallest item, with a real reason.

Claude Code runs these automatically through `.claude/settings.json`: `cargo fmt` after
every edit, and clippy plus the tests before finishing a turn.

## Working conventions

- Keep each change small and verifiable; one step of the plan per change.
- Never commit secrets, tokens or personal data. Tokens live in the OS keyring only.
- `Cargo.lock` is committed; update it only through cargo.
- Integrations with outside services are always off by default and toggled in Settings.
