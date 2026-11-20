#!/usr/bin/env bun
import { BunEnvironment } from "@destack/environment/bun";
import { Computer } from "@destack/machine/computer";
import { DESKTOP_OBJECTS } from "./object/index.ts";

/** Serve the computer in the system's environment, with the desktop's local objects. */
export async function main(): Promise<void> {
    await Computer.run({ localObjects: DESKTOP_OBJECTS }, BunEnvironment.system);
}

// serve the computer as a program
if (import.meta.main) {
    await main();
}
