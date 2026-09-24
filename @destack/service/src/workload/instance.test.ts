import { expect, test } from "@destack/test";
import { defineWorkload } from "./workload.ts";
import { WorkloadInstance } from "./instance.ts";
import { defineService } from "../declare/service.ts";
import { defineSchedule } from "../schedule/index.ts";
import { hosting } from "../server/tests/fixture.ts";

/** The first declared fixture service. */
const first = defineService("first", {});
/** The second declared fixture service. */
const second = defineService("second", {});
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
                        services: [{ service: first, router: {}, authorize: async () => {} }],
                    };
                },
            }),
            { resources: hosting.resources, service: () => ({ ...hosting, drainTimeout: 100 }) },
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
                            authorize: async () => {},
                            route: async () => {
                                accepted.resolve();
                                await released.promise;

                                return new Response("complete");
                            },
                        },
                        { service: second, router: {}, authorize: async () => {} },
                    ],
                };
            },
        }),
        { resources: hosting.resources, service: () => ({ ...hosting, drainTimeout: 1000 }) },
    );

    // expose every declared service through the same runtime-independent dispatcher
    const probe = await instance.fetch(second, new Request("https://fixture.test/readyz"));
    expect([probe.status, await probe.text()]).toEqual([200, "ok\n"]);
    const request = instance.fetch(first, new Request("https://fixture.test/work"));
    await accepted.promise;
    const closing = instance.close();
    expect(instance.close()).toBe(closing);
    expect(events).toEqual(["shutdown"]);

    // hold resources until the accepted response is consumed
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
                            authorize: async () => {},
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
        { resources: hosting.resources, service: () => ({ ...hosting, drainTimeout: 5 }) },
    );

    // observe failures immediately while holding the cancelled request open
    const response = instance.fetch(first, new Request("https://fixture.test/work"));
    const responseFailure = expect(response).rejects.toMatchObject({
        name: "TimeoutError",
        message: "Service drain deadline exceeded.",
    });
    await accepted.promise;
    const closing = instance.close();
    const closeFailure = expect(closing).rejects.toMatchObject({
        message: "workload shutdown failed",
        errors: [{ name: "TimeoutError", message: "Service drain deadline exceeded." }],
    });
    await cancelled.promise;
    expect(events).toEqual([]);

    // release only after the handler stops accessing its resources
    released.resolve();
    await Promise.all([responseFailure, closeFailure]);
    expect(events).toEqual(["resources"]);
}, 1500);

test("reject a service implemented twice and release startup resources", async () => {
    const events: string[] = [];
    const implementation = { service: first, router: {}, authorize: async () => {} };
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
            { resources: hosting.resources, service: () => ({ ...hosting, drainTimeout: 1000 }) },
        ),
    ).rejects.toThrow(`duplicate workload service: ${first.package.id}/first`);
    expect(events).toEqual(["resources"]);
}, 1500);

test("run a declared schedule through the workload that implements it", async () => {
    const runs: string[] = [];
    await using instance = await WorkloadInstance.start(
        defineWorkload({
            name: "fixture",
            start: () => ({
                services: [],
                schedules: [
                    {
                        schedule: nightly,
                        run: async (signal) => {
                            signal.throwIfAborted();
                            runs.push(nightly.name);
                        },
                    },
                ],
            }),
        }),
        { resources: hosting.resources, service: () => ({ ...hosting, drainTimeout: 1000 }) },
    );

    // run the implemented schedule and reject an unknown one
    await instance.run(nightly, new AbortController().signal);
    expect(runs).toEqual(["nightly"]);
    const unknown = defineSchedule({
        name: "weekly",
        timing: "cron",
        cron: "0 3 * * 1",
        timezone: "UTC",
        concurrency: "forbid",
        deadline: 60000,
    });
    expect(() => instance.run(unknown, new AbortController().signal)).toThrow(
        `unknown workload schedule: ${unknown.package.id}/weekly`,
    );
}, 1500);
