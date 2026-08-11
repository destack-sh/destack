import type {
    ObservationPage,
    ReadRuntimeRequest,
    SpawnRuntimeRequest,
    Topology,
    WorldId,
} from "../_generated/runtime/service/world.js";
import type {
    Branch,
    BranchId,
} from "../_generated/runtime/world/lineage/branch.js";
import type { ImageId } from "../_generated/runtime/world/lineage/image.js";
import type { Moment } from "../_generated/runtime/world/lineage/moment.js";
import type { ObservationEntry } from "../_generated/runtime/world/observation/entry.js";
import type { ObservationQuery } from "../_generated/runtime/world/observation/query.js";
import type { Instant } from "../_generated/runtime/world/time/instant.js";
import type { Run, RunOutcome } from "../_generated/runtime/world/world/run.js";
import {
    WorldClient,
    worldService,
} from "../_generated/world/world.js";
import type { Daemon } from "../daemon/daemon.js";
import { Program } from "../program/program.js";

import { Debugger } from "./debugger.js";
import { Clock, Host, Random } from "./host.js";
import { Image } from "./image.js";
import { Policy } from "./policy.js";
import { Runtime } from "./runtime.js";

/** Runtime creation fields bound to one World. */
export type SpawnRuntimeOptions = Omit<
    SpawnRuntimeRequest,
    "worldId" | "program"
>;

/** One daemon-hosted Destack World. */
export class World {
    /** Daemon hosting this World. */
    readonly daemon: Daemon;
    /** Complete generated World service client. */
    readonly client: WorldClient;
    /** Daemon-hosted World identity. */
    readonly id: WorldId;

    /** World-bound debugger. */
    readonly debugger: Debugger;
    /** World-bound controllable host. */
    readonly host: Host;
    /** World-bound Clock. */
    readonly clock: Clock;
    /** World-bound Random source. */
    readonly random: Random;
    /** World-bound runtime Policy. */
    readonly policy: Policy;

    /** Bind one World identity to its daemon. */
    constructor(daemon: Daemon, id: WorldId) {
        this.daemon = daemon;
        this.client = new WorldClient(daemon.connection);
        this.id = id;

        this.debugger = new Debugger(this);
        this.host = new Host(this);
        this.clock = this.host.clock;
        this.random = this.host.random;
        this.policy = new Policy(this);
    }

    /** Shared RPC connection. */
    get connection() {
        return this.daemon.connection;
    }

    /** Shared immutable Blob operations. */
    get blobs() {
        return this.daemon.blobs;
    }

    // =============================================================================
    // World
    // =============================================================================

    /** Read this World's current Moment. */
    async moment(): Promise<Moment> {
        const response = await this.client.readMoment({ worldId: this.id });

        return response.value;
    }

    // =============================================================================
    // Runtime
    // =============================================================================

    /** Return one Runtime hosted by this World. */
    async runtime(runtimeId: ReadRuntimeRequest["runtimeId"]): Promise<Runtime> {
        const response = await this.client.readRuntime({
            worldId: this.id,
            runtimeId,
        });

        return new Runtime(this, response.value);
    }

    /** Spawn one Runtime from a Program. */
    async spawnRuntime(
        program: Program,
        options: SpawnRuntimeOptions,
    ): Promise<Runtime> {
        const response = await this.client.spawnRuntime({
            worldId: this.id,
            program: program.blob,
            ...options,
        });

        return new Runtime(this, response.value);
    }

    /** Return the Runtimes currently hosted by this World. */
    async runtimes(): Promise<Runtime[]> {
        const response = await this.client.listRuntimes({ worldId: this.id });

        return response.value.map((runtime) => new Runtime(this, runtime));
    }

    /** Remove one Runtime from this World. */
    async removeRuntime(runtimeId: ReadRuntimeRequest["runtimeId"]): Promise<void> {
        await this.client.removeRuntime({ worldId: this.id, runtimeId });
    }

    // =============================================================================
    // Execution
    // =============================================================================

    /** Run one unit of World work. */
    async run(run: Run): Promise<RunOutcome> {
        const response = await this.client.run({ worldId: this.id, run });

        return response.value;
    }

    // =============================================================================
    // Observation
    // =============================================================================

    /** Return one page of this World's Observations. */
    async observations(
        query: ObservationQuery,
        limit: number,
    ): Promise<ObservationPage> {
        const response = await this.client.listObservations({
            worldId: this.id,
            query,
            limit,
        });

        return response.value;
    }

    /** Watch this World's Observations after one sequence. */
    async *watchObservations(
        query: ObservationQuery = {},
    ): AsyncGenerator<ObservationEntry> {
        const call = this.client.watchObservations({ worldId: this.id, query });

        for await (const observation of call) {
            yield observation;
        }
        await call.response();
    }

    // =============================================================================
    // Topology
    // =============================================================================

    /** Read this World's Topology at one Moment. */
    async topology(moment?: Moment): Promise<Topology> {
        const response = await this.client.readTopology({
            worldId: this.id,
            moment,
        });

        return response.value;
    }

    // =============================================================================
    // Branch
    // =============================================================================

    /** Return one Branch in this World's lineage. */
    async branch(branchId: BranchId): Promise<Branch> {
        const response = await this.client.readBranch({
            worldId: this.id,
            branchId,
        });

        return response.value;
    }

    /** Return the Branches in this World's lineage. */
    async branches(): Promise<readonly Branch[]> {
        const response = await this.client.listBranches({ worldId: this.id });

        return response.value;
    }

    /** Rewind this World to one committed Moment. */
    async rewind(moment: Moment): Promise<void> {
        await this.client.rewind({ worldId: this.id, moment });
    }

    /** Fork one new World from a committed Moment. */
    async fork(moment: Moment, name: string): Promise<World> {
        const response = await this.client.fork({
            worldId: this.id,
            moment,
            name,
        });

        return this.daemon.world(response.value);
    }

    // =============================================================================
    // Image
    // =============================================================================

    /** Return one retained Image in this World's lineage. */
    async image(imageId: ImageId): Promise<Image> {
        const response = await this.client.readImage({
            worldId: this.id,
            imageId,
        });

        return new Image(this, response.value);
    }

    /** Return the retained Images in this World's lineage. */
    async images(): Promise<Image[]> {
        const response = await this.client.listImages({ worldId: this.id });

        return response.value.map((image) => new Image(this, image));
    }

    /** Capture one Image for this World. */
    async capture(name: string): Promise<Image> {
        const response = await this.client.capture({
            worldId: this.id,
            name,
        });

        return new Image(this, response.value);
    }

    // =============================================================================
    // Lifecycle
    // =============================================================================

    /** Close this World in its daemon. */
    async close(): Promise<void> {
        await this.daemon.closeWorld(this);
    }
}

export type {
    Branch,
    BranchId,
    ImageId,
    Instant,
    Moment,
    ObservationEntry,
    ObservationPage,
    ObservationQuery,
    Run,
    RunOutcome,
    Topology,
    WorldId,
};
export type { Entry, FiberId, ModuleId, Value, WorkerId } from "./runtime.js";
export * from "./debugger.js";
export * from "./host.js";
export * from "./image.js";
export * from "./policy.js";
export { Debugger, Host, Policy, Program, Runtime, WorldClient, worldService };
