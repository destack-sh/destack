# Library

The Destack standard library.
Integrated packages for building full-stack applications with Destack.

## Packages

| Package | Description |
|---------|-------------|
| `entity` | Core entity system, events, and paths |
| `telemetry` | Logging, metrics, and tracing |
| `auth` | Authentication, roles, and permissions |
| `ui` | UI primitives, input handling, styling |
| `physics` | Physics simulation (rigid bodies, colliders, joints) |
| `animation` | Easing functions and transitions |
| `universe` | Spaces, users, social features |
| `web` | Web platform utilities |
| `mobile` | Mobile platform utilities |
| `gpu` | GPU compute utilities |
| `ai` | AI/ML integrations |
| `finance` | Financial primitives |
| `test` | Testing utilities |
| `napi` | N-API bindings exposing Rust toolchain to JS |

## Commands

```sh
just library/napi   # build napi bindings
just library/test   # run tests
just library/fmt    # format code
just library/lint   # lint code
```

