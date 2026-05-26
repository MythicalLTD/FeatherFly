# FeatherFly

Web hosting daemon for [FeatherPanel](https://featherpanel.com).

## Status

**Experimental — not a Plesk replacement yet.**

Single-node daemon with Wings-compatible auth: one **node bearer token** for panel API calls, **JWTs** for browser downloads (and WebSockets/uploads planned). See [auth model](.cursor/plan/auth-model.md).

## Quick start

```bash
make build
cargo run --bin featherfly -- configure   # first-time setup
cargo run --bin featherfly                # start daemon
curl http://localhost:9090/health
```

## Development

```bash
make ci      # fmt, docs, clippy, test, build
make docs    # generate static documentation
make test
```

## Security and planning docs

- [Auth model (Wings-compatible)](.cursor/plan/auth-model.md)
- [Roadmap](.cursor/plan/featherd-roadmap.md)
- [Threat model](.cursor/plan/threat-model.md)
- [Security review notes](.cursor/plan/security-review.md)
- [Release blockers](.cursor/plan/release-blockers.md)
