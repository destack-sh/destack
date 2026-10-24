import { exerciseMultipart } from "./scenario/multipart.ts";
import { runR2 } from "./cloudflare/local.ts";
import { test } from "@destack/test";
import { join } from "node:path";
import { LocalBucket } from "../src/bun/index.ts";
import { Simulation } from "@destack/environment/simulation";

/** The start of every test's world: 2026-10-24 at midnight UTC. */
const START = Date.UTC(2026, 9, 24);

test("replace multipart parts and publish identical local and R2 files", async () => {
    await using simulation = new Simulation({ seed: 1, start: START });
    const world = simulation.environment("host");
    const directory = await world.files.makeTemporaryDirectory("multipart-");
    await using local = await LocalBucket.open(join(directory, "local"), "space-test", world);
    await exerciseMultipart(local);
    await runR2("exerciseMultipart");
});
