import { expect, test } from "@destack/test";
import { schema } from "@destack/schema";
import { Health } from "../health/index.ts";
import { createClient } from "../client/index.ts";
import { defineOperation, defineOperationProcedures } from "../operation/index.ts";
import { ServiceError } from "../error/index.ts";
import { implementOperation, OperationStore, Server } from "../server/index.ts";
import { hosting, createCaller } from "../server/tests/fixture.ts";

test.for([undefined, "/builds"] as const)(
    "run authorized operations through HTTP, reconnect, cancel, and release results (%s)",
    async (path) => {
        // define the operation and its limits
        const result = schema.object({ count: schema.number().int() });
        const progress = schema.object({ step: schema.string() });
        const definition = defineOperation(result, progress);
        await using store = new OperationStore(definition, {
            concurrency: 2,
            capacity: 3,
            retention: 60000,
            timeout: 1000,
        });

        // serve the procedures
        const service = defineOperationProcedures(definition, path);
        const router = implementOperation(store, path);
        const alice = createCaller("alice");
        await using server = Server.start({
            ...hosting,
            router,
            health: new Health("operations"),
            drainTimeout: 1000,
        });

        // serve the route
        const response = await server.fetch(
            new Request(`https://test.local${path ?? "/operations"}`, {
                headers: { authorization: "alice" },
            }),
        );
        expect(response.status).toBe(200);
        expect(await response.json()).toEqual([]);

        // connect two callers
        const client = createClient(service, {
            url: "https://test.local",
            headers: { authorization: "alice" },
            fetch: (request) => server.fetch(request),
        });
        const other = createClient(service, {
            url: "https://test.local",
            headers: { authorization: "bob" },
            fetch: (request) => server.fetch(request),
        });

        // start work
        const release = Promise.withResolvers<void>();
        const started = store.start(alice.id, { step: "compile" }, async ({ report }) => {
            await release.promise;
            report({ step: "complete" });

            return { count: 2 };
        });

        // isolate callers
        expect(await client.get({ id: started.id })).toEqual(started);
        expect(await other.list()).toEqual([]);
        await expect(other.get({ id: started.id })).rejects.toMatchObject({ code: "NOT_FOUND" });
        await expect(other.cancel({ id: started.id })).rejects.toMatchObject({ code: "NOT_FOUND" });
        await expect(client.delete({ id: started.id })).rejects.toMatchObject({ code: "CONFLICT" });

        // disconnect a subscriber
        const disconnect = new AbortController();
        const stream = await client.watch({ id: started.id }, { signal: disconnect.signal });
        expect(await stream.next()).toEqual({ done: false, value: started });
        disconnect.abort();
        expect(store.get(alice.id, started.id)).toEqual(started);

        // reconnect after completion
        release.resolve();
        const resumed = await client.watch({ id: started.id });
        const states = [];
        for await (const state of resumed) {
            states.push(state);
        }

        // keep the result until deletion
        expect(states.at(-1)).toEqual({
            ...started,
            updatedAt: expect.any(Number),
            completedAt: expect.any(Number),
            state: "succeeded",
            progress: { step: "complete" },
            result: { count: 2 },
        });
        expect(await client.get({ id: started.id })).toEqual(states.at(-1));
        await client.delete({ id: started.id });
        expect(await client.list()).toEqual([]);

        // cancel a running operation
        const entered = Promise.withResolvers<void>();
        let isCleaned = false;
        const cancelled = store.start(alice.id, { step: "wait" }, async ({ signal }) => {
            const aborted = Promise.withResolvers<void>();
            signal.addEventListener("abort", () => aborted.resolve(), { once: true });
            entered.resolve();
            try {
                await aborted.promise;
                signal.throwIfAborted();

                return { count: 0 };
            } finally {
                isCleaned = true;
            }
        });

        // request cancellation
        await entered.promise;
        const requested = await client.cancel({ id: cancelled.id });
        expect(requested.cancellationRequested).toBe(true);

        // observe cancellation after cleanup
        const cancellation = await client.watch({ id: cancelled.id });
        const outcomes = [];
        for await (const state of cancellation) {
            outcomes.push(state);
        }

        // keep the cancelled operation with its progress
        expect(isCleaned).toBe(true);
        expect(outcomes.at(-1)).toEqual({
            ...cancelled,
            updatedAt: expect.any(Number),
            completedAt: expect.any(Number),
            cancellationRequested: true,
            state: "cancelled",
        });
    },
);

test("enforce operation limits and retain successful work when cancellation loses the race", async () => {
    // limit the store to one operation
    await using store = new OperationStore(defineOperation(schema.string(), schema.number()), {
        concurrency: 1,
        capacity: 1,
        retention: 60000,
        timeout: 1000,
    });

    // hold publication until cancellation
    const entered = Promise.withResolvers<void>();
    const release = Promise.withResolvers<void>();
    const first = store.start("alice", 0, async () => {
        entered.resolve();
        await release.promise;

        return "published";
    });

    // reject concurrent work
    await entered.promise;
    try {
        expect(() => store.start("alice", 0, async () => "extra")).toThrow(
            expect.objectContaining({ code: "RATE_LIMITED" }),
        );
    } finally {
        store.cancel("alice", first.id);
        release.resolve();
    }

    // keep a result that wins against cancellation
    const states = [];
    for await (const state of store.watch("alice", first.id)) {
        states.push(state);
    }

    // keep the result and the cancellation request
    expect(states.at(-1)).toEqual({
        ...first,
        updatedAt: expect.any(Number),
        completedAt: expect.any(Number),
        cancellationRequested: true,
        state: "succeeded",
        result: "published",
    });

    // require deletion before another operation
    expect(() => store.start("alice", 0, async () => "extra")).toThrow(
        expect.objectContaining({ code: "RATE_LIMITED" }),
    );
    store.delete("alice", first.id);
    const failed = store.start("alice", 0, async () => {
        throw new ServiceError("CONFLICT", { message: "revision changed" });
    });

    // keep a service failure as the outcome
    const failures = [];
    for await (const state of store.watch("alice", failed.id)) {
        failures.push(state);
    }
    expect(failures.at(-1)).toEqual({
        ...failed,
        updatedAt: expect.any(Number),
        completedAt: expect.any(Number),
        state: "failed",
        error: { code: "CONFLICT", message: "revision changed" },
    });
});

test("enforce deadlines, expire completed operations, and cancel work on shutdown", async () => {
    // use short deadlines and retention
    const store = new OperationStore(defineOperation(schema.string(), schema.number()), {
        concurrency: 1,
        capacity: 2,
        retention: 5,
        timeout: 10,
    });

    // run until the deadline
    const operation = store.start("alice", 0, async ({ signal }) => {
        await new Promise<void>((resolve) =>
            signal.addEventListener("abort", () => resolve(), { once: true }),
        );
        signal.throwIfAborted();

        return "unreachable";
    });

    // collect the deadline failure
    const states = [];
    for await (const state of store.watch("alice", operation.id)) {
        states.push(state);
    }

    // report the deadline apart from cancellation
    expect(states.at(-1)).toEqual({
        ...operation,
        updatedAt: expect.any(Number),
        completedAt: expect.any(Number),
        cancellationRequested: true,
        state: "failed",
        error: { code: "DEADLINE_EXCEEDED", message: "operation deadline exceeded" },
    });

    // expire the result
    await new Promise((resolve) => setTimeout(resolve, 6));
    expect(store.list("alice")).toEqual([]);
    expect(() => store.get("alice", operation.id)).toThrow(
        expect.objectContaining({ code: "NOT_FOUND" }),
    );

    // stop an unstarted runner and refuse work after disposal
    let isInvoked = false;
    store.start("alice", 0, async () => {
        isInvoked = true;

        return "unexpected";
    });
    await store.close();
    expect(isInvoked).toBe(false);
    expect(() => store.start("alice", 0, async () => "unexpected")).toThrow(
        expect.objectContaining({ code: "UNAVAILABLE" }),
    );
    await store.close();
});
