import type { ResourceDeclaration } from "@destack/resource";
import { present } from "@destack/schema";
import { runWorkload } from "@destack/runtime/bun";
import type { EphemeralHost, Workload } from "@destack/runtime/workload";
import { connectBunSqlite } from "@destack/db/bun";
import { EphemeralStorage } from "@destack/object/server";
import { spaceRunner } from "../client/index.ts";
import { Clock } from "@destack/environment";

/** Keep a Bun instance's ephemeral objects in a memory database. */
export const bunEphemeral: EphemeralHost = {
    open: (tables) => connectBunSqlite(":memory:", tables),
    channel: (durable) => EphemeralStorage.channel(durable),
};

/** Host a workload in this Bun process as its machine starts it, with its package's resources. */
export async function hostWorkload(
    workload: Workload,
    resources: Readonly<Record<string, ResourceDeclaration<unknown>>>,
): Promise<void> {
    // serve the workload with the stack client of its installation, on the socket its machine gives
    const socket = present(process.argv.at(2), "the socket the machine passes after the runner");
    await runWorkload(
        { ...spaceRunner(workload, resources, Clock.system), ephemeral: bunEphemeral },
        console[Symbol.asyncIterator](),
        async (ready) => {
            process.stdout.write(`${JSON.stringify(ready)}\n`);
        },
        socket,
    );

    // exit once the workload drains
    process.exit(0);
}
