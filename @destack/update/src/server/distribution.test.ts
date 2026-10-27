import { ReleaseChannel } from "../release/index.ts";
import { testHost } from "@destack/test/host";
import { join } from "node:path";
import { principal } from "@destack/access";
import { TestObjectStore } from "@destack/object/test";
import { Identifier } from "@destack/schema";
import { Scope } from "@destack/sync";
import { afterAll, beforeAll, expect, onTestFinished, refusal, test } from "@destack/test";
import { Distribution } from "../distribution/index.ts";
import { distribution } from "../object/index.ts";
import updatePackage from "../package.ts";
import { SignedRepository, type SigningKey } from "../publish/index.ts";
import { Release } from "../release/index.ts";
import { createRootKey, UpdateFixture } from "../test/index.ts";
import { distributionServer } from "./server.ts";

/** Offline signers shared as immutable key material. */
let rootKeys: SigningKey[];
/** The compiled fixture distribution, reporting version 2026.9.1. */
let fixture: string;

beforeAll(async () => {
    rootKeys = await Promise.all(Array.from({ length: 3 }, () => createRootKey(testHost().crypto)));
    fixture = await UpdateFixture.compile();
}, 10000);

afterAll(async () => {
    await testHost().files.remove(fixture, { recursive: true });
});

test("record a device's distribution once and stage the published release as the device, refusing another device", async () => {
    // run 2026.9.0 of a distribution following the fixture's repository
    await using repository = await UpdateFixture.open(
        fixture,
        rootKeys,
        "2026.9.1",
        UpdateFixture.world(),
    );
    const home = await repository.environment.files.makeTemporaryDirectory("destack-distribution-");
    onTestFinished(() => repository.environment.files.remove(home, { recursive: true }));
    const installed = new Distribution({
        home,
        repository: repository.options.repository,
        root: SignedRepository.encode(repository.root).toString(),
        running: new Release("2026.9.0", repository.target),
        executable: join(home, "bin", "destack"),
        applications: join(home, "Applications"),
        channel: new ReleaseChannel("stable"),
        environment: repository.environment,
    });

    // serve the distribution in a device's own scope, its controller recording and staging it
    const deviceId = Identifier.create("device", repository.environment);
    const device = principal.device.reference(Scope.universe.id, deviceId);
    await using served = await TestObjectStore.open({
        packages: [
            distributionServer({
                device: deviceId,
                host: { distribution: installed, activate: () => {} },
            }),
        ],
        scope: device,
        visibility: "private",
        simulation: repository.simulation,
    });
    await served.settle();

    // read the row as the device, and refuse another device
    const other = principal.device.reference(
        Scope.universe.id,
        Identifier.create("device", repository.environment),
    );
    const rows = await served.store.database.select().from(distribution.table);
    const refused = await refusal(
        served.as(other).client(updatePackage).distribution.list({ deviceId }),
    );
    expect({
        rows: rows.map((row) => [row.scope, row.installed, row.staged, row.status]),
        refused,
    }).toEqual({
        rows: [[deviceId, "2026.9.0", "2026.9.1", "staged"]],
        refused: ["NOT_FOUND", "SCOPE_NOT_FOUND", `scope ${deviceId} not found`],
    });
});
