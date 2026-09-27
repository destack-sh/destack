import { exerciseMultipart } from "./scenario/multipart.ts";
import { runR2 } from "./r2/local.ts";
import { test } from "@destack/test";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { LocalBucket } from "../src/local/index.ts";

test("replace multipart parts and publish identical local and R2 files", async () => {
    const directory = await mkdtemp(join(tmpdir(), "destack-multipart-"));
    try {
        await using local = await LocalBucket.open(join(directory, "local"));
        await exerciseMultipart(local);
        await runR2("exerciseMultipart");
    } finally {
        await rm(directory, { recursive: true });
    }
});
