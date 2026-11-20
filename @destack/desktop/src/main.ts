#!/usr/bin/env bun
import { BunEnvironment } from "@destack/environment/bun";
import { Desktop } from "./workload/index.ts";

/** Run the desktop in the system's environment, opening the link it launched with. */
export async function main(words: readonly string[]): Promise<void> {
    const [link] = words;
    await Desktop.start(BunEnvironment.system, link);
}

// run the desktop as a program
if (import.meta.main) {
    await main(BunEnvironment.system.process.arguments);
}
