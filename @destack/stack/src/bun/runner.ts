import type { ResourceDeclaration } from "@destack/resource";
import { present } from "@destack/schema";
import { runWorkload } from "@destack/object/bun";
import type { EphemeralHost, Workload } from "@destack/object/workload";
import { connectBunSqlite } from "@destack/db/bun";
import { EphemeralStorage } from "@destack/object/server";
import { spaceRunner } from "../client/index.ts";
import { BunEnvironment } from "@destack/environment/bun";

/** Store a Bun instance's ephemeral objects in a memory database. */
export const bunEphemeral: EphemeralHost = {
    open: (tables) => connectBunSqlite(":memory:", tables, { environment: BunEnvironment.system }),
    channel: (durable) => EphemeralStorage.channel(durable),
};

/** Host a workload in this Bun process as its machine starts it, with its package's resources. */
export async function hostWorkload(
    workload: Workload,
    resources: Readonly<Record<string, ResourceDeclaration<unknown>>>,
): Promise<void> {
    // serve the workload with the stack client of its installation, on the socket its machine gives
    const environment = BunEnvironment.system;
    const { process: host } = environment;
    const socket = present(host.arguments[0], "the socket the machine passes the runner");
    await runWorkload(
        { ...spaceRunner(workload, resources, environment), ephemeral: bunEphemeral, environment },
        console[Symbol.asyncIterator](),
        async (ready) => {
            await host.stdout.write(`${JSON.stringify(ready)}\n`);
        },
        socket,
    );

    // exit once the workload drains
    host.exit(0);
}
