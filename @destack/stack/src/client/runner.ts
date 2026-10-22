import type { ResourceDeclaration } from "@destack/resource";
import type { RunnerOptions, Workload } from "@destack/object/workload";
import { createClient } from "@destack/object/client";
import { Uplink } from "@destack/sync";
import stackPackage from "../package.ts";
import type { StackClient } from "./client.ts";
import type { Clock } from "@destack/environment";

/** Run a workload in a space with its package's resources at a clock. */
export function spaceRunner(
    workload: Workload,
    resources: Readonly<Record<string, ResourceDeclaration<unknown>>>,
    clock: Clock,
): Omit<RunnerOptions, "ephemeral"> {
    // connect once to each machine URL under each secret
    const clients = new Map<string, StackClient>();
    const connect = (url: string, secret: string) => {
        // reuse the client of a URL and secret
        const key = JSON.stringify([url, secret]);
        const known = clients.get(key);
        if (known !== undefined) {
            return known;
        }

        // connect to a new one
        const client = createClient(stackPackage, {
            url,
            headers: () => ({ authorization: `Bearer ${secret}` }),
        });
        clients.set(key, client);

        return client;
    };

    return {
        workload,
        resources,
        clock,
        publisher: (url, secret) => Uplink.of(() => connect(url, secret).store),
    };
}
