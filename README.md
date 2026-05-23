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

## Config

Production config path: `/etc/featherfly/config.yml`

See `config.example.yml`.

## API

- `GET /health` — no auth
- `GET /docs` — Swagger UI
- `GET /api/system` — bearer auth required

## CLI

```bash
featherfly              # run daemon
featherfly --debug      # use ./config.yml
featherfly install      # systemd setup (root)
featherfly diagnostics
featherfly version
```

## Links

- https://featherpanel.com
- https://github.com/mythicalltd/featherfly
