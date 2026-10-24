import {
    exerciseContents,
    exerciseConditions,
    exerciseListing,
    exerciseGroups,
    exerciseMarkers,
} from "./scenario/bucket.ts";
import { runR2 } from "./cloudflare/local.ts";
import { test } from "@destack/test";
import { join } from "node:path";
import { LocalBucket } from "../src/bun/index.ts";
import { Simulation } from "@destack/environment/simulation";

/** The start of every test's world: 2026-10-24 at midnight UTC. */
const START = Date.UTC(2026, 9, 24);

/** Independent bucket behaviors exercised against both implementations. */
const cases = [
    { name: "preserve bodies, metadata, and content digests", run: exerciseContents },
    {
        name: "apply conditions and ranges without losing retained readers",
        run: exerciseConditions,
    },
    { name: "paginate literal keys and delete selected files", run: exerciseListing },
    { name: "paginate grouped prefixes while keys change", run: exerciseGroups },
    { name: "paginate past a folder marker key equal to the prefix", run: exerciseMarkers },
];

test.each(cases)("$name through local and R2 buckets", async ({ run }) => {
    await using simulation = new Simulation({ seed: 1, start: START });
    const world = simulation.environment("host");
    const directory = await world.files.makeTemporaryDirectory("bucket-");
    await using local = await LocalBucket.open(join(directory, "local"), "space-test", world);
    await run(local);
    await runR2(run.name);
});

test("count each R2 operation a bucket sends in the class R2 bills it in, each in exactly one take", async () => {
    await runR2("exerciseOperations");
});
