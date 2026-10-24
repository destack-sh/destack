#!/usr/bin/env bun
import { BunEnvironment } from "@destack/environment/bun";
import { main } from "./check/command.ts";

await main(BunEnvironment.system);
