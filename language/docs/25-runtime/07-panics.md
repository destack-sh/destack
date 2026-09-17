---
title: Panics
description: no exceptions, results for known unknowns
---

# Panics

- no exceptions, results for known unknowns
- but still need some way to model hard failures
- trap / abort .. abort.. but what  about panics for
- overflows / underflows
- out of bounds
- deliberate unreachable

- worker scoped
- a panic unwinds only the current Worker, running `using`, `await using`, `finally`, and `Drop` cleanup in reverse order
- the Worker terminates with a `Panic` containing its message, source location, and available stack trace; supervisors, tests, and simulation observe that termination through the Worker API
- catch unwind
- also useful for testing
- again like in Rust

- must-unwrapping a failure, explicit `panic`, overflow, out-of-bounds access, lone-surrogate string indexing, and reached `unreachable` code panic
