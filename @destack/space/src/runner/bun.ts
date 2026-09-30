import type { Resource } from "@destack/resource";
import { AuditEvent } from "@destack/audit";
import { createAuditClient } from "@destack/audit/client";
import { runWorkload } from "@destack/service/bun";
import type { Workload } from "@destack/service/workload";
import { RequestId } from "@destack/service/request";
import { connect } from "../client/index.ts";

/** Run a workload in this process as its host starts it, with its package's resources. */
export async function run(
    workload: Workload,
    resources: Readonly<Record<string, Resource<unknown>>>,
): Promise<void> {
    // serve the workload with the audit and space clients of its installation
    await runWorkload(
        {
            workload,
            resources,
            history: (url, secret) => {
                const client = createAuditClient({
                    url,
                    headers: () => ({ authorization: `Bearer ${secret}` }),
                });

                // parse each outbox event before sending its batch
                return {
                    ingest: (batch, options) =>
                        client.ingest(
                            { events: batch.events.map((event) => AuditEvent.parse(event)) },
                            options,
                        ),
                };
            },
            replicas: (url, secret) => {
                const client = connect({
                    url,
                    headers: () => ({ authorization: `Bearer ${secret}` }),
                });

                return {
                    async *stream(request, signal) {
                        yield* await client.replica.stream(request, { signal });
                    },
                };
            },
            runs: (url, start) => {
                const client = connect({
                    url,
                    headers: () => ({ authorization: `Bearer ${start.secret}` }),
                });

                // record each request as a run of the installation, under a new request unless delivered under one
                return {
                    send: async (request, delivery = {}) => {
                        await client.run.send(
                            {
                                spaceId: start.scope,
                                installation: start.installation,
                                requestId: delivery.requestId ?? RequestId.create(),
                                ...request,
                            },
                            delivery.signal === undefined ? {} : { signal: delivery.signal },
                        );
                    },
                };
            },
        },
        console[Symbol.asyncIterator](),
        async (ready) => {
            process.stdout.write(`${JSON.stringify(ready)}\n`);
        },
    );

    // exit once the workload drains
    process.exit(0);
}
