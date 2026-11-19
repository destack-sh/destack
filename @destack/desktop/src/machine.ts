#!/usr/bin/env bun
import { BunEnvironment } from "@destack/environment/bun";
import { Computer } from "@destack/machine/computer";
import { DESKTOP_OBJECTS } from "./object/index.ts";

// serve the computer in the system's environment
await Computer.run(
    {
        localObjects: DESKTOP_OBJECTS,
    },
    BunEnvironment.system,
);
