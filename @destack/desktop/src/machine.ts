#!/usr/bin/env bun
import { BunEnvironment } from "@destack/environment/bun";
import { Computer } from "@destack/machine/computer";
import { FORGE_INSTALLATION } from "@destack/forge/client";
import { DESKTOP_OBJECTS } from "./object/index.ts";

// serve the computer in the system's environment
await Computer.run(
    {
        localObjects: DESKTOP_OBJECTS,
        registry: FORGE_INSTALLATION,
    },
    BunEnvironment.system,
);
