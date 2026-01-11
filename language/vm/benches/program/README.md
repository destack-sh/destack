This directory contains the hand-authored MIR programs used by the VM benchmarks.
Each program lives in its own module with a sibling `.mir` source file, plus the
metadata needed to run, scale, and validate it.

Program modules should include default arguments, scale axes and tags, an
expected-value validator, and brief documentation that explains what the
workload models.
Keep benchmark defaults fast enough for quick validation runs; scale heavier runs
through the benchmark runner options instead of inflating defaults here.
