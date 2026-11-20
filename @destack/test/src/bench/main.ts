#!/usr/bin/env bun
import { join } from "node:path";
import { BunEnvironment } from "@destack/environment/bun";
import { BenchRun } from "./bench.ts";

/** The system's environment. */
const environment = BunEnvironment.system;

/** The bench directory, the baseline to print and the one to compare against. */
const [directory, baseline, against] = environment.process.arguments;
if (directory === undefined || baseline === undefined) {
    throw new Error("usage: main.ts <bench directory> <baseline> [<base baseline>]");
}

/** The baseline's operations. */
const run = await BenchRun.read(join(directory, baseline), environment.files);

/** The base's operations, none without one. */
const base =
    against === undefined
        ? undefined
        : await BenchRun.read(join(directory, against), environment.files);

// print the table
environment.process.stdout.writeSync(
    new TextEncoder().encode(`${run.baseline}\n${run.table(base)}\n`),
);
