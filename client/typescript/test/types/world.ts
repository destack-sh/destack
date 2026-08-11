import type {
    Allocation,
    Blob,
    Branch,
    Connection,
    Debugger,
    Frame,
    Image,
    Instant,
    Moment,
    MemoryMap,
    Probe,
    Reference,
    Root,
    RunOutcome,
    Runtime,
    Snapshot,
    SpawnRuntimeOptions,
    Value,
    World,
} from "../../src/index.js";
import { Destack, Program } from "../../src/index.js";

declare const connection: Connection;
declare const blob: Blob;
declare const options: SpawnRuntimeOptions;
declare const allocation: Allocation;

const destack = new Destack(connection);
const created: Promise<World> = destack.daemon.createWorld();

async function exerciseWorld(): Promise<void> {
    const world = await created;
    const program = new Program(destack.blobs, blob);
    const runtime: Runtime = await world.spawnRuntime(program, options);
    const worldDebugger: Debugger = world.debugger;
    const value: Promise<Value> = runtime.invoke("main");
    const outcome: Promise<RunOutcome> = world.run("task");
    const resumed: Promise<RunOutcome> = runtime.resume(1n);
    const advanced: Promise<{ wall: Instant }> = world.clock.advance(1_000_000n);
    const moment: Promise<Moment> = world.moment();
    const branches: Promise<readonly Branch[]> = world.branches();
    const runtimes: Promise<Runtime[]> = world.runtimes();
    const image: Promise<Image> = world.capture("ready");
    const probes: Promise<readonly Probe[]> = worldDebugger.probes();
    const frames: Promise<readonly Frame[]> = worldDebugger.frames();
    const current = await moment;
    const memoryMap: Promise<MemoryMap> = worldDebugger.memoryMap(current);
    const allocations: AsyncGenerator<Allocation> = worldDebugger.allocations(current);
    const roots: AsyncGenerator<Root> = worldDebugger.roots(current);
    const references: AsyncGenerator<Reference> = worldDebugger.references(
        current,
        allocation.id,
    );

    const snapshot: Promise<Snapshot> = image.then((value) => value.snapshot());
    const restored: Promise<World> = snapshot.then((value) =>
        destack.daemon.restoreWorld(value),
    );

    runtime.remove();
    world.close();

    void value;
    void outcome;
    void resumed;
    void advanced;
    void moment;
    void branches;
    void runtimes;
    void image;
    void snapshot;
    void restored;
    void probes;
    void frames;
    void memoryMap;
    void allocations;
    void roots;
    void references;
}

void exerciseWorld;
