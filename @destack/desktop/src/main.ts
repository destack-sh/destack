#!/usr/bin/env bun
import { BunEnvironment } from "@destack/environment/bun";
import { Desktop } from "./workload/index.ts";

// run the desktop in the system's environment
await Desktop.start(BunEnvironment.system);
