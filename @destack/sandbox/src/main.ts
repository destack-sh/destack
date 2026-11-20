#!/usr/bin/env bun
import { BunEnvironment } from "@destack/environment/bun";
import { runLauncher } from "./launcher/index.ts";

/** Run the sandbox launcher in the system's environment. */
export async function main(): Promise<void> {
    await runLauncher(BunEnvironment.system);
}

// run the sandbox launcher as a program
if (import.meta.main) {
    await main();
}
