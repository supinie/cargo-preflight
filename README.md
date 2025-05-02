<p align="center">
  <img src="./preflight_ferris.png" alt="Preflight ferris" width="250" height="250"/>
</p>

# Preflight 🛫

Preflight is a custom Cargo subcommand to run local "CI" on certain Git actions.

Preflight's aim is to ensure that trivially broken commits don't reach your remote, wasting CI time, adding extra fix commits, and most importantly saving you the embarrasment.

Preflight runs as a git hook to automatically run checks on commit or push.

## _Warning: Preflight is currently in development, and will be subject to breaking changes._

# Installing Preflight

```
cargo install cargo-preflight
```

# Configuring Preflight

Preflight can be configured by running:

```
cargo preflight --config
```

which will open a configuration wizard to walk you through the available options.

Alteratively, Preflight can be manually configured by editing the global `~/.config/cargo-preflight/preflight.toml` configuration or local `<your repo>/.preflight.toml` configuration files.

## Possible Options

```toml
[[preflight]] # Create new table entry for different behaviours
run_when = [
    "commit",
    "push",
] # Default values: ["push"]

# List of branch names to run on, below is an example.
# If the list is empty (default), then it will run on any branch.
branches = [
    "main",
    "my_feature",
    "supinie_dev",
] # Default values: []

checks = [
    "fmt", # `cargo fmt -- --check`
    "clippy", # `cargo clippy -- -D warnings`
    "test", # `cargo test`
    "check_tests", # `cargo check --tests`
    "check_examples",  # `cargo check --examples`
    "check_benches", # `cargo check --benches`
    "unused_deps", # uses `cargo-shear`
] # Default values: ["fmt", "test"]

autofix = false # Enables autofix functionality (for fmt and clippy)

over_ride = false # Enables override functionality
```

## Example Config:

```toml
[[preflight]]
run_when = ["commit"]
branches = []
checks = [
    "fmt",
    "clippy",
]
autofix = true
over_ride = true

[[preflight]]
run_when = ["push"]
branches = [
    "main",
    "master",
]
checks = [
    "fmt",
    "clippy",
    "test",
    "unused_deps",
]
autofix = false
over_ride = false
```

# Using Preflight

Preflight can be enabled in a repository by running:

```
cargo preflight --init
```

Preflight can also be run as a one-off test with the `cargo preflight` command.

_Note: Currently, Preflight only supports Linux systems._

# Roadmap

- [x] Override if checks fail
- [x] Auto-fix failed checks (when applicable, ie. clippy, fmt)
- [x] Set which branch(es) Preflight will run against
- [x] Different checks for different hooks
- [ ] Check for secrets
- [ ] Check semver for libs
- [ ] Check multiple commits for last "stable" commit
- [ ] Run on `cargo publish`
- [ ] Automatically remove unused hooks
- [ ] Properly overwrite old hooks

These are in no particular order, and many will introduce breaking changes.
