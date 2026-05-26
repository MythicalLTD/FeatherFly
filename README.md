# FeatherFly

Bare HTTP daemon for CloudPanel status integration.

## Status

Experimental minimal daemon.

The daemon exposes health endpoints and authenticated system status using a single node bearer token.

## Quick Start

```bash
make build
cargo run --bin featherfly -- configure --panel-url https://panel.example.com --token change-me
cargo run --bin featherfly
curl http://localhost:9090/health
```

## Development

```bash
make ci
make test
```

## Security And Planning Docs

- [Auth model](.cursor/plan/auth-model.md)
- [Roadmap](.cursor/plan/featherd-roadmap.md)
- [Threat model](.cursor/plan/threat-model.md)
- [Security review notes](.cursor/plan/security-review.md)
- [Release blockers](.cursor/plan/release-blockers.md)
