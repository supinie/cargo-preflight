# Preflight

Preflight is a custom Cargo subcommand to run local "CI" on certain Git actions.

Preflight's aim is to ensure that trivially broken commits don't reach your remote, wasting CI time, adding extra fix commits, and worst of all saving you the embarrasment.

Preflight runs as a git hook to automatically run checks on commit or push.

# Installing Preflight

_coming soon: `cargo add cargo-preflight`_

# Using Preflight

Preflight can be enabled in a repository by running:

```
cargo preflight --init
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
] # Default values: ["fmt", "test"]
```

# Roadmap

- [ ] Add override if checks fail
- [ ] Add option to set which branch(es) Preflight will run against
- [ ] Check for secrets
- [ ] Check multiple commits for last "stable" commit
- [ ] Run on `cargo publish`
