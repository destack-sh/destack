import { TestObjectStore } from "../src/test/index.ts";
import type { Environment } from "@destack/environment";
import { testSimulation } from "../src/test/index.ts";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { testCallKey } from "@destack/object/test";
import { reconciliation, TestDatabase } from "@destack/db/test";
import { testEvents, serversOf } from "../src/test/index.ts";

import {
    AccessContext,
    accessRelationship,
    Caller,
    principal,
    Relationship,
} from "@destack/access";
import { defineDatabase, eq, Transaction } from "@destack/db";
import { PackageId } from "@destack/package";
import { defineResourceKind, Plan, type Provider } from "@destack/resource";
import { unreachablePlan } from "@destack/resource/error";
import { canonicalize, Digest, present, schema } from "@destack/schema";
import { Scope, RequestId } from "@destack/sync";

import { CONSUMER, defineObject, type Resource } from "../src/index.ts";
import { DRAINING } from "../src/server/index.ts";
import { claim, serveClaims } from "./fixture/claim.ts";
import { openSpace, space, spacePrincipal } from "./fixture/space.ts";
import { tablesOf } from "../src/index.ts";
import objectPackage from "../src/package.ts";

/** The space with the boxes and links. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000051");

/** The host running the boxes its provider provisions. */
const machineId = schema
    .identifier("machine")
    .parse("machine-01996ab0-0000-7000-8000-000000000052");

/** Boxes whose size their consumers require. */
const BoxKind = defineResourceKind("box", {
    spec: schema.object({}),
    state: schema.object({ size: schema.number().int().min(0) }),
});

/** Links nothing provisions. */
const LinkKind = defineResourceKind("link", { spec: schema.object({}) });

/** A space's box: a resource of the box kind. */
const box = defineObject({
    name: "box",
    plural: "boxes",
    scope: space,
    provisioned: { kind: BoxKind },
    fields: {},
});

/** A space's link: a resource of the link kind. */
const link = defineObject({
    name: "link",
    plural: "links",
    scope: space,
    provisioned: { kind: LinkKind },
    fields: {},
});

/** The boxes and links the scenarios use. */
const ids = {
    first: box.identifier("box-01996ab0-0000-7000-8000-000000000053"),
    second: box.identifier("box-01996ab0-0000-7000-8000-000000000054"),
    link: link.identifier("link-01996ab0-0000-7000-8000-000000000055"),
};

/** The database with the space, its boxes and links, their access and the journal. */
const boxDatabase = defineDatabase({
    name: "main",
    tables: [
        ...new Set(
            tablesOf({
                packages: [objectPackage, { objects: { space, box, link }, tables: [] }],
                follows: [],
                tables: [],
            }).tables,
        ),
    ],
});

/** The installation consuming boxes. */
const installationId = schema
    .identifier("installation")
    .parse("installation-01996ab0-0000-7000-8000-000000000057");

/** The fields a box of the fixture package starts with. */
const declared = {
    scope: spaceId,
    definitionPackageId: PackageId.parse("package-01996ab0-0000-7000-8000-000000000056"),
    definitionVersion: "2026.9.0",
    definitionName: "box",
    spec: {},
    approval: "backward-incompatible" as const,
};

/** Boxes a provider stores in memory, each at the size it applied. */
class Boxes {
    /** The applied size of each provisioned box. */
    readonly sizes = new Map<string, number>();
    /** The provisionings left to fail. */
    failures = 0;

    /** Provide the boxes, growing them safely and shrinking them destructively. */
    provider(): Provider<typeof BoxKind, typeof box> {
        return {
            kind: BoxKind,
            code: "memory",
            object: box,
            provision: {
                provision: async (record) => {
                    // fail as many provisionings as the scenario asks
                    if (this.failures > 0) {
                        this.failures -= 1;
                        throw new Error("the disk is full");
                    }
                    this.sizes.set(record.id, this.sizes.get(record.id) ?? 0);

                    return { reference: `memory:${record.id}` };
                },
                destroy: async (record) => {
                    this.sizes.delete(record.id);
                },
            },
            reconcile: {
                plan: async (record, desired) => {
                    // refuse consumers requiring different sizes
                    const sizes = [...new Set(desired.map((state) => state.size))];
                    if (sizes.length > 1) {
                        throw unreachablePlan([
                            {
                                target: "size",
                                detail: `consumers require sizes ${sizes.join(" and ")}`,
                            },
                        ]);
                    }

                    // grow safely and shrink destructively to the size required
                    const size = Math.max(0, ...sizes);
                    const current = this.sizes.get(record.id) ?? 0;
                    const action = size > current ? "update" : "replace";
                    const risk = size > current ? "safe" : "destructive";
                    const detail = `resize from ${current} to ${size}`;

                    return {
                        steps: size === current ? [] : [{ action, target: "size", risk, detail }],
                    };
                },
                apply: async (record, desired) => {
                    this.sizes.set(record.id, Math.max(0, ...desired.map((state) => state.size)));
                },
            },
        };
    }
}

/** Digest each desired state of a box. */
function digests(
    states: readonly { size: number }[],
    crypto: Environment["crypto"],
): Promise<Digest[]> {
    return Promise.all(states.map((state) => Digest.json(state, crypto)));
}

/** Serve the boxes and links of an open space with their providers. */
async function serve(world: Environment, boxes: Boxes) {
    // open the database with the space
    const storage = await TestDatabase.create(boxDatabase, {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const database = storage.database;
    await openSpace(database, world, spaceId);
    await database.transaction(
        async (transaction) =>
            await transaction
                .insert(space.table)
                .values({ id: spaceId, scope: "universe", createdAt: 1, updatedAt: 1 }),
    );

    // store the claims of the resource names in the universe
    const universe = await TestDatabase.create(
        [
            ...new Set(
                tablesOf({
                    packages: [objectPackage, { objects: { claim }, tables: [] }],
                    follows: [],
                    tables: [],
                }).tables,
            ),
        ],
        { environment: world, isMigrated: true },
    );
    onTestFinished(() => universe.close());

    // serve the boxes through their memory provider and links through one provisioning nothing
    const links = { kind: LinkKind, code: "memory", object: link } satisfies Provider<
        typeof LinkKind,
        typeof link
    >;
    const store = await TestObjectStore.receive(space.reference(Scope.universe.id, spaceId), {
        chain: async () => undefined,
        environment: world,
        principal: spacePrincipal(spaceId),
        servers: serversOf([box, link], { providers: [boxes.provider(), links] }),
        machine: machineId,
        database,
        claims: await serveClaims(universe.database, world),
        callKey: testCallKey,
        events: await testEvents(database, world),
        origin: { package: box.package },
    });

    // reconcile one resource as its kind's control loop does
    const reconcile = async (name: "box" | "link", id: string) => {
        const controller = present(
            store.controllers().find((each) => each.name === name),
            `the ${name} controller`,
        );

        return controller.reconcile(
            canonicalize({ id }),
            reconciliation(world.clock.timeout(5000)),
        );
    };
    const read = async (id: typeof ids.first) =>
        (await database.select().from(box.table).where(eq(box.table.id, id)))[0];

    // store a box's desired states and write their digests and its approval threshold as its space does
    const want = async (
        id: typeof ids.first,
        states: { size: number }[],
        drainingStates: { size: number }[] = [],
    ) => {
        const row = present(await read(id), `box ${id}`);
        for (const state of [...states, ...drainingStates]) {
            await store.blobs.writeJson(state);
        }
        await Transaction.run(database, async (transaction) => {
            await store.mutate(
                AccessContext.of(Caller.of(spacePrincipal(spaceId)), world.clock.now()),
                {
                    id: RequestId.create(world),
                    calls: [
                        box.call("specify", {
                            ...box.target(row),
                            states: await digests(states, world.crypto),
                            drainingStates: await digests(drainingStates, world.crypto),
                            approval: declared.approval,
                        }),
                    ],
                },
                { transaction },
            );
        });
    };

    return { store, database, reconcile, read, want };
}

test("provision a box, apply its consumers' growth, and apply a destructive shrink only once its plan is approved", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const boxes = new Boxes();
    const { store, database, reconcile, read, want } = await serve(world, boxes);
    const now = world.clock.now();
    await database.transaction(
        async (transaction) =>
            await transaction.insert(box.table).values({
                ...declared,
                approval: null,
                id: ids.first,
                name: "first",
                createdAt: now,
                updatedAt: now,
            }),
    );

    // leave the box alone until its space sets its approval threshold, and while a transfer fences its space
    await reconcile("box", ids.first);
    const unset = present(await read(ids.first), "the box without a threshold");
    await want(ids.first, [{ size: 2 }]);
    await Transaction.run(database, (transaction) =>
        Scope.fence(transaction, spaceId, "machine-elsewhere", now),
    );
    await reconcile("box", ids.first);
    const fenced = present(await read(ids.first), "the fenced box");

    // provision it and grow it to the size its consumer requires
    await Transaction.run(database, (transaction) => Scope.unfence(transaction, spaceId));
    await reconcile("box", ids.first);
    const grown = present(await read(ids.first), "the grown box");

    // wait for approval of a shrink and leave the waiting box untouched when reconciled again
    await want(ids.first, [{ size: 1 }]);
    await reconcile("box", ids.first);
    const waiting = present(await read(ids.first), "the waiting box");
    await reconcile("box", ids.first);
    const unchanged = present(await read(ids.first), "the unchanged box");

    // apply the approved plan and forget the approval
    const plan = present(waiting.plan, "the waiting plan");
    await database.transaction(
        async (transaction) =>
            await transaction
                .update(box.table)
                .set({ approvedPlan: await Plan.digest(plan, world.crypto) })
                .where(eq(box.table.id, ids.first)),
    );
    await reconcile("box", ids.first);
    const shrunk = present(await read(ids.first), "the shrunk box");

    // store the one state the box needs, and refuse a digest of a state the space lacks
    const stored = await store.blobs.readJson(await Digest.json({ size: 1 }, world.crypto));
    const unknown = await refusal(
        store.mutate(AccessContext.of(Caller.of(spacePrincipal(spaceId)), world.clock.now()), {
            id: RequestId.create(store.environment),
            calls: [
                box.call("specify", {
                    ...box.target(shrunk),
                    states: await digests([{ size: 4 }], world.crypto),
                    drainingStates: [],
                    approval: declared.approval,
                }),
            ],
        }),
    );

    // record the provider, the reference and the host, and apply each size in turn
    expect({
        unset: [unset.reference, unset.conditions],
        fenced: [fenced.reference, fenced.conditions],
        grown: [
            grown.provider,
            grown.reference,
            grown.machineId,
            grown.conditions["Ready"]?.reason,
        ],
        waiting: [waiting.conditions["Ready"]?.reason, plan.steps],
        unchanged,
        shrunk: [shrunk.conditions["Ready"]?.reason, shrunk.approvedPlan, shrunk.plan],
        sizes: [...boxes.sizes],
        stored,
        unknown,
    }).toEqual({
        unset: [null, {}],
        fenced: [null, {}],
        grown: ["memory", `memory:${ids.first}`, machineId, "Applied"],
        waiting: [
            "AwaitingApproval",
            [
                {
                    action: "replace",
                    target: "size",
                    risk: "destructive",
                    detail: "resize from 2 to 1",
                },
            ],
        ],
        unchanged: waiting,
        shrunk: ["Applied", null, null],
        sizes: [[ids.first, 1]],
        stored: { size: 1 },
        unknown: [
            "BAD_REQUEST",
            "DESIRED_STATE_MISSING",
            `${spaceId} has no desired states ${await Digest.json({ size: 4 }, world.crypto)}`,
        ],
    });
});

test("recreate a box without the states only draining consumers need once approved, draining until they stop", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const boxes = new Boxes();
    const { database, reconcile, read, want } = await serve(world, boxes);
    const now = world.clock.now();
    await database.transaction(
        async (transaction) =>
            await transaction.insert(box.table).values({
                ...declared,
                id: ids.first,
                name: "first",
                createdAt: now,
                updatedAt: now,
            }),
    );
    await want(ids.first, [{ size: 2 }]);
    await reconcile("box", ids.first);

    // plan a recreation once the active consumers require a size the draining ones conflict with
    await want(ids.first, [{ size: 3 }], [{ size: 2 }]);
    await reconcile("box", ids.first);
    const waiting = present(await read(ids.first), "the waiting box");
    const plan = present(waiting.plan, "the recreation plan");

    // report draining once approved, leaving the size until the draining consumers stop
    await database.transaction(
        async (transaction) =>
            await transaction
                .update(box.table)
                .set({ approvedPlan: await Plan.digest(plan, world.crypto) })
                .where(eq(box.table.id, ids.first)),
    );
    await reconcile("box", ids.first);
    const draining = present(await read(ids.first), "the draining box");
    const sizeWhileDraining = boxes.sizes.get(ids.first);

    // apply the growth once the draining consumers stopped and end the drain
    await want(ids.first, [{ size: 3 }]);
    await reconcile("box", ids.first);
    const applied = present(await read(ids.first), "the recreated box");
    expect({
        waiting: [conditions(waiting), plan.steps.map((step) => [step.action, step.risk])],
        draining: [conditions(draining), draining.approvedPlan, sizeWhileDraining],
        applied: [conditions(applied), boxes.sizes.get(ids.first)],
    }).toEqual({
        waiting: [
            ["AwaitingApproval", undefined],
            [
                ["delete", "backward-incompatible"],
                ["update", "safe"],
            ],
        ],
        draining: [["Recreating", "true"], null, 2],
        applied: [["Applied", "false"], 3],
    });
});

test("retain a deleted box for its window, retain it while an installation consumes it, then destroy it", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const boxes = new Boxes();
    const { database, reconcile, read } = await serve(world, boxes);
    const now = world.clock.now();
    await database.transaction(
        async (transaction) =>
            await transaction.insert(box.table).values({
                ...declared,
                id: ids.first,
                name: "first",
                retention: { within: { days: 1 } },
                createdAt: now,
                updatedAt: now,
            }),
    );
    await reconcile("box", ids.first);

    // retain the box and its content within its day
    await database.transaction(
        async (transaction) =>
            await transaction
                .update(box.table)
                .set({ deletedAt: now })
                .where(eq(box.table.id, ids.first)),
    );
    const wait = present(await reconcile("box", ids.first), "the wait until the window ends");
    const retained = present(await read(ids.first), "the retained box").conditions["Ready"];

    // retain it past its day while an installation consumes it
    await database.transaction(
        async (transaction) =>
            await transaction
                .update(box.table)
                .set({ deletedAt: now - 2 * 86_400_000 })
                .where(eq(box.table.id, ids.first)),
    );
    const consumer = Relationship.encode({
        id: schema
            .identifier("relationship")
            .parse("relationship-01996ab0-0000-7000-8000-000000000058"),
        object: box.reference(spaceId, ids.first),
        relation: CONSUMER,
        subject: principal.installation.reference(spaceId, installationId),
        createdAt: now,
        expiresAt: null,
    });
    await database.transaction(
        async (transaction) => await transaction.insert(accessRelationship).values(consumer),
    );
    await reconcile("box", ids.first);
    const used = present(await read(ids.first), "the used box").conditions["Ready"];

    // destroy its content and finish its deletion once unused
    await database.transaction(
        async (transaction) =>
            await transaction
                .delete(accessRelationship)
                .where(eq(accessRelationship.id, consumer.id)),
    );
    await reconcile("box", ids.first);
    expect({
        retained: [retained?.reason, wait > 86_000_000 && wait <= 86_400_000],
        used: [used?.reason, used?.message],
        removed: [await read(ids.first), [...boxes.sizes]],
    }).toEqual({
        retained: ["Retained", true],
        used: ["InUse", `consumed by ${installationId}`],
        removed: [undefined, []],
    });
});

test("record a failed provisioning, and provision on the next attempt", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const boxes = new Boxes();
    const { database, reconcile, read } = await serve(world, boxes);
    const now = world.clock.now();
    await database.transaction(
        async (transaction) =>
            await transaction.insert(box.table).values({
                ...declared,
                id: ids.first,
                name: "first",
                createdAt: now,
                updatedAt: now,
            }),
    );

    // record and throw the failure and provision on the next attempt
    boxes.failures = 1;
    const failed = await reconcile("box", ids.first).then(
        () => "reconciled",
        (error: unknown) => (error instanceof Error ? error.message : error),
    );
    const failure = present(await read(ids.first), "the failed box").conditions["Ready"];
    await reconcile("box", ids.first);
    const provisioned = present(await read(ids.first), "the provisioned box").conditions["Ready"];
    expect({
        failed: [failed, failure?.reason, failure?.message],
        provisioned: provisioned?.reason,
        sizes: [...boxes.sizes.keys()],
    }).toEqual({
        failed: ["the disk is full", "ProvisioningFailed", "the disk is full"],
        provisioned: "Applied",
        sizes: [ids.first],
    });
});

test("refuse provisioning through a provider that provisions nothing, and finish a deletion without destroying", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const { database, reconcile } = await serve(world, new Boxes());
    const now = world.clock.now();
    await database.transaction(
        async (transaction) =>
            await transaction.insert(link.table).values({
                ...declared,
                id: ids.link,
                name: "link",
                definitionName: "link",
                retention: "delete",
                createdAt: now,
                updatedAt: now,
            }),
    );
    const readLink = async () =>
        (await database.select().from(link.table).where(eq(link.table.id, ids.link)))[0];

    // observe that the provider provisions nothing
    await reconcile("link", ids.link);
    const refused = present(await readLink(), "the link").conditions["Ready"];

    // finish its deletion without destroying anything
    await database.transaction(
        async (transaction) =>
            await transaction
                .update(link.table)
                .set({ deletedAt: now })
                .where(eq(link.table.id, ids.link)),
    );
    await reconcile("link", ids.link);
    expect([refused?.reason, refused?.message, await readLink()]).toEqual([
        "NoProvisioning",
        "provider memory of link provisions nothing",
        undefined,
    ]);
});

test("set a resource's name after another kind's resource in its space, refusing a second of its kind", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const { store } = await serve(world, new Boxes());
    const { scope, approval: _approval, ...written } = declared;
    const create = (kind: typeof box | typeof link) =>
        store.mutate(AccessContext.of(Caller.of(spacePrincipal(spaceId)), world.clock.now()), {
            id: RequestId.create(store.environment),
            calls: [kind.call("create", { ...kind.in(scope), ...written, name: "main" })],
        });

    // create a box and a link named main, and refuse another box of the name
    const [created] = await create(box);
    const [linked] = await create(link);
    const taken = await refusal(create(box));

    expect({ names: [created?.name, linked?.name], taken }).toEqual({
        names: ["main", "main"],
        taken: ["CONFLICT", "DUPLICATE", "another box has the same scope and name"],
    });
});

/** Read a box's ready reason and draining status. */
function conditions(row: { readonly conditions: Resource["conditions"] }) {
    return [row.conditions["Ready"]?.reason, row.conditions[DRAINING]?.status] as const;
}
