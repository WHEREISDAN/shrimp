# Contributing

Shrimp is a personal project, shared in case it's useful to someone else. It's in
maintenance mode: the plan is to fix bugs, not to add features. Issues and pull requests
are welcome, but replies may be slow and feature requests will most likely be declined.

## Reporting a bug

Open an issue with the bug report form. Include your operating system and version,
Shrimp's version (shown on the Help page), what you did, what you expected and
what happened. For security problems, follow [SECURITY.md](SECURITY.md) instead.

## Pull requests

- Keep a pull request to one fix, with a test where one is practical.
- Run `pnpm check` before opening it: formatting, lint, type check, Vitest, clippy and the
  Rust tests must pass.
- Follow the style of the code around your change. [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md)
  covers setup, testing and the dev app; [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)
  explains how the parts fit together.
- Commit messages use [Conventional Commits](https://www.conventionalcommits.org)
  (`fix: …`, `feat: …`, `docs: …`).

By contributing, you agree that your contribution is licensed under the
[MIT license](LICENSE).

## Builds and releases

There is no CI. Release builds need the maintainer's signing and notarization credentials,
so they're built locally with the scripts in `scripts/`; see
[docs/RELEASING.md](docs/RELEASING.md).
