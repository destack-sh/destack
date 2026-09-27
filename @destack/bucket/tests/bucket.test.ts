import {
    exerciseContents,
    exerciseConditions,
    exerciseListing,
    exerciseGroups,
    exerciseMarkers,
} from "./scenario/bucket.ts";
import { runR2 } from "./r2/local.ts";
import { test } from "@destack/test";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { LocalBucket } from "../src/local/index.ts";

/** Independent bucket behaviors exercised against both implementations. */
const cases = [
    { name: "preserve bodies, metadata, and checksums", run: exerciseContents },
    {
        name: "apply conditions and ranges without losing retained readers",
        run: exerciseConditions,
    },
    { name: "paginate literal keys and delete selected files", run: exerciseListing },
    { name: "paginate grouped prefixes while keys change", run: exerciseGroups },
    { name: "paginate past a folder marker key equal to the prefix", run: exerciseMarkers },
];

test.each(cases)("$name through local and R2 buckets", async ({ run }) => {
    const directory = await mkdtemp(join(tmpdir(), "destack-bucket-"));
    try {
        await using local = await LocalBucket.open(join(directory, "local"));
        await run(local);
        await runR2(run.name);
    } finally {
        await rm(directory, { recursive: true });
    }
});
