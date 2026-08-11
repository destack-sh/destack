import type { ConditionSet } from "../_generated/artifact/core/condition.js";
import type * as program from "../_generated/program/value.js";
import type { FiberId } from "../_generated/program/fiber.js";
import type { ModuleId } from "../_generated/program/info.js";
import type * as debuggerService from "../_generated/runtime/service/debugger.js";
import type * as worldService from "../_generated/runtime/service/world.js";
import type { Entry } from "../_generated/runtime/machine/entry.js";
import type { WorkerId } from "../_generated/runtime/worker/worker.js";
import type { RunOutcome } from "../_generated/runtime/world/world/run.js";

import { Program } from "../program/program.js";

import type { World } from "./world.js";

/** Evaluation fields bound to one execution context. */
export type EvaluationOptions = Omit<
    debuggerService.EvaluateRequest,
    "worldId" | "target" | "expression"
>;

/** One Runtime hosted inside a World. */
export class Runtime {
    /** Owning World. */
    readonly world: World;
    /** World-local Runtime identity. */
    readonly id: worldService.Runtime["id"];
    /** Program instantiated by this Runtime. */
    readonly program: Program;
    /** Runtime configuration. */
    readonly options: worldService.Runtime["options"];
    /** Ambient Runtime environment. */
    readonly environment: worldService.Runtime["environment"];
    /** Active Program conditions. */
    readonly conditions: worldService.Runtime["conditions"];

    /** Bind one Runtime record to its World. */
    constructor(world: World, runtime: worldService.Runtime) {
        this.world = world;
        this.id = runtime.id;
        this.program = new Program(world.blobs, runtime.program);
        this.options = runtime.options;
        this.environment = runtime.environment;
        this.conditions = runtime.conditions;
    }

    /** Bind one known Worker identity to this Runtime. */
    worker(workerId: WorkerId): Worker {
        return new Worker(this, workerId);
    }

    /** Invoke one Program entrypoint. */
    async invoke(
        entry: string | Entry,
        values: readonly program.Value[] = [],
    ): Promise<program.Value> {
        const entrypoint = typeof entry === "string" ? { name: entry } : entry;
        const response = await this.world.client.invoke({
            worldId: this.world.id,
            runtimeId: this.id,
            entry: entrypoint,
            arguments: values,
        });

        return response.value;
    }

    /** Replace this Runtime's Program at a committed safepoint. */
    async reload(program: Program, conditions: ConditionSet): Promise<Runtime> {
        const response = await this.world.client.reloadRuntime({
            worldId: this.world.id,
            runtimeId: this.id,
            program: program.blob,
            conditions,
        });

        return new Runtime(this.world, response.value);
    }

    /** Pause this Runtime. */
    async pause(): Promise<void> {
        await this.world.debugger.pause({ kind: "runtime", runtimeId: this.id });
    }

    /** Resume one debugger-stopped Worker in this Runtime. */
    resume(workerId: WorkerId): Promise<RunOutcome> {
        return this.world.debugger.resume(this.id, workerId);
    }

    /** Evaluate one TS++ expression in this Runtime module. */
    evaluate(
        moduleId: ModuleId,
        expression: string,
        options: EvaluationOptions,
    ): Promise<debuggerService.Evaluation> {
        return this.world.debugger.evaluate(
            { kind: "module", runtimeId: this.id, moduleId },
            expression,
            options,
        );
    }

    /** Remove this Runtime from its World. */
    async remove(): Promise<void> {
        await this.world.removeRuntime(this.id);
    }
}

/** One Worker hosted inside a Runtime. */
export class Worker {
    /** Owning Runtime. */
    readonly runtime: Runtime;
    /** World-local Worker identity. */
    readonly id: WorkerId;

    /** Bind one Worker identity to its Runtime. */
    constructor(runtime: Runtime, id: WorkerId) {
        this.runtime = runtime;
        this.id = id;
    }

    /** Bind one known Fiber identity to this Worker. */
    fiber(fiberId: FiberId): Fiber {
        return new Fiber(this, fiberId);
    }

    /** Pause this Worker. */
    async pause(): Promise<void> {
        await this.runtime.world.debugger.pause({
            kind: "worker",
            runtimeId: this.runtime.id,
            workerId: this.id,
        });
    }

    /** Resume this debugger-stopped Worker. */
    resume(): Promise<RunOutcome> {
        return this.runtime.world.debugger.resume(this.runtime.id, this.id);
    }
}

/** One Fiber hosted inside a Worker. */
export class Fiber {
    /** Owning Worker. */
    readonly worker: Worker;
    /** Worker-local Fiber identity. */
    readonly id: FiberId;

    /** Bind one Fiber identity to its Worker. */
    constructor(worker: Worker, id: FiberId) {
        this.worker = worker;
        this.id = id;
    }

    /** Pause this Fiber. */
    async pause(): Promise<void> {
        await this.worker.runtime.world.debugger.pause({
            kind: "fiber",
            runtimeId: this.worker.runtime.id,
            workerId: this.worker.id,
            fiberId: this.id,
        });
    }
}

export type { ConditionSet, Entry, FiberId, ModuleId, RunOutcome, WorkerId };
export type { Value } from "../_generated/program/value.js";
