# Contributing to BiFrost

Thanks for helping. BiFrost is built by one person, so the process is kept small.

## Reporting a bug or asking for something

- **Bugs and requests:** [open an issue](https://github.com/nickmorell/bifrost/issues/new/choose) and pick
  the form that fits. The forms ask for what's needed to act on it.
- **Questions and ideas:** use [Discussions](https://github.com/nickmorell/bifrost/discussions).
- **Security problems:** report them privately, see [SECURITY.md](SECURITY.md).

New issues are read once a week. Each gets one label:

| Label | Means |
|---|---|
| `accepted` | A real bug, or a request that fits the direction. It goes into a monthly release when there's room. |
| `needs-info` | Can't reproduce it or it's unclear. Closes after 30 days without an answer. |
| `later` | Fits, but not now. |
| `out-of-scope` | On the [not planned](README.md#not-planned) list. Closed with a link to it. |
| `good first issue` | Small and well described, left for contributors. |

## What gets merged

BiFrost is a curated app, not a platform. The [roadmap](README.md#roadmap) and the
[not planned](README.md#not-planned) list in the README decide what fits. If you want to build something
bigger than a bug fix, open an issue or discussion first so nobody's work is wasted.

## Pull requests

Pull requests are welcome.

- **Keep them small:** one fix or one feature per pull request.
- **Follow [CLAUDE.md](CLAUDE.md).** Despite the name it's the rulebook for everyone: where code goes, which
  module may use which, and the house style. `tests/architecture.rs` checks most of it.
- **Run the checks** before you push; CI runs the same ones on Windows, macOS and Linux:

  ```sh
  cargo fmt
  cargo clippy --all-targets -- -D warnings
  cargo test
  ```

- **Add a test** for a bug fix or new behaviour, beside the code it tests.
- **Integrations stay off by default.** Anything that contacts a new outside service needs a switch in Settings
  that starts off, and a row in the README's integrations table saying what it sends where.
- **No new telemetry, AI features or plugin systems** (see the not planned list).
- Write `Fixes #123` in the description when the pull request closes an issue.

### AI-assisted pull requests

Fine. BiFrost itself is built with a lot of help from Claude (see
[How BiFrost is made](README.md#how-bifrost-is-made)). What matters is that you have read the change, run it
and run the checks above, and that you can answer questions about it. Please say in the description which
tools you used.

## License

By contributing you agree that your work is licensed under the [Apache License, Version 2.0](LICENSE), like
the rest of BiFrost.
