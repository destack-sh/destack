import { WORKERD_COMPATIBILITY_DATE } from "@destack/package/runtime";
import { Miniflare } from "miniflare";
import { MemoryBuild } from "@destack/package/test";
import { schema } from "@destack/schema";
import { PackageMount } from "@destack/object";
import { START_PATH } from "@destack/object/cloudflare";
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
        bindings: { DESTACK_JURISDICTION: "local", MODULE: workload, SPACE, SECRET },
        durableObjects: { PARTITION: { className: "Partition", useSQLite: true } },
        workerLoaders: { LOADER: {} },
    });
    onTestFinished(() => worker.dispose());

    return worker;
}

test("leave a partition's WebSocket open in workerd while the host answers pings", async () => {
    const { crypto } = testSimulation().environment("machine");

    const worker = await start();
    const name = PartitionName.of({
        space: SPACE,
        installation: INSTALLATION,
        partition: PARTITION,
    });

    // start the echo workload in the installation's partition
    const workload: WorkloadStart = {
        instance: schema
            .identifier("instance")
            .parse("instance-01996ab0-0000-7000-8000-00000000f004"),
        scope: SPACE,
        installation: INSTALLATION,
        name: "echo",
        partition: PARTITION,
        bindings: {},
        settings: {},
        capabilities: [],
        secret: SECRET,
        egress: "https://space/.destack/egress",
        url: "https://space/installation",
        sampling: 0,
        root: "00".repeat(32),
    };
    const build = await MemoryBuild.write(new Map(), { role: "app", package: ECHO_PACKAGE }, crypto);
    const started: DurableObjectInstanceStart = {
        workload: { start: workload, manifest: build.manifest },
        space: SPACE,
        directory: "workerd",
        main: MODULE,
        modules: [MODULE],
    };
    const startup = await worker.dispatchFetch(`https://partition/${name}${START_PATH}`, {
        method: "POST",
        body: JSON.stringify(started),
    });
    await startup.arrayBuffer();

    // open a WebSocket at the echo service's route, offering its protocol
    const origin = await worker.ready;
    const url = new URL(`/${name}${PackageMount.path(ECHO_PACKAGE.id)}/open?id=socket-1`, origin);
    url.protocol = "ws:";
    const socket = testHost().network.websocket(url.href, "echo");
    await new Promise<void>((resolve, reject) => {
        socket.addEventListener("open", () => resolve());
        socket.addEventListener("close", (event) =>
            reject(new Error(`the partition closed the WebSocket: ${event.code} ${event.reason}`)),
        );
    });

    // send each message and read its answer in order
    const answers: string[] = [];
    const waiting: { count: number; resolve: () => void }[] = [];
    socket.addEventListener("message", (event) => {
        const data: unknown = event.data;
        answers.push(data instanceof ArrayBuffer ? new TextDecoder().decode(data) : String(data));
        for (const wait of waiting.filter((each) => each.count === answers.length)) {
            wait.resolve();
        }
    });
    const answered = (count: number) =>
        new Promise<void>((resolve) => {
            waiting.push({ count, resolve });
        });
    const ponged = answered(1);
    socket.send("ping");
    await ponged;
    const echoed = answered(4);
    for (const message of ["one", "two", "list"]) {
        socket.send(message);
    }
    await echoed;
    socket.close();

    expect([startup.status, socket.protocol, answers]).toEqual([
        200,
        "echo",
        ["pong", `${PARTITION} one`, `${PARTITION} two`, JSON.stringify(["socket-1"])],
    ]);
});
