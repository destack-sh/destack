import type { Environment } from "@destack/environment";
import type { ResourceDeclaration } from "@destack/resource";
import type { WorkloadRunnerOptions, Workload } from "@destack/object/workload";
import { ObjectConnection } from "@destack/object/client";

/** Run a workload in a space with its package's resources in an environment. */
export function spaceRunner(
    workload: Workload,
    resources: Readonly<Record<string, ResourceDeclaration<unknown>>>,
    environment: Environment,
): Omit<WorkloadRunnerOptions, "ephemeral" | "log"> {
    // connect once to each machine URL under each secret
    const connections = new Map<string, ObjectConnection>();
    const connect = (url: string, secret: string) => {
        // reuse the connection of a URL and secret
        const key = JSON.stringify([url, secret]);
        const known = connections.get(key);
        if (known !== undefined) {
            return known;
        }

        // connect to a new one, signing its upgrade and each call with the instance's secret
        const connection = ObjectConnection.open({
            url: ObjectConnection.url(url),
            authorize: async (request) => {
                const headers = new Headers(request.headers);
                headers.set("authorization", `Bearer ${secret}`);

                return new Request(request, { headers });
            },
            environment,
        });
        connections.set(key, connection);

        return connection;
    };

    return { workload, resources, environment, remote: connect };
}
