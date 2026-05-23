# FeatherFly

Web hosting daemon for [FeatherPanel](https://featherpanel.com).

## Build

```bash
cargo build --release
```

## Development

```bash
cp config.example.yml config.yml
make setup-hooks   # once: auto-rustfmt before every git commit
make debug
```

`make build`, `make test`, and other Make targets run `cargo fmt` automatically.

## Install (production)

```bash
cargo build --release
sudo ./target/release/featherfly install
sudo systemctl enable --now featherfly
```

`install` copies the binary to `/usr/local/bin/featherfly`, creates `/etc/featherfly/config.yml` if needed, and writes the systemd unit.

Use `--override` to replace an existing unit file.

## Downloads

Stable releases are published when you push a tag like `release-0.1.0`.

Nightly builds from `main` are published automatically:

- Nightly downloads: https://github.com/MythicalLTD/featherfly/releases/tag/nightly
- All releases: https://github.com/MythicalLTD/featherfly/releases

Pick the binary for your platform, for example `featherfly-x86_64-linux` or `featherfly-aarch64-linux`.

## Updates

FeatherFly can check GitHub for newer builds:

```bash
featherfly version --check
featherfly update
featherfly update --channel nightly
sudo featherfly update --apply --restart
```

Configure the default channel in `/etc/featherfly/config.yml`:

```yaml
updates:
  channel: stable   # stable | nightly | disabled
  check_on_startup: true
```

The daemon also exposes:

- `GET /api/system/update` — current update status
- `POST /api/system/upgrade` — panel-driven binary upgrade
- `GET /api/system/plugins` — loaded plugins

## Plugins

FeatherFly loads native plugins (`.so`) from the plugins directory at startup and exposes Minecraft-style event hooks.

Default path: `/var/lib/featherfly/plugins` (debug: `./data/plugins`)

```yaml
system:
  plugins_directory: /var/lib/featherfly/plugins

plugins:
  enabled: true
```

### Events

| Event | When |
|-------|------|
| `config.loaded` | Config file has been loaded |
| `plugin.loaded` | A plugin finished loading |
| `daemon.starting` | Daemon is about to start HTTP |
| `daemon.started` | HTTP server is listening |
| `daemon.stopping` | Daemon is shutting down |

Hooks run in load order. Return `HookResult::cancel()` to stop remaining handlers for that event.

### JSON mutation hooks

Plugins can modify JSON API responses and the `actions` step arrays panels use for follow-up work. Hook names and examples are auto-generated from `featherfly-plugin-sdk/src/metadata.rs`.

| Target | Input |
|--------|-------|
| `json.response` | Full response object |
| `json.actions` | The `actions` array only |

Regenerate docs after changing hooks or routes:

```bash
cargo run --bin generate-docs
# or
featherfly docs generate
```

Full reference on GitHub Pages: **https://mythicalltd.github.io/featherfly/** (plugin events, JSON hooks, Swagger). Preview locally with `make docs` and open `docs/index.html`.

### Build and ship

```bash
featherfly plugin build plugins/hello
featherfly plugin install plugins/hello
featherfly plugin ship plugins/hello

make plugin-ship PLUGIN=plugins/hello
```

### Author a plugin

```rust
use featherfly_plugin_sdk::{
    declare_plugin, hook, log_info, EventContext, HookResult, HostApi, PluginEvent,
};

extern "C" fn init(host: *const HostApi) -> i32 {
    hook!(host, PluginEvent::DaemonStarted, on_daemon_started);
    unsafe { log_info(host, "hello plugin ready") };
    0
}

extern "C" fn on_daemon_started(_ctx: *const EventContext) -> HookResult {
    HookResult::r#continue()
}

declare_plugin! {
    name: "hello",
    version: "0.1.0",
    init: init,
}
```

Example source: `plugins/hello/`

## Config

Production config path: `/etc/featherfly/config.yml`

See `config.example.yml`.

## API

Human-readable docs are **generated at deploy time** by `.github/workflows/docs.yml` and published to GitHub Pages — they are **not** committed to git or bundled in the daemon binary.

- **https://mythicalltd.github.io/featherfly/** — public docs (plugin hooks, Swagger, curl examples)
- `make docs` — generate into `docs/` for local preview only
- `GET /openapi.json` — machine-readable OpenAPI schema from a running daemon
- `GET /health` — no auth
- `GET /api/system` — bearer auth required (includes `actions`)
- `GET /api/system/update` — bearer auth required
- `GET /api/system/plugins` — loaded plugins, hooks, and JSON targets

## CLI

```bash
featherfly              # run daemon
featherfly --debug      # use ./config.yml
featherfly install      # systemd setup (root)
featherfly diagnostics
featherfly version
featherfly version --check
featherfly update
sudo featherfly update --apply --restart
featherfly plugin ship plugins/hello
make docs
featherfly docs generate
```

## Links

- https://featherpanel.com
- https://github.com/MythicalLTD/featherfly
- https://mythicalltd.github.io/featherfly
