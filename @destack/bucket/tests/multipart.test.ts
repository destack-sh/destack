import { testSimulation } from "@destack/test/world";
import { exerciseMultipart } from "./scenario/multipart.ts";
import { runR2 } from "./cloudflare/local.ts";
import { test } from "@destack/test";
import { join } from "node:path";
import { LocalBucket } from "../src/bun/index.ts";

test("replace multipart parts and publish identical local and R2 files", async () => {
    await using simulation = testSimulation();
    const world = simulation.environment("host");
    const directory = await world.files.makeTemporaryDirectory("multipart-");
    await using local = await LocalBucket.open(join(directory, "local"), "space-test", world);
    await exerciseMultipart(local);
    await runR2("exerciseMultipart");
});
