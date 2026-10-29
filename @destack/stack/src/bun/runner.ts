import type { ResourceDeclaration } from "@destack/resource";
import { present } from "@destack/schema";
import { runWorkload, startWorkload } from "@destack/object/bun";
import type {
    EphemeralHost,
    RunnerOptions,
    Workload,
    WorkloadRunner,
    WorkloadStart,
} from "@destack/object/workload";
import type { BuildReader } from "@destack/package/manifest";
import { connectBunSqlite } from "@destack/db/bun";
import type { Environment } from "@destack/environment";
import { spaceRunner } from "../client/index.ts";
import { BunEnvironment } from "@destack/environment/bun";

/** A workload a Bun build hosts with its package's resources, as a process or in this one. */
export class BunWorkload {
    /** The workload. */
    readonly workload: Workload;
    /** The package's resources, by name. */
    readonly resources: Readonly<Record<string, ResourceDeclaration<unknown>>>;

    /** Host a workload with its package's resources. */
    constructor(
        workload: Workload,
        resources: Readonly<Record<string, ResourceDeclaration<unknown>>>,
    ) {
        this.workload = workload;
        this.resources = resources;
    }

    /** Build the workload's runner in an environment, its ephemeral objects in memory. */
    runner(environment: Environment): RunnerOptions {
        // store ephemeral objects in a memory database on the environment's clock
        const ephemeral: EphemeralHost = {
            open: (tables) => connectBunSqlite(":memory:", tables, { environment }),
        };

        return { ...spaceRunner(this.workload, this.resources, environment), ephemeral };
    }

    /** Start the workload in this process from its host's start, on an environment. */
    start(
        environment: Environment<"process">,
        start: WorkloadStart,
        build: BuildReader,
        report: (error: unknown) => void,
    ): Promise<WorkloadRunner> {
        return startWorkload({ ...this.runner(environment), environment }, start, build, report);
    }

    /** Run the workload as the process its machine starts, until it drains. */
    async run(): Promise<void> {
        // serve on the socket its machine gives, reading its start from standard input
        const environment = BunEnvironment.system;
        const { process: host } = environment;
        const socket = present(host.arguments[0], "the socket the machine passes the runner");
        await runWorkload(
            { ...this.runner(environment), environment },
            console[Symbol.asyncIterator](),
            async (ready) => {
                await host.stdout.write(`${JSON.stringify(ready)}\n`);
            },
            socket,
        );

        // exit once the workload drains
        host.exit(0);
    }
}

/** Host a workload in a Bun build with its package's resources, as its build's entry exports it. */
export function hostWorkload(
    workload: Workload,
    resources: Readonly<Record<string, ResourceDeclaration<unknown>>>,
): BunWorkload {
    return new BunWorkload(workload, resources);
}
