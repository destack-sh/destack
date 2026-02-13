# MIR Bench

Shared benchmark program registry for MIR fixtures.
This crate is consumed by the VM benchmarks and the compiler optimizer harness.

## Overview

Programs are defined as Rust metadata plus a matching MIR fixture.
Each program supplies default arguments, scale axes, tags, and an expected output validator.

| Location | Purpose |
|----------|---------|
| language/test/mirbench/src | program registry and metadata |
| language/test/fixtures/mirbench | MIR fixtures used by the programs |

## Guidelines

Defaults should be fast enough for quick validation runs.
Use benchmark runner profiles to scale heavier workloads instead of inflating defaults.
