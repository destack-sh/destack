import { exerciseContentStore } from "./scenario/store.ts";
import { runR2 } from "./r2/local.ts";
import { test } from "@destack/test";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { LocalBucket } from "../src/local/index.ts";

test("keep, read, copy and delete blobs below a prefix of a local and an R2 bucket", async () => {
    const directory = await mkdtemp(join(tmpdir(), "destack-store-"));
    try {
        await using local = await LocalBucket.open(join(directory, "local"), "space-test");
        await exerciseContentStore(local);
        await runR2("exerciseContentStore");
    } finally {
        await rm(directory, { recursive: true });
    }
});
