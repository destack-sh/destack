import { type ReplicaSource } from "@destack/sync";
import { expect, test } from "@destack/test";
import { defineWorkload } from "./workload.ts";
import { WorkloadInstance } from "./instance.ts";
import { defineService } from "../declare/service.ts";
import { defineSchedule, type ScheduleOccurrence } from "../schedule/index.ts";
import { defineWebhook, type WebhookDelivery } from "../webhook/index.ts";
import { hosting } from "../server/tests/fixture.ts";

/** The first declared fixture service. */
const first = defineService("first", {});
/** The second declared fixture service. */
const second = defineService("second", {});
/** An audit history keeping the batches it receives. */
const history = {
    batches: [] as unknown[],
    async ingest(batch: { readonly events: readonly unknown[] }) {
        history.batches.push(batch);
    },
};

/** The copies of a space, from a source streaming none to fixture workloads. */
const replicas = {
    scope: "space-01996ab0-0000-7000-8000-000000000001",
    source: {
        stream: () => {
            throw new Error("the fixture source streams no copies");
        },
    } satisfies ReplicaSource,
};

/** A declared fixture schedule. */
const nightly = defineSchedule({
    name: "nightly",
    timing: "cron",
    cron: "0 3 * * *",
    timezone: "UTC",
    concurrency: "forbid",
    deadline: 60000,
});

test("release startup resources when the workload shuts down during startup", async () => {
    const events: string[] = [];
    await expect(
        WorkloadInstance.start(
            defineWorkload({
                name: "fixture",
                start: (context) => {
                    context.defer(() => {
                        events.push("resources");
                    });
                    context.shutdown();

                    return {
                        services: [{ service: first, router: {} }],
                    };
                },
            }),
            {
                resources: hosting.resources,
                history,
                replicas,
                service: () => ({ ...hosting, drainTimeout: 100 }),
            },
        ),
    ).rejects.toMatchObject({ name: "AbortError" });
    expect(events).toEqual(["resources"]);
}, 1000);

test("start two services and drain accepted requests before shared cleanup", async () => {
    const events: string[] = [];
    const accepted = Promise.withResolvers<void>();
    const released = Promise.withResolvers<void>();
    const instance = await WorkloadInstance.start(
        defineWorkload({
            name: "fixture",
            start: async (context) => {
                context.defer(() => {
                    events.push("resources");
                });
                context.signal.addEventListener("abort", () => events.push("shutdown"));

                return {
                    services: [
                        {
                            service: first,
                            router: {},
                            route: async () => {
                                accepted.resolve();
                                await released.promise;

                                return new Response("complete");
                            },
                        },
                        { service: second, router: {} },
                    ],
                };
            },
        }),
        {
            resources: hosting.resources,
            history,
            replicas,
            service: () => ({ ...hosting, drainTimeout: 1000 }),
        },
    );

    // dispatch to every service
    const probe = await instance.fetch(second, new Request("https://fixture.test/readyz"));
    expect([probe.status, await probe.text()]).toEqual([200, "ok\n"]);
    const request = instance.fetch(first, new Request("https://fixture.test/work"));
    await accepted.promise;
    const closing = instance.close();
    expect(instance.close()).toBe(closing);
    expect(events).toEqual(["shutdown"]);

    // hold resources until the response is consumed
    released.resolve();
    const response = await request;
    expect(await response.text()).toBe("complete");
    await closing;
    expect(events).toEqual(["shutdown", "resources"]);
}, 1500);

test("retain shared resources until an overdue request observes cancellation", async () => {
    const accepted = Promise.withResolvers<void>();
    const cancelled = Promise.withResolvers<void>();
    const released = Promise.withResolvers<void>();
    const events: string[] = [];
    const instance = await WorkloadInstance.start(
        defineWorkload({
            name: "fixture",
            start: async (context) => {
                context.defer(() => {
                    events.push("resources");
                });

                return {
                    services: [
                        {
                            service: first,
                            router: {},
                            route: async (request: Request) => {
                                request.signal.addEventListener("abort", () => cancelled.resolve());
                                accepted.resolve();
                                await released.promise;
                                request.signal.throwIfAborted();

                                return new Response();
                            },
                        },
                    ],
                };
            },
        }),
        {
            resources: hosting.resources,
            history,
            replicas,
            service: () => ({ ...hosting, drainTimeout: 5 }),
        },
    );

    // observe failures while the request stays open
    const response = instance.fetch(first, new Request("https://fixture.test/work"));
    const responseFailure = expect(response).rejects.toMatchObject({
        name: "TimeoutError",
        message: "service drain deadline exceeded",
    });
    await accepted.promise;
    const closing = instance.close();
    const closeFailure = expect(closing).rejects.toMatchObject({
        message: "workload shutdown failed",
        errors: [{ name: "TimeoutError", message: "service drain deadline exceeded" }],
    });
    await cancelled.promise;
    expect(events).toEqual([]);

    // release after the handler stops
    released.resolve();
    await Promise.all([responseFailure, closeFailure]);
    expect(events).toEqual(["resources"]);
}, 1500);

test("reject a service implemented twice and release startup resources", async () => {
    const events: string[] = [];
    const implementation = { service: first, router: {} };
    await expect(
        WorkloadInstance.start(
            defineWorkload({
                name: "fixture",
                start: async (context) => {
                    context.defer(() => {
                        events.push("resources");
                    });

                    return { services: [implementation, implementation] };
                },
            }),
            {
                resources: hosting.resources,
                history,
                replicas,
                service: () => ({ ...hosting, drainTimeout: 1000 }),
            },
        ),
    ).rejects.toThrow(`duplicate workload service: ${first.package.id}/first`);
    expect(events).toEqual(["resources"]);
}, 1500);

test("deliver a schedule's occurrence and a webhook's delivery through the workload's triggers", async () => {
    const delivered: unknown[] = [];
    const github = defineWebhook({
        name: "github",
        verification: "github",
        route: "/",
    });
    await using instance = await WorkloadInstance.start(
        defineWorkload({
            name: "fixture",
            start: () => ({
                services: [],
                triggers: [
                    {
                        trigger: nightly,
                        handle: async (occurrence: ScheduleOccurrence, signal: AbortSignal) => {
                            signal.throwIfAborted();
                            delivered.push([nightly.name, occurrence.scheduledAt]);
                        },
                    },
                    github.handle(
                        async (delivery: WebhookDelivery, signal: AbortSignal) => {
                            signal.throwIfAborted();
                            delivered.push([github.name, delivery.id]);
                        },
                        { secret: async () => "secret" },
                    ),
                ],
            }),
        }),
        {
            resources: hosting.resources,
            history,
            replicas,
            service: () => ({ ...hosting, drainTimeout: 1000 }),
        },
    );

    // list the triggers in workload order
    expect(instance.triggers).toEqual([nightly, github]);
    const signal = new AbortController().signal;
    await instance.deliver(nightly, { scheduledAt: 1800000000000 }, signal);
    const delivery = {
        id: "delivery-1",
        event: "push",
        payload: {},
        parameters: {},
        receivedAt: 1,
    };
    await instance.deliver(github, delivery, signal);
    expect(delivered).toEqual([
        ["nightly", 1800000000000],
        ["github", "delivery-1"],
    ]);

    // reject an unimplemented trigger
    const weekly = defineSchedule({
        name: "weekly",
        timing: "cron",
        cron: "0 3 * * 1",
        timezone: "UTC",
        concurrency: "forbid",
        deadline: 60000,
    });
    expect(() => instance.deliver(weekly, { scheduledAt: 0 }, signal)).toThrow(
        `unknown workload schedule: ${weekly.package.id}/weekly`,
    );
}, 1500);

test("reject a trigger implemented twice and release startup resources", async () => {
    const events: string[] = [];
    const handler = { trigger: nightly, handle: async () => {} };
    await expect(
        WorkloadInstance.start(
            defineWorkload({
                name: "fixture",
                start: (context) => {
                    context.defer(() => {
                        events.push("resources");
                    });

                    return { services: [], triggers: [handler, handler] };
                },
            }),
            {
                resources: hosting.resources,
                history,
                replicas,
                service: () => ({ ...hosting, drainTimeout: 1000 }),
            },
        ),
    ).rejects.toThrow(`duplicate workload schedule: ${nightly.package.id}/nightly`);
    expect(events).toEqual(["resources"]);
}, 1500);

test("give a starting workload the host's audit history and its space's source of copies", async () => {
    // start a workload that delivers one batch to the history it receives and keeps the source
    let received: { readonly scope: string; readonly source: ReplicaSource } | undefined;
    await using instance = await WorkloadInstance.start(
        defineWorkload({
            name: "fixture",
            start: async (context) => {
                await context.history.ingest({ events: ["started"] });
                received = context.replicas;

                return { services: [] };
            },
        }),
        {
            resources: hosting.resources,
            history,
            replicas,
            service: () => ({ ...hosting, drainTimeout: 100 }),
        },
    );
    expect([instance.triggers, history.batches.at(-1), received]).toEqual([
        [],
        { events: ["started"] },
        replicas,
    ]);
});
