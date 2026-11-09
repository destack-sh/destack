import { WORKERD_COMPATIBILITY_DATE } from "@destack/package/runtime";
import { Miniflare } from "miniflare";
import { MemoryBuild } from "@destack/package/test";
import { schema } from "@destack/schema";
import { PackageMount } from "@destack/object";
import { ALARM_PATH, START_PATH } from "@destack/object/cloudflare";
import type { WorkloadStart } from "@destack/object/workload";
import { testSimulation } from "@destack/object/test";
import { testHost } from "@destack/test/host";
import { expect, onTestFinished, test } from "@destack/test";
import { type DurableObjectInstanceStart, PartitionName } from "./instance.ts";
import { ECHO_PACKAGE } from "./test/package.ts";
import { bundleWorker } from "../test/bundle.ts";

/** The space whose object serves the echo workload's build. */
const SPACE = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-00000000f002");

/** The installation of the echo package. */
const INSTALLATION = schema
    .identifier("installation")
    .parse("installation-01996ab0-0000-7000-8000-00000000f003");

/** A second installation of the echo package, beside the first in the space's object. */
const SECOND_INSTALLATION = schema
    .identifier("installation")
    .parse("installation-01996ab0-0000-7000-8000-00000000f005");

/** The partition below the installation the scenario opens its WebSocket in. */
const PARTITION = "room-lobby";

/** The secret the partition's object and the workload prove each other's requests with. */
const SECRET = "partition-secret";

/** The module of the echo workload's build. */
const MODULE = "echo.js";

/** Run the scenario Worker with the partitions' one class and a loader of workloads. */
async function start(): Promise<Miniflare> {
    // bundle the echo workload and the scenario Worker
    const [workload, script] = await Promise.all([
        bundleWorker(new URL("./test/echo.ts", import.meta.url)),
        bundleWorker(new URL("./test/partition.ts", import.meta.url)),
    ]);

    // run the Worker, its space's object serving the workload's one module
    const worker = new Miniflare({
        modules: true,
        script,
        compatibilityDate: WORKERD_COMPATIBILITY_DATE,
        compatibilityFlags: ["nodejs_compat"],
        bindings: {
            DESTACK_JURISDICTION: "local",
            MODULE: workload,
            MODULE_NAME: MODULE,
            SPACE,
            SECRET,
        },
        durableObjects: { PARTITION: { className: "Partition", useSQLite: true } },
        workerLoaders: { LOADER: {} },
    });
    onTestFinished(() => worker.dispose());

    return worker;
}

/** Start the echo workload at a path of the scenario Worker, in a partition or not. */
async function startEcho(
    worker: Miniflare,
    path: string,
    placed: { readonly installation: string; readonly secret: string; readonly partition?: string },
): Promise<number> {
    // describe the start of the echo package's build
    const { crypto } = testSimulation().environment("machine");
    const workload: WorkloadStart = {
        instance: schema
            .identifier("instance")
            .parse("instance-01996ab0-0000-7000-8000-00000000f004"),
        scope: SPACE,
        installation: schema.identifier("installation").parse(placed.installation),
        name: "echo",
        ...(placed.partition === undefined ? {} : { partition: placed.partition }),
        bindings: {},
        settings: {},
        capabilities: [],
        secret: placed.secret,
        egress: "https://space/.destack/egress",
        url: "https://space/installation",
        sampling: 0,
        root: "00".repeat(32),
    };
    const build = await MemoryBuild.write(
        new Map(),
        { role: "app", package: ECHO_PACKAGE },
        crypto,
    );
    const started: DurableObjectInstanceStart = {
        workload: { start: workload, manifest: build.manifest },
        space: SPACE,
        main: MODULE,
    };

    // post the start, reading the answer to its end
    const startup = await worker.dispatchFetch(`https://partition/${path}${START_PATH}`, {
        method: "POST",
        body: JSON.stringify(started),
    });
    await startup.arrayBuffer();

    return startup.status;
}

/** Converse with the echo service below a path over a WebSocket, one answer per message. */
async function converse(
    worker: Miniflare,
    path: string,
    id: string,
    messages: readonly string[],
): Promise<{ readonly protocol: string; readonly answers: readonly string[] }> {
    // open a WebSocket at the echo service's route, offering its protocol
    const origin = await worker.ready;
    const url = new URL(`/${path}${PackageMount.path(ECHO_PACKAGE.id)}/open?id=${id}`, origin);
    url.protocol = "ws:";
    const socket = testHost().network.websocket(url.href, "echo");
    await new Promise<void>((resolve, reject) => {
        socket.addEventListener("open", () => resolve());
        socket.addEventListener("close", (event) =>
            reject(new Error(`the object closed the WebSocket: ${event.code} ${event.reason}`)),
        );
    });

    // send each message once the previous one answered, reading the answers in order
    const answers: string[] = [];
    for (const message of messages) {
        const answered = new Promise<void>((resolve) => {
            socket.addEventListener(
                "message",
                (event) => {
                    const data: unknown = event.data;
                    answers.push(
                        data instanceof ArrayBuffer ? new TextDecoder().decode(data) : String(data),
                    );
                    resolve();
                },
                { once: true },
            );
        });
        socket.send(message);
        await answered;
    }
    socket.close();

    return { protocol: socket.protocol, answers };
}

test("leave a partition's WebSocket open in workerd while the host answers pings", async () => {
    // start the echo workload in the installation's partition, and converse on a WebSocket
    const worker = await start();
    const name = PartitionName.of({
        space: SPACE,
        installation: INSTALLATION,
        partition: PARTITION,
    });
    const status = await startEcho(worker, name, {
        installation: INSTALLATION,
        secret: SECRET,
        partition: PARTITION,
    });
    const conversed = await converse(worker, name, "socket-1", ["ping", "one", "two", "list"]);

    expect([status, conversed]).toEqual([
        200,
        {
            protocol: "echo",
            answers: ["pong", `${PARTITION} one`, `${PARTITION} two`, JSON.stringify(["socket-1"])],
        },
    ]);
});

test("host two installations' workloads in their space's object, each answering and listing its own WebSockets", async () => {
    // start the echo workload of two installations in the space's object, each with its own secret
    const worker = await start();
    const installations = [INSTALLATION, SECOND_INSTALLATION];
    const statuses = await Promise.all(
        installations.map((installation) =>
            startEcho(worker, `${SPACE}/${installation}`, {
                installation,
                secret: `${SECRET}:${installation}`,
            }),
        ),
    );

    // converse with each on a WebSocket of its own, listing only its own sockets
    const conversed = await Promise.all(
        installations.map((installation, index) =>
            converse(worker, `${SPACE}/${installation}`, `socket-${index}`, ["ping", "hi", "list"]),
        ),
    );

    expect([statuses, conversed]).toEqual([
        [200, 200],
        installations.map((installation, index) => ({
            protocol: "echo",
            answers: ["pong", `${installation} hi`, JSON.stringify([`socket-${index}`])],
        })),
    ]);
});

test("move a facet's wake-up for the workload proving its installation's secret, and refuse another installation's", async () => {
    // start the echo workload of two installations in the space's object
    const worker = await start();
    const installations = [INSTALLATION, SECOND_INSTALLATION];
    await Promise.all(
        installations.map((installation) =>
            startEcho(worker, `${SPACE}/${installation}`, {
                installation,
                secret: `${SECRET}:${installation}`,
            }),
        ),
    );

    // move the first installation's wake-up with its own secret, then with the second's
    const move = async (secret: string) => {
        const response = await worker.dispatchFetch(
            `https://partition/${SPACE}/${INSTALLATION}${ALARM_PATH}`,
            {
                method: "POST",
                headers: { authorization: `Bearer ${secret}` },
                body: JSON.stringify({ at: 4_102_444_800_000 }),
            },
        );

        return [response.status, await response.json()];
    };
    const own = await move(`${SECRET}:${INSTALLATION}`);
    const foreign = await move(`${SECRET}:${SECOND_INSTALLATION}`);

    expect({ own, foreign }).toEqual({
        own: [200, {}],
        foreign: [
            401,
            {
                defined: true,
                code: "UNAUTHORIZED",
                status: 401,
                message: "invalid instance secret",
                reason: "INSTANCE_SECRET_INVALID",
                domain: "@destack/machine",
            },
        ],
    });
});
