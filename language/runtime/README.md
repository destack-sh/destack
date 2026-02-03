# Runtime

The runtime is how Destack actually does anything meaningful beyond pure computation.
It wraps VM and/or native execution with scheduling, bindings, determinism, replay, snapshots, and policy.
Essentially, the runtime is where we marry Node/Bun/Deno-level semantics _with_ V8/JSC-runtime features, all in one place.

## Overview

Destack has a single runtime that can drive both VM and native execution (even simultaneously).
The runtime owns time, randomness, scheduling, external bindings, resource tracking, and GC coordination.
VM and native are "engines" that run until they yield back to the runtime (microtask-style).

## Components

The runtime is organized around subsystems that will sound familiar to V8 and JSC enjoyers, with some additional features for Destack:

| Component | Description |
|-----------|-------------|
| runtime | primary runtime instance and orchestration |
| scheduler | event loop, tasks, microtasks, and runnables |
| bindings | bindings and effects |
| time | virtual, monotonic, and wall clocks |
| random | deterministic streams and entropy control |
| memory | heap coordination and GC safepoints |
| replay | log schema, record, replay, and branching |
| snapshot | checkpoint capture and restore |
| platform | OS ("platform") integration and resources |
| telemetry | profiling and tracing |

## Execution Model

We follow traditional WHATWG (and Node) semantics for event loop scheduling.
The unit of scheduling is a task, which represents a macrotask in the event loop.
A microtask represents a Promise job and is drained after each task.

Tasks and microtasks carry a runnable and a resume value.
The runnable can be a VM continuation or a native continuation, and the scheduler treats them uniformly.
(This keeps one scheduling model for both VM and native execution.)

VM entry points are single threaded per isolate, and only run one "tick" at a time.
The runtime decides when to run, yield, and resume, and it owns the scheduling policy.

## Bindings and Platform

All external effects ("platform effects") go through runtime bindings.
Platform code implements the actual OS integration and resource stuff.
Blocking work yields through the scheduler and resumes through the same bindings as everything else.

## Determinism and Replay

Determinism is enforced by the runtime by controlling time, randomness, scheduling, and all other effects, all of which is captured in the "replay log" (when needed).
As the name implies, the replay log records everything we need to replay the program execution deterministically.
(There are things we cannot replay, in which case we just error. Sad.)

## Randomness

Randomness is split into deterministic "streams" so concurrent work does not cross-contaminate.
Each task and microtask gets a stable stream id, and user code can allocate its own stream ids explicitly.
Stream allocation itself is recorded as a replay event, so replays stay aligned even when streams are created dynamically.

## Snapshots

Snapshots capture heap state, task queues, clock state, random state, and resource mappings.
Snapshots are taken at safepoints where VM and native frames are in a resumable state.
Snapshots allow fast replay and time travel without re-executing from genesis.
