# FeatherFly

Web hosting daemon for [FeatherPanel](https://featherpanel.com).

## Build

```bash
cargo build --release
```

## Development

```bash
cp config.example.yml config.yml
make debug
```

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

## Config

Production config path: `/etc/featherfly/config.yml`

See `config.example.yml`.

## API

- `GET /health` — no auth
- `GET /docs` — Swagger UI
- `GET /api/system` — bearer auth required
- `GET /api/system/update` — bearer auth required

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
```

## Links

- https://featherpanel.com
- https://github.com/MythicalLTD/featherfly
