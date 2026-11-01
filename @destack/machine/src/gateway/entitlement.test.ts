import * as accountObject from "@destack/account/object";
import type { SpaceResource } from "@destack/account/object";
import { eq } from "@destack/db";
import { expect, refusal, test } from "@destack/test";
import { ids, TestSpace } from "@destack/stack/test";
import { deployNotes, openGateway } from "./test/gateway.ts";
import { Authentication, Caller, principal } from "@destack/access";
import { VERSION_HEADER } from "@destack/object";
import { schema } from "@destack/schema";
import type { Runtime } from "@destack/stack/runtime";
import { Scope } from "@destack/sync";

/** Open the fixture's space, its resources' conditions set as a test says. */
async function open() {
    const fixture = await TestSpace.open();

    // record each resource's condition on the space's row, as its conditions controller does
    const conditions = new Map<SpaceResource, "within" | "blocked">();
    const entitlement = async (resource: SpaceResource, state: "within" | "blocked") => {
        conditions.set(resource, state);
        const now = fixture.machine.clock.now();
        const recorded = Object.fromEntries(
            [...conditions].map(([name, value]) => [
                name,
                {
                    status: value === "within" ? ("true" as const) : ("false" as const),
                    reason: value === "within" ? "Within" : "Blocked",
                    message: `${name} is ${value}`,
                    observedGeneration: 1,
                    lastTransitionAt: now,
                },
            ]),
        );
        await fixture.database.transaction(
            async (transaction) =>
                await transaction
                    .update(accountObject.space.table)
                    .set({ conditions: recorded })
                    .where(eq(accountObject.space.table.id, ids.space)),
        );
    };

    return { fixture, entitlement };
}

/** A runtime answering every call its instances serve. */
const runtime: Runtime = {
    name: "bun",
    start: async () => {},
    stop: async () => {},
    isRunning: () => true,
    identify: () => undefined,
    fetch: async () => new Response(null, { status: 200 }),
    deliver: () => Promise.reject(new Error("the fixture delivers nothing")),
    call: () => Promise.reject(new Error("the fixture calls no export")),
};

/** Call notes through the fixture space's gateway as a person, as its origin hands calls in. */
async function route(fixture: TestSpace) {
    await deployNotes(fixture, [
        {
            instanceId: schema
                .identifier("instance")
                .parse("instance-01996ab0-0000-7000-8000-0000000000c3"),
            release: "2026.10.0",
            status: "active",
        },
    ]);
    const gateway = openGateway(fixture, { runtimes: [runtime] });
    const now = fixture.machine.clock.now();
    const person: Authentication = {
        caller: Caller.of(principal.user.reference(Scope.universe.id, "user-owner")),
        credential: {
            kind: "session",
            id: "fixture",
            audience: ids.package,
            verifiedAt: now,
            expiresAt: now + 60 * 60_000,
        },
    };

    return () =>
        gateway.ingress(
            ids.notes,
            "/notes/list",
            new Request("http://notes.test/", { headers: { [VERSION_HEADER]: "2026.10.0" } }),
            person,
        );
}

/** Pass a burst of 20 requests, then read the refusal of the next. */
async function exhaust(call: () => Promise<unknown>) {
    for (let request = 0; request < 20; request++) {
        await call();
    }

    return refusal(call());
}

test("limit a space's request rate while its compute entitlement is exceeded, and allow it again once within", async () => {
    // block compute and pass a burst of 20 requests
    const { fixture, entitlement } = await open();
    await entitlement("compute", "blocked");
    const call = await route(fixture);
    const refused = await exhaust(call);

    // bring it within the plan and pass more than a burst
    await entitlement("compute", "within");
    for (let request = 0; request < 30; request++) {
        await call();
    }

    expect(refused).toEqual([
        "TOO_MANY_REQUESTS",
        "RATE_LIMITED",
        `space ${ids.space} is rate-limited while its compute entitlement is exceeded`,
    ]);
});

test("limit a space's request rate while its network is exceeded, and leave it alone while its storage alone is", async () => {
    // block storage alone and pass more than a burst
    const { fixture, entitlement } = await open();
    await entitlement("storage", "blocked");
    const call = await route(fixture);
    for (let request = 0; request < 30; request++) {
        await call();
    }

    // block the network and call a burst
    await entitlement("storage", "within");
    await entitlement("network", "blocked");
    const refused = await exhaust(call);

    expect(refused).toEqual([
        "TOO_MANY_REQUESTS",
        "RATE_LIMITED",
        `space ${ids.space} is rate-limited while its network entitlement is exceeded`,
    ]);
});
