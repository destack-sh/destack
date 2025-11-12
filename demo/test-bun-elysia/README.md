# Elysia with Bun runtime

This demo stitches together several of the more advanced examples from https://github.com/elysiajs/elysia/tree/main/example so you can poke at cookies, guards, typed bodies, custom parsers, server-sent streams, and WebSockets from a single Bun app.

## Development
Run the server locally with:
```bash
bun run dev
```
Then open http://localhost:3000/ to see a live summary of every handler.

## Featured endpoints
- `GET /` – shows live metrics captured via `state`, `derive`, and lifecycle hooks.
- `POST /profiles` – validates/normalizes the payload and also accepts `application/elysia-demo` bodies encoded as `id|username|tag,tag`.
- `GET /cookies/session` / `GET /cookies/session/verify` – showcases signed cookie issuance and verification.
- `GET /secure/profile?token=<secret>` and `POST /secure/audit?token=<secret>` – guard-protected routes with shared query validation.
- `GET /streams/time` – streams five timestamp events via the Response API.
- `WS /ws/echo` plus `GET /ws/publish/:message` – WebSocket echo/broadcast channel powered by `.ws` and `.publish()`.
- `GET /teapot` – custom response headers + status code reminiscent of the `response.ts` example.

## Feature modules
Each module exports its own `new Elysia()` plugin that the root `src/index.ts` composes via `.use(...)`:
- `src/features/metrics.ts` – shared state, derived helpers, and request bookkeeping composed into every handler.
- `src/features/profiles.ts` + `src/features/parser.ts` – typed body validation and the custom `application/elysia-demo` parser.
- `src/features/cookies.ts` – signed cookie creation + verification endpoints.
- `src/features/secure.ts` – guarded routes that reuse the shared schema and touch the metrics helpers.
- `src/features/streams.ts`, `src/features/websocket.ts`, `src/features/teapot.ts`, `src/features/root.ts` – streaming, WebSocket, teapot response, and landing-page summary grouped per feature.

Use `bun run dev` while editing to hot reload the Bun runtime as you experiment with each endpoint.
