import { testSimulation } from "@destack/test/world";
import { exerciseBlobStorage } from "./scenario/store.ts";
import { runR2 } from "./cloudflare/local.ts";
import { test } from "@destack/test";
import { join } from "node:path";
import { LocalBucket } from "../src/bun/index.ts";

test("keep, read, copy and delete blobs below a prefix of a local and an R2 bucket", async () => {
    await using simulation = testSimulation();
    const world = simulation.environment("host");
    const directory = await world.files.makeTemporaryDirectory("store-");
    await using local = await LocalBucket.open(join(directory, "local"), "space-test", world);
    await exerciseBlobStorage(local, world.random);
    await runR2("exerciseBlobStorage");
});
