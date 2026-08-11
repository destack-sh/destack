import type { MemoryRange } from "../_generated/memory/address/map.js";
import type { BreakpointId } from "../_generated/program/outcome.js";
import type { WatchpointId } from "../_generated/program/watch.js";
import type { Breakpoint } from "../_generated/runtime/debugger/breakpoint.js";
import type {
    EvaluationLimits,
    EvaluationMode,
    EvaluationOutcome,
    EvaluationRepresentation,
    EvaluationTarget,
    EvaluationValue,
} from "../_generated/runtime/debugger/evaluation.js";
import type {
    Execution,
    Step,
} from "../_generated/runtime/debugger/execution.js";
import type * as debugFrame from "../_generated/runtime/debugger/frame.js";
import type {
    Allocation,
    AllocationId,
    Reference,
    Root,
    RootId,
    RootSource,
} from "../_generated/runtime/debugger/heap.js";
import type {
    MemoryMap,
    Region,
    RegionKind,
} from "../_generated/runtime/debugger/memory.js";
import type {
    Probe,
    ProbeAction,
    ProbeFilter,
    ProbeId,
} from "../_generated/runtime/debugger/probe.js";
import type { Watchpoint } from "../_generated/runtime/debugger/watchpoint.js";
import type { RuntimeId } from "../_generated/runtime/runtime.js";
import type * as debuggerService from "../_generated/runtime/service/debugger.js";
import type { WorkerId } from "../_generated/runtime/worker/worker.js";
import type { Moment } from "../_generated/runtime/world/lineage/moment.js";
import type { RunOutcome } from "../_generated/runtime/world/world/run.js";
import { DebuggerClient } from "../_generated/world/debugger.js";

import type { EvaluationOptions } from "./runtime.js";
import type { World } from "./world.js";

/** Frame query fields bound to one World. */
export type FrameOptions = Omit<debuggerService.ReadFramesRequest, "worldId">;

/** One World-bound Destack debugger. */
export class Debugger {
    /** Owning World. */
    readonly world: World;
    /** Complete generated Debugger service client. */
    readonly client: DebuggerClient;

    /** Bind one Debugger to its World. */
    constructor(world: World) {
        this.world = world;
        this.client = new DebuggerClient(world.connection);
    }

    // =============================================================================
    // Execution
    // =============================================================================

    /** Pause selected execution in this World. */
    async pause(execution: Execution = { kind: "world" }): Promise<void> {
        await this.client.pause({ worldId: this.world.id, execution });
    }

    /** Resume one debugger-stopped Worker. */
    async resume(runtimeId: RuntimeId, workerId: WorkerId): Promise<RunOutcome> {
        const response = await this.client.resume({
            worldId: this.world.id,
            runtimeId,
            workerId,
        });

        return response.value;
    }

    /** Step one debugger-stopped Frame. */
    async step(
        frameId: debugFrame.FrameId,
        step: Step,
    ): Promise<RunOutcome> {
        const response = await this.client.step({
            worldId: this.world.id,
            frameId,
            step,
        });

        return response.value;
    }

    // =============================================================================
    // Evaluation
    // =============================================================================

    /** Evaluate one TS++ expression in a selected execution context. */
    async evaluate(
        target: EvaluationTarget,
        expression: string,
        options: EvaluationOptions,
    ): Promise<debuggerService.Evaluation> {
        const response = await this.client.evaluate({
            worldId: this.world.id,
            target,
            expression,
            ...options,
        });

        return response.value;
    }

    // =============================================================================
    // Frame
    // =============================================================================

    /** Read captured Frames from this World. */
    async frames(options: FrameOptions = {}): Promise<Frame[]> {
        const response = await this.client.readFrames({
            worldId: this.world.id,
            ...options,
        });

        return response.value.map((frame) => new Frame(this.world, frame));
    }

    // =============================================================================
    // Memory
    // =============================================================================

    /** Read this World's mapped memory Regions. */
    async memoryMap(
        moment?: debuggerService.ReadMemoryMapRequest["moment"],
    ): Promise<MemoryMap> {
        const response = await this.client.readMemoryMap({
            worldId: this.world.id,
            moment,
        });

        return response.value;
    }

    /** Stream one exact memory range from this World. */
    async *memory(
        moment: Moment,
        range: MemoryRange,
    ): AsyncGenerator<Uint8Array> {
        const call = this.client.readMemory({
            worldId: this.world.id,
            moment,
            range,
        });

        // normalize every transport byte batch
        for await (const bytes of call) {
            yield bytes instanceof Uint8Array ? bytes : Uint8Array.from(bytes);
        }

        await call.response();
    }

    // =============================================================================
    // Heap
    // =============================================================================

    /** Stream managed heap Allocations at one exact Moment. */
    async *allocations(
        moment: Moment,
        after?: AllocationId,
    ): AsyncGenerator<Allocation> {
        const call = this.client.listAllocations({
            worldId: this.world.id,
            moment,
            after,
        });

        // flatten transport batches into semantic Allocations
        for await (const allocations of call) {
            for (const allocation of allocations) {
                yield allocation;
            }
        }

        await call.response();
    }

    /** Stream heap Roots at one exact Moment. */
    async *roots(moment: Moment, after?: RootId): AsyncGenerator<Root> {
        const call = this.client.listRoots({
            worldId: this.world.id,
            moment,
            after,
        });

        // flatten transport batches into semantic Roots
        for await (const roots of call) {
            for (const root of roots) {
                yield root;
            }
        }

        await call.response();
    }

    /** Stream outgoing References from one Allocation at one exact Moment. */
    async *references(
        moment: Moment,
        allocationId: AllocationId,
        after?: Reference["offset"],
    ): AsyncGenerator<Reference> {
        const call = this.client.listReferences({
            worldId: this.world.id,
            moment,
            allocationId,
            after,
        });

        // flatten transport batches into semantic References
        for await (const references of call) {
            for (const reference of references) {
                yield reference;
            }
        }

        await call.response();
    }

    // =============================================================================
    // Breakpoint
    // =============================================================================

    /** Return the Breakpoints installed in this World. */
    async breakpoints(
        moment?: debuggerService.ListBreakpointsRequest["moment"],
    ): Promise<readonly Breakpoint[]> {
        const response = await this.client.listBreakpoints({
            worldId: this.world.id,
            moment,
        });

        return response.value;
    }

    /** Add one Breakpoint to this World. */
    async addBreakpoint(filter: Breakpoint["filter"]): Promise<BreakpointId> {
        const response = await this.client.addBreakpoint({
            worldId: this.world.id,
            filter,
        });

        return response.value;
    }

    /** Replace one Breakpoint in this World. */
    async updateBreakpoint(breakpoint: Breakpoint): Promise<void> {
        await this.client.updateBreakpoint({
            worldId: this.world.id,
            breakpoint,
        });
    }

    /** Remove one Breakpoint from this World. */
    async removeBreakpoint(breakpointId: BreakpointId): Promise<void> {
        await this.client.removeBreakpoint({
            worldId: this.world.id,
            breakpointId,
        });
    }

    // =============================================================================
    // Watchpoint
    // =============================================================================

    /** Return the Watchpoints installed in this World. */
    async watchpoints(
        moment?: debuggerService.ListWatchpointsRequest["moment"],
    ): Promise<readonly Watchpoint[]> {
        const response = await this.client.listWatchpoints({
            worldId: this.world.id,
            moment,
        });

        return response.value;
    }

    /** Add one Watchpoint to this World. */
    async addWatchpoint(filter: Watchpoint["filter"]): Promise<WatchpointId> {
        const response = await this.client.addWatchpoint({
            worldId: this.world.id,
            filter,
        });

        return response.value;
    }

    /** Replace one Watchpoint in this World. */
    async updateWatchpoint(watchpoint: Watchpoint): Promise<void> {
        await this.client.updateWatchpoint({
            worldId: this.world.id,
            watchpoint,
        });
    }

    /** Remove one Watchpoint from this World. */
    async removeWatchpoint(watchpointId: WatchpointId): Promise<void> {
        await this.client.removeWatchpoint({
            worldId: this.world.id,
            watchpointId,
        });
    }

    // =============================================================================
    // Probe
    // =============================================================================

    /** Return the Probes installed in this World. */
    async probes(
        moment?: debuggerService.ListProbesRequest["moment"],
    ): Promise<readonly Probe[]> {
        const response = await this.client.listProbes({
            worldId: this.world.id,
            moment,
        });

        return response.value;
    }

    /** Add one Probe to this World. */
    async addProbe(filter: ProbeFilter, action: ProbeAction): Promise<ProbeId> {
        const response = await this.client.addProbe({
            worldId: this.world.id,
            filter,
            action,
        });

        return response.value;
    }

    /** Replace one Probe in this World. */
    async updateProbe(probe: Probe): Promise<void> {
        await this.client.updateProbe({ worldId: this.world.id, probe });
    }

    /** Remove one Probe from this World. */
    async removeProbe(probeId: ProbeId): Promise<void> {
        await this.client.removeProbe({ worldId: this.world.id, probeId });
    }
}

/** One captured execution Frame. */
export class Frame {
    /** World containing this Frame. */
    readonly world: World;
    /** Complete captured Frame record. */
    readonly record: debugFrame.Frame;

    /** Bind one captured Frame record to its World. */
    constructor(world: World, record: debugFrame.Frame) {
        this.world = world;
        this.record = record;
    }

    /** Frame identity. */
    get id(): debugFrame.FrameId {
        return this.record.id;
    }

    /** Canonical captured Frame bytes. */
    get bytes(): Uint8Array {
        return this.record.bytes instanceof Uint8Array
            ? this.record.bytes
            : Uint8Array.from(this.record.bytes);
    }

    /** Step this debugger-stopped Frame. */
    step(step: Step): Promise<RunOutcome> {
        return this.world.debugger.step(this.id, step);
    }

    /** Evaluate one TS++ expression in this Frame. */
    evaluate(
        expression: string,
        options: EvaluationOptions,
    ): Promise<debuggerService.Evaluation> {
        return this.world.debugger.evaluate(
            { kind: "frame", frame: this.id },
            expression,
            options,
        );
    }
}

export type {
    Allocation,
    AllocationId,
    Breakpoint,
    BreakpointId,
    EvaluationLimits,
    EvaluationMode,
    EvaluationOutcome,
    EvaluationRepresentation,
    EvaluationTarget,
    EvaluationValue,
    Execution,
    MemoryMap,
    MemoryRange,
    Probe,
    ProbeAction,
    ProbeFilter,
    ProbeId,
    Reference,
    Region,
    RegionKind,
    Root,
    RootId,
    RootSource,
    Step,
    Watchpoint,
    WatchpointId,
};
export { DebuggerClient };
export type { Evaluation } from "../_generated/runtime/service/debugger.js";
export { debuggerService } from "../_generated/world/debugger.js";
