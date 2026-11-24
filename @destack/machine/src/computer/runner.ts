import { principal } from "@destack/access";
import { assignment, placement } from "@destack/account/object";
import type { LocalGitEnvironment } from "@destack/git/local";
import type { Select } from "@destack/db";
import { type Controller, Reconciliation } from "@destack/db/database";
import { Failure } from "@destack/error";
import { CallError, type Follow, ObjectRemote, type Shape } from "@destack/object";
import { type Identifier, type JsonValue, present, schema } from "@destack/schema";
import { Scope, Subject } from "@destack/sync";
import type * as sync from "@destack/sync";
import type { Workflow } from "@destack/workflow";
import type { Machine } from "../machine/index.ts";
import { MachineError } from "../error/index.ts";
import type { Builder } from "./build.ts";

/** The placed steps this machine runs at once, two beside a laptop's other work. */
const RUN_CONCURRENCY = 2;

/** What a placed step's function runs with on this machine. */
export interface RunnerContext {
    /** The machine running it. */
    readonly machine: Machine;
    /** The space the step's run is in. */
    readonly scope: Identifier<"space">;
    /** The builder compiling on this machine. */
    readonly builder: Builder;
    /** A new directory the function works in, removed after it. */
    readonly directory: string;
    /** The host the function works on. */
    readonly environment: LocalGitEnvironment & Pick<Machine["environment"], "scheduler">;
    /** Make one call as the run's actor, through the run's space. */
    readonly call: (call: sync.Call) => Promise<JsonValue>;
    /** The signal stopping the function once its step ended elsewhere. */
    readonly signal: AbortSignal;
}

/** A function placed steps run by name: the workflow placing it, and its run. */
export interface RunnerFunction {
    /** The workflow whose steps place the function: its parts free of its input and output. */
    readonly workflow: Pick<Workflow, "name" | "package" | "step" | "placed">;
    /** Run it from its recorded input to its output. */
    readonly run: (input: JsonValue, context: RunnerContext) => Promise<JsonValue>;
}

/** The machine's runner: it follows the steps placed on it and runs each by its function. */
export class Runner {
    /** The machine whose replicas list the steps. */
    readonly #machine: Machine;
    /** The functions the steps run, by name. */
    readonly #functions: Readonly<Record<string, RunnerFunction>>;
    /** What the functions run with, apart from the step's own call, directory and signal. */
    readonly #context: Pick<RunnerContext, "builder" | "environment">;
    /** The directory of the functions' working directories. */
    readonly #directory: string;

    /** Run the steps a machine replicates through the functions it has. */
    constructor(
        machine: Machine,
        functions: Readonly<Record<string, RunnerFunction>>,
        context: Pick<RunnerContext, "builder" | "environment">,
        directory: string,
    ) {
        // set the machine, the functions, their context and their directory
        this.#machine = machine;
        this.#functions = functions;
        this.#context = context;
        this.#directory = directory;
    }

    /** Follow each shape of work assigned to a machine in the spaces it serves or has work in. */
    static follows(
        machineId: Identifier<"machine">,
        shapes: readonly Shape<{ readonly runner: string }>[],
    ): Follow[] {
        const runner = Subject.key(principal.machine.reference(Scope.universe.id, machineId));

        return shapes.flatMap((shape) => [
            // follow the work in each space the machine serves
            shape.per(placement, {
                where: { machineId },
                key: async (row) => ({ key: { scope: row.spaceId, runner } }),
            }),
            // follow the work in the space of each placement the machine has work in
            shape.per(assignment, {
                where: { machineId },
                key: async (row, view) => {
                    const placed = await view
                        .read(placement.table)
                        .findFirst({ where: { id: row.placementId } });

                    return placed === undefined
                        ? undefined
                        : { key: { scope: placed.spaceId, runner } };
                },
            }),
        ]);
    }

    /** The workflows placing the functions, each once. */
    get workflows(): readonly RunnerFunction["workflow"][] {
        return [...new Set(Object.values(this.#functions).map((entry) => entry.workflow))];
    }

    /** The shapes of the steps placed on this machine's runner. */
    get shapes(): readonly Shape<{ readonly runner: string }>[] {
        return this.workflows.map((workflow) => workflow.placed);
    }

    /** Build a controller per workflow running its open placed steps, two at once. */
    controllers(): Controller[] {
        return this.workflows.map((workflow) => ({
            name: `destack.runner.${workflow.name}`,
            table: workflow.step.table,
            when: { function: { isNotNull: true }, completedAt: { isNull: true } },
            concurrency: RUN_CONCURRENCY,
            reconcile: (
                rows: readonly Select<Workflow["step"]["table"]>[],
                reconciliation: Reconciliation,
            ) => this.reconcile(workflow, rows, reconciliation),
        }));
    }

    /** Run an open placed step once, ending it with its function's output or failure. */
    async reconcile(
        workflow: RunnerFunction["workflow"],
        rows: readonly Select<Workflow["step"]["table"]>[],
        reconciliation: Reconciliation,
    ): Promise<void> {
        // take the reconciled step
        const [row] = rows;
        if (row === undefined) {
            return;
        }

        // call the step's methods in its space
        const spaceId = schema.identifier("space").parse(row.scope);
        const remote = await this.#machine.connection({ id: workflow.package.id }, spaceId);
        const { environment } = this.#machine;
        const run = (name: string, input: Record<string, JsonValue>) =>
            ObjectRemote.run(
                remote,
                workflow.step,
                name,
                { id: row.id, ...input },
                spaceId,
                environment,
            );

        // leave a step its space ended already, which a lagging replica still shows open
        const current = schema
            .looseObject({ completedAt: schema.number().nullable() })
            .parse(await run("get", {}));
        if (current.completedAt !== null) {
            return;
        }

        // run its function until the step ended elsewhere, ending it with the output or the failure
        await Reconciliation.runUntil(
            reconciliation,
            async () => {
                // stop once the replica shows the step ended elsewhere, such as by its run's cancel
                const replicated = await this.#machine.database
                    .snapshot()
                    .read(workflow.step.table)
                    .findFirst({ where: { id: row.id } });
                const isOpen = replicated !== undefined && replicated.completedAt === null;

                return isOpen
                    ? undefined
                    : MachineError.of("STEP_CANCELLED", `step '${row.id}' was cancelled`);
            },
            async (signal) => {
                // give the function its own directory and the step's calls
                const directory = this.#context.environment.files.join(this.#directory, row.id);
                const context: RunnerContext = {
                    ...this.#context,
                    machine: this.#machine,
                    scope: spaceId,
                    directory,
                    signal,
                    call: (call) => run("forward", { call: schema.jsonObject().parse(call) }),
                };
                // run the step's function, refusing one this workflow lacks
                try {
                    const work =
                        this.#functions[present(row.function, `the function of step '${row.id}'`)];
                    if (work?.workflow !== workflow) {
                        throw MachineError.of(
                            "FUNCTION_NOT_FOUND",
                            `function '${String(row.function)}' not found`,
                        );
                    }
                    const output = await work.run(row.input, context);
                    await run("complete", { output });
                } catch (error) {
                    // leave an aborted step open, and retry one its space refuses
                    if (signal.aborted || CallError.of(error)?.code === "SERVICE_UNAVAILABLE") {
                        throw error;
                    }
                    await run("fail", { failure: Failure.of(error) });
                } finally {
                    await this.#context.environment.files.remove(directory, {
                        recursive: true,
                        force: true,
                    });
                }
            },
        );
    }
}
