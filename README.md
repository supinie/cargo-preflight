<p align="center">
  <img src="./preflight_ferris.png" alt="Preflight ferris" width="250" height="250"/>
</p>

# Preflight 🛫

Preflight is a custom Cargo subcommand to run local "CI" on certain Git actions.

Preflight's aim is to ensure that trivially broken commits don't reach your remote, wasting CI time, adding extra fix commits, and most importantly saving you the embarrasment.

Preflight runs as a git hook to automatically run checks on commit or push.

# Installing Preflight

```
cargo install cargo-preflight
```

# Using Preflight

Preflight can be enabled in a repository by running:

```
cargo preflight --init
```

Preflight can also be run as a one-off test with the `cargo preflight` command.

_Note: Currently, Preflight only supports Linux systems._

# Configuring Preflight

Preflight can be configured by running:

```
cargo preflight --config
```

which will open a configuration wizard to walk you through the available options.

Alteratively, Preflight can be manually configured by editing the global `~/.config/cargo-preflight/preflight.toml` configuration or local `<your repo>/.preflight.toml` configuration files.

## Possible Options

```toml
run_when = [
    "commit",
    "push",
] # Default values: ["push"]

checks = [
    "fmt",
    "clippy",
    "test",
    "check_tests",
    "check_examples",
    "check_benches",
    "unused_deps", # uses `cargo-shear`
] # Default values: ["fmt", "test"]
```

# Roadmap

- [ ] Add override if checks fail
- [ ] Add option to set which branch(es) Preflight will run against
- [ ] Check for secrets
- [ ] Check multiple commits for last "stable" commit
- [ ] Run on `cargo publish`
