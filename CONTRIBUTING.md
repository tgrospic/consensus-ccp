# Contributing to CCP

Read [`AGENTS.md`](AGENTS.md) before changing code. It defines the repository's authority order,
design rules, Rust conventions, and required checks.

## Development

Install Rust 1.91, `just`, and `cargo-nextest`, then run:

```sh
just build
just test
just clippy
just doc
```

Dependencies belong in the root workspace manifest. The `ccp` member inherits them with
`dependency.workspace = true`.

## Pull requests

A semantic change should include:

- the meaning being added or changed
- the minimal capabilities required by that meaning
- an extension or composition that derives the behavior
- a reusable law or focused invariant check
- an executable scenario using the public API
- documentation updates when terminology or architecture changes

Before submitting, run `just ci`.

Use Conventional Commits 1.0.0 for commit messages.
