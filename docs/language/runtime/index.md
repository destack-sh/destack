---
title: Runtime
description: Execution, effects, observability, and the integrated Destack runtime.
order: 30
---

# Runtime

The runtime is where code actually _runs_, and it's where pure computation touches the real world via our well defined `host` bindings.
This is nice because it means we get to capture and analyze all effects through a relatively thin well known boundary, which enables great observability and debugging.
And because Destack is a fully integrated stack, the runtime has been co-designed as part of the entire language toolchain, and the standard library takes advantage of this.
