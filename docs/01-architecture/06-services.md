---
title: Services
description: Services.
---

# Services

- `library/service` defines HTTP services and clients with oRPC and Zod.
- services declare operations, input and output schemas, and errors
- calls include authenticated identity, authorization, cancellation, and tracing
- Remote operations use OpenAPI-compatible HTTP endpoints; event streams use SSE; files use HTTP.
- Service packages expose separate client and server entrypoints with shared request and response schemas.
- Client imports do not load server implementations or credentials.
