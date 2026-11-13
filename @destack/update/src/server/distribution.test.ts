import { Stage } from "../release/index.ts";
import { testHost } from "@destack/test/host";
import { join } from "node:path";
import { principal } from "@destack/access";
import { TestObjectStore } from "@destack/object/test";
import { Identifier } from "@destack/schema";
import { Scope } from "@destack/sync";
import { afterAll, beforeAll, expect, onTestFinished, refusal, test } from "@destack/test";
import { Distribution } from "../distribution/index.ts";
import { checkForUpdates, distribution, restartToUpdate } from "../object/index.ts";
import updatePackage from "../package.ts";
import { SignedRepository, type SigningKey } from "../publish/index.ts";
import { Release } from "../release/index.ts";
import { createRootKey, TestUpdate } from "../test/index.ts";
import { distributionServer } from "./server.ts";
import { DistributionHost } from "./distribution.ts";

/** Offline signers shared as immutable key material. */
let rootKeys: SigningKey[];
/** The compiled fixture distribution, reporting version 2026.9.1. */
let fixture: string;

beforeAll(async () => {
    rootKeys = await Promise.all(Array.from({ length: 3 }, () => createRootKey(testHost().crypto)));
    fixture = await TestUpdate.compile();
});

afterAll(async () => {
    await testHost().files.remove(fixture, { recursive: true });
});

test("record a device's distribution once, stage the published release and offer restarting into it as the device, refusing another device", async () => {
    // run 2026.9.0 of a distribution following the fixture's repository
    await using repository = await TestUpdate.open(
        fixture,
        rootKeys,
        "2026.9.1",
        TestUpdate.world(),
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
        stage: new Stage("production"),
        environment: repository.environment,
    });

    // serve the distribution in a device's own scope, record it, and let its controller stage it
    const deviceId = Identifier.create("device", repository.environment);
    const device = principal.device.reference(Scope.universe.id, deviceId);
    const host = { distribution: installed, activate: () => {} };
    await using served = await TestObjectStore.open({
        packages: [distributionServer(host)],
        scope: device,
        visibility: "private",
        simulation: repository.simulation,
    });
    await DistributionHost.record(served.store, deviceId, host);
    await served.simulation.idle();

    // read the row and the update commands applying to it as the device, and refuse another device
    const offered = await Promise.all(
        [checkForUpdates, restartToUpdate].map(async (command) => {
            const { items } = await served
                .as(device)
                .client(updatePackage)
                .distribution.list({ deviceId, ...command.query() });

            return items.length === 1 ? [command.name] : [];
        }),
    );
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
        offered: offered.flat(),
        refused,
    }).toEqual({
        rows: [[deviceId, "2026.9.0", "2026.9.1", "staged"]],
        offered: ["restart-to-update"],
        refused: ["NOT_FOUND", "SCOPE_NOT_FOUND", `scope '${deviceId}' not found`],
    });
});
