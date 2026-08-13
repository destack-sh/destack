---
title: Runtime
description: Execution, effects, observability, and the integrated Destack runtime.
order: 30
---

# Runtime

The runtime is where Destack (`.ds`) code actually _runs_, and it's the only place where "pure computation" touches the real world via our well defined `@binding`s.
This is nice because it means we get to capture and analyze all effects through a relatively thin well known boundary, which enables great observability and debugging.
And because Destack is a fully integrated stack, the runtime has been co-designed as part of the entire language toolchain, and the standard library takes advantage of this.
