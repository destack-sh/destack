import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterAll, beforeAll, expect, test } from "@destack/test";
import { Release } from "../release/index.ts";
import { createRootKey, UpdateFixture } from "../test/index.ts";
import { SignedRepository, type SigningKey } from "../publish/index.ts";
import { Distribution } from "./distribution.ts";

/** Offline signers shared as immutable key material. */
let rootKeys: SigningKey[];
/** The compiled fixture distribution, reporting version 2026.9.1. */
let fixture: string;

beforeAll(async () => {
    rootKeys = Array.from({ length: 3 }, createRootKey);
    fixture = await UpdateFixture.compile();
}, 10000);

afterAll(async () => {
    await rm(fixture, { recursive: true, force: true });
});

test("stage a newer release once, keeping it staged while none newer is published", async () => {
    // run 2026.9.0 of a distribution following the fixture's repository
    await using repository = await UpdateFixture.open(fixture, rootKeys, "2026.9.1");
    const home = await mkdtemp(join(tmpdir(), "destack-distribution-"));
    const distribution = new Distribution({
        channel: "stable",
        home,
        repository: repository.options.repository,
        root: SignedRepository.encode(repository.root).toString(),
        running: new Release("2026.9.0", repository.target),
        executable: join(home, "bin", "destack"),
        applications: join(home, "Applications"),
        applicationIdentifier: "app.destack.desktop",
        title: "Destack",
    });
    try {
        // stage the published release, then stage again without a newer one
        const first = await distribution.stage();
        const second = await distribution.stage();

        // expect the release staged both times, and the running one installed
        expect({
            first: first?.release.version,
            second: second?.release.version,
            staged: (await distribution.staged())?.release.version,
            installed: (await distribution.installed()).version,
            application: await distribution.application(),
        }).toEqual({
            first: "2026.9.1",
            second: "2026.9.1",
            staged: "2026.9.1",
            installed: "2026.9.0",
            application: undefined,
        });
    } finally {
        await rm(home, { recursive: true, force: true });
    }
});
