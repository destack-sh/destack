import { testHost } from "@destack/test/host";
import { join } from "node:path";
import { RequestId } from "@destack/sync";
import { expect, refusal, test } from "@destack/test";
import { Distribution, Release, Stage } from "@destack/update";
import { distribution } from "@destack/update/object";
import updatePackage from "@destack/update/package";
import { SignedRepository } from "@destack/update/publish";
import { createRootKey, TestUpdate } from "@destack/update/test";
import { TestComputer } from "../test/index.ts";

/** The node of the repository origin that finds nothing. */
const REFUSING_NODE = "refusing";

/** The loopback host of the refusing origin, which updates call without HTTPS. */
const REFUSING_HOST = "127.0.0.1";

/** The node of the tests' homes, over the host's files. */
const FILES_NODE = "files";

test("record this machine's distribution, stage the published release and hand its activation over", async () => {
    // follow a repository publishing 2026.9.1 while 2026.9.0 runs
    const compiled = await TestUpdate.compile();
    const keys = await Promise.all(
        Array.from({ length: 3 }, () => createRootKey(testHost().crypto)),
    );
    const world = await TestComputer.world();
    await using repository = await TestUpdate.open(compiled, keys, "2026.9.1", world.simulation);
    const { files } = world.simulation.environment(FILES_NODE);
    const home = await files.makeTemporaryDirectory("destack-distribution-");
    const activations: string[] = [];
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
    try {
        // let the controller record the distribution and stage the release
        const opened = await TestComputer.open({
            world,
            distribution: {
                distribution: installed,
                activate: () => activations.push("activate"),
            },
        });
        const { reports, device } = opened;
        const { objects } = opened.computer;
        const isStaged = async () =>
            reports.length > 0 ||
            (await objects.database.select().from(distribution.table)).some(
                (recorded) => recorded.status === "staged",
            );
        await opened.waitUntil(objects.database, isStaged);
        const client = opened.connect(updatePackage);
        const [row] = (await client.distribution.list({ deviceId: device.id })).items;

        // activate it as the owner
        const activated = await client.distribution.activate({
            deviceId: device.id,
            requestId: RequestId.create(opened.environment),
            id: row?.id ?? "",
        });

        // expect the staged release recorded and its activation handed over once
        expect({
            reports,
            staged: [row?.installed, row?.staged, row?.status],
            activated: [activated.staged, activated.status],
            activations,
        }).toEqual({
            reports: [],
            staged: ["2026.9.0", "2026.9.1", "staged"],
            activated: ["2026.9.1", "activating"],
            activations: ["activate"],
        });
    } finally {
        await files.remove(home, { recursive: true, force: true });
        await files.remove(compiled, { recursive: true, force: true });
    }
}, 20_000);

test("report a controller's failed reconciliation to the machine: a stage its repository refuses", async () => {
    // follow a repository whose origin finds nothing, in the computer's world
    const world = await TestComputer.world();
    const { files } = world.simulation.environment(FILES_NODE);
    const home = await files.makeTemporaryDirectory("destack-distribution-");
    const keys = await Promise.all(
        Array.from({ length: 3 }, () => createRootKey(testHost().crypto)),
    );
    await using fixture = await TestUpdate.create(home, keys, world.simulation);
    const refusing = fixture.simulation.network.server(REFUSING_NODE).http({
        hostname: REFUSING_HOST,
        fetch: () => new Response(null, { status: 404 }),
    });
    try {
        const installed = new Distribution({
            home,
            repository: new URL("stable/", refusing.url),
            root: fixture.options.root,
            running: new Release("2026.9.0", "x86_64-unknown-linux-gnu"),
            executable: join(home, "bin", "destack"),
            applications: join(home, "Applications"),
            stage: new Stage("production"),
            environment: fixture.environment,
        });

        // let the controller try to stage and read the failure it reports first
        const { reported, device } = await TestComputer.open({
            world,
            distribution: { distribution: installed, activate: () => {} },
        });
        const first = await reported;
        const [recorded] = await device.database.select().from(distribution.table);
        const key = JSON.stringify({ id: recorded?.id });

        // expect the refused download reported with the controller and its distribution's key, and captured once
        expect({
            reported:
                first instanceof Error && first.cause instanceof Error
                    ? [first.message, first.cause.message]
                    : first,
            captured: world.telemetry.take().map((captured) => captured.message),
        }).toEqual({
            reported: [
                `distribution failed to reconcile ${key}`,
                "metadata refresh failed: metadata request failed: HTTP 404",
            ],
            captured: ["metadata refresh failed: metadata request failed: HTTP 404"],
        });
    } finally {
        await refusing.stop(true);
    }
}, 20_000);

test("refuse staging on a machine running no installed distribution", async () => {
    const opened = await TestComputer.open();
    const { device } = opened;
    const { objects } = opened.computer;
    const client = opened.connect(updatePackage);

    // record a distribution as the device and ask to stage it
    const now = opened.clock.now();
    const [row] = await objects.mutate(objects.as(distribution, now), {
        id: RequestId.create(objects.environment),
        calls: [
            distribution.call("create", {
                ...distribution.in(device.id),
                installed: "2026.9.0",
            }),
        ],
    });
    const refused = await refusal(
        client.distribution.stage({
            deviceId: device.id,
            requestId: RequestId.create(opened.environment),
            id: row?.id ?? "",
        }),
    );

    expect(refused).toEqual([
        "PRECONDITION_FAILED",
        "DEVELOPMENT_BUILD",
        "development builds cannot take published updates",
    ]);
});
