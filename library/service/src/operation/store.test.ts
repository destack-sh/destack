import { expect, test } from "@destack/test";
import { schema } from "@destack/schema";
import { Health } from "../health/index.ts";
import { createClient } from "../client/index.ts";
import { defineOperation, defineOperationProcedures } from "../operation/index.ts";
import { ServiceError } from "../error/index.ts";
import { implementOperation, OperationStore, Server, ServiceHandler } from "../server/index.ts";

test("run authorized operations through HTTP, reconnect, cancel, and release results", async () => {
    // configure typed progress and bounded operation retention
    const result = schema.object({ count: schema.number().int() });
    const progress = schema.object({ step: schema.string() });
    const definition = defineOperation(result, progress);
    await using store = new OperationStore(definition, {
        concurrency: 2,
        capacity: 3,
        retention: 60000,
        timeout: 1000,
    });

    // serve operation procedures under the authenticated caller
    const service = defineOperationProcedures(definition);
    const router = implementOperation(store);
    await using server = await Server.start({
        handler: new ServiceHandler(router, {
            health: new Health("operations"),
            authorize: async ({ context }) => {
                if (!context.owner) {
                    throw new ServiceError("UNAUTHORIZED");
                }
            },
        }),
        context: (request) => ({ owner: request.headers.get("authorization")! }),
        drainTimeout: 1000,
    });

    // connect two callers to the same operation store
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

    // observe work independently of a subscriber's connection
    const release = Promise.withResolvers<void>();
    const started = store.start("alice", { step: "compile" }, async ({ report }) => {
        await release.promise;
        report({ step: "complete" });

        return { count: 2 };
    });

    // isolate callers and retain records while work is running
    expect(await client.get({ id: started.id })).toEqual(started);
    expect(await other.list()).toEqual([]);
    await expect(other.get({ id: started.id })).rejects.toMatchObject({ code: "NOT_FOUND" });
    await expect(other.cancel({ id: started.id })).rejects.toMatchObject({ code: "NOT_FOUND" });
    await expect(client.delete({ id: started.id })).rejects.toMatchObject({ code: "CONFLICT" });

    // disconnect a subscriber without cancelling the operation
    const disconnect = new AbortController();
    const stream = await client.watch({ id: started.id }, { signal: disconnect.signal });
    expect(await stream.next()).toEqual({ done: false, value: started });
    disconnect.abort();
    expect(store.get("alice", started.id)).toEqual(started);

    // reconnect after publication and collect the retained outcome
    release.resolve();
    const resumed = await client.watch({ id: started.id });
    const states = [];
    for await (const state of resumed) {
        states.push(state);
    }

    // retain the complete result until the owner deletes its record
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

    // acknowledge cancellation only after the runner observes its signal and cleans up
    const entered = Promise.withResolvers<void>();
    let cleaned = false;
    const cancelled = store.start("alice", { step: "wait" }, async ({ signal }) => {
        const aborted = Promise.withResolvers<void>();
        signal.addEventListener("abort", () => aborted.resolve(), { once: true });
        entered.resolve();
        try {
            await aborted.promise;
            signal.throwIfAborted();

            return { count: 0 };
        } finally {
            cleaned = true;
        }
    });

    // request cancellation after the runner starts listening
    await entered.promise;
    const requested = await client.cancel({ id: cancelled.id });
    expect(requested.cancellationRequested).toBe(true);

    // observe cancellation only after runner cleanup finishes
    const cancellation = await client.watch({ id: cancelled.id });
    const outcomes = [];
    for await (const state of cancellation) {
        outcomes.push(state);
    }

    // retain the cancelled operation with its final progress
    expect(cleaned).toBe(true);
    expect(outcomes.at(-1)).toEqual({
        ...cancelled,
        updatedAt: expect.any(Number),
        completedAt: expect.any(Number),
        cancellationRequested: true,
        state: "cancelled",
    });
});

test("enforce operation limits and retain successful work when cancellation loses the race", async () => {
    // restrict both active runners and retained records to one operation
    await using store = new OperationStore(defineOperation(schema.string(), schema.number()), {
        concurrency: 1,
        capacity: 1,
        retention: 60000,
        timeout: 1000,
    });

    // hold publication until cancellation has been requested
    const entered = Promise.withResolvers<void>();
    const release = Promise.withResolvers<void>();
    const first = store.start("alice", 0, async () => {
        entered.resolve();
        await release.promise;

        return "published";
    });

    // reject concurrent work and release the original runner even on assertion failure
    await entered.promise;
    try {
        expect(() => store.start("alice", 0, async () => "extra")).toThrow(
            expect.objectContaining({ code: "RATE_LIMITED" }),
        );
    } finally {
        store.cancel("alice", first.id);
        release.resolve();
    }

    // preserve a published result when cancellation loses the race
    const states = [];
    for await (const state of store.watch("alice", first.id)) {
        states.push(state);
    }

    // keep successful output and the cancellation request in the terminal record
    expect(states.at(-1)).toEqual({
        ...first,
        updatedAt: expect.any(Number),
        completedAt: expect.any(Number),
        cancellationRequested: true,
        state: "succeeded",
        result: "published",
    });

    // require deletion before admitting another retained operation
    expect(() => store.start("alice", 0, async () => "extra")).toThrow(
        expect.objectContaining({ code: "RATE_LIMITED" }),
    );
    store.delete("alice", first.id);
    const failed = store.start("alice", 0, async () => {
        throw new ServiceError("CONFLICT", { message: "Revision changed." });
    });

    // retain a declared service failure as the operation outcome
    const failures = [];
    for await (const state of store.watch("alice", failed.id)) {
        failures.push(state);
    }
    expect(failures.at(-1)).toEqual({
        ...failed,
        updatedAt: expect.any(Number),
        completedAt: expect.any(Number),
        state: "failed",
        error: { code: "CONFLICT", message: "Revision changed." },
    });
});

test("enforce deadlines, expire completed operations, and cancel work on shutdown", async () => {
    // use real short deadlines and retention without replacing the clock
    const store = new OperationStore(defineOperation(schema.string(), schema.number()), {
        concurrency: 1,
        capacity: 2,
        retention: 5,
        timeout: 10,
    });

    // keep the runner active until its deadline aborts it
    const operation = store.start("alice", 0, async ({ signal }) => {
        await new Promise<void>((resolve) =>
            signal.addEventListener("abort", () => resolve(), { once: true }),
        );
        signal.throwIfAborted();

        return "unreachable";
    });

    // collect the deadline failure through the operation subscription
    const states = [];
    for await (const state of store.watch("alice", operation.id)) {
        states.push(state);
    }

    // distinguish deadline expiry from user cancellation
    expect(states.at(-1)).toEqual({
        ...operation,
        updatedAt: expect.any(Number),
        completedAt: expect.any(Number),
        cancellationRequested: true,
        state: "failed",
        error: { code: "DEADLINE_EXCEEDED", message: "Operation deadline exceeded." },
    });

    // expire the retained result and remove it from both read paths
    await new Promise((resolve) => setTimeout(resolve, 6));
    expect(store.list("alice")).toEqual([]);
    expect(() => store.get("alice", operation.id)).toThrow(
        expect.objectContaining({ code: "NOT_FOUND" }),
    );

    // stop a runner before it starts and refuse further work after disposal
    let invoked = false;
    store.start("alice", 0, async () => {
        invoked = true;

        return "unexpected";
    });
    await store.close();
    expect(invoked).toBe(false);
    expect(() => store.start("alice", 0, async () => "unexpected")).toThrow(
        expect.objectContaining({ code: "UNAVAILABLE" }),
    );
    await store.close();
});
