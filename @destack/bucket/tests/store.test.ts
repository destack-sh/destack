import { exerciseBlobStore } from "./scenario/store.ts";
import { runR2 } from "./cloudflare/local.ts";
import { test } from "@destack/test";
import { join } from "node:path";
import { LocalBucket } from "../src/bun/index.ts";
import { Simulation } from "@destack/environment/simulation";

/** The start of every test's world: 2026-10-24 at midnight UTC. */
const START = Date.UTC(2026, 9, 24);

test("keep, read, copy and delete blobs below a prefix of a local and an R2 bucket", async () => {
    await using simulation = new Simulation({ seed: 1, start: START });
    const world = simulation.environment("host");
    const directory = await world.files.makeTemporaryDirectory("store-");
    await using local = await LocalBucket.open(join(directory, "local"), "space-test", world);
    await exerciseBlobStore(local, world.random);
    await runR2("exerciseBlobStore");
});
