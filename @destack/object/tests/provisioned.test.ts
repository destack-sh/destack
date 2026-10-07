import { expect, onTestFinished, refusal, test } from "@destack/test";
import { reconciliation, testCallKey } from "@destack/service/test";
import { journal } from "@destack/audit/stack";
import { accessRelationship, principal, Relationship } from "@destack/access";
import { defineDatabase, eq } from "@destack/db";
import { DirectoryStore, directoryTables } from "@destack/directory";
import { TestDatabase } from "@destack/db/test";
import { PackageId } from "@destack/package";
import { defineResourceKind, Plan, type Provider } from "@destack/resource";
import { PlanError } from "@destack/resource/error";
import { canonicalize, present, schema } from "@destack/schema";
import { Scope } from "@destack/sync";

import { CONSUMER, defineObject, type ProvisionedRecord, SystemCall } from "../src/index.ts";
import { DRAINING, ObjectServer } from "../src/server/index.ts";
import { openSpace, space } from "./fixture/space.ts";

/** The space with the boxes and links. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000051");

/** The host keeping the boxes its provider provisions. */
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

/** The boxes and links the scenarios name. */
const ids = {
    first: box.identifier("box-01996ab0-0000-7000-8000-000000000053"),
    second: box.identifier("box-01996ab0-0000-7000-8000-000000000054"),
    link: link.identifier("link-01996ab0-0000-7000-8000-000000000055"),
};

/** The database with the space, its boxes and links, their access and the journal. */
const boxDatabase = defineDatabase({
    name: "main",
    tables: [...new Set([journal, ...space.tables, ...box.tables, ...link.tables])],
});

/** The installation consuming boxes. */
const installationId = schema
    .identifier("installation")
    .parse("installation-01996ab0-0000-7000-8000-000000000057");

/** The fields a box of the fixture package starts with, its approval threshold set as its space sets it. */
const declared = {
    scope: spaceId,
    definitionPackageId: PackageId.parse("package-01996ab0-0000-7000-8000-000000000056"),
    definitionVersion: "2026.9.0",
    definitionName: "box",
    spec: {},
    approval: "backward-incompatible" as const,
};

/** Boxes a provider keeps in memory, each at the size it applied. */
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
                        throw new PlanError([
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

/** Serve the boxes and links of an open space with their providers. */
async function serve(boxes: Boxes) {
    // open the database with the space
    const storage = await TestDatabase.create("sqlite", boxDatabase, { isMigrated: true });
    onTestFinished(() => storage.close());
    const database = storage.database;
    await openSpace(database, spaceId);
    await database
        .insert(space.table)
        .values({ id: spaceId, scope: "universe", createdAt: 1, updatedAt: 1 });

    // keep the claims of the resource names in a directory
    const universe = await TestDatabase.create("sqlite", directoryTables, { isMigrated: true });
    onTestFinished(() => universe.close());

    // serve the boxes through their memory provider and links through one provisioning nothing
    const links = { kind: LinkKind, code: "memory", object: link } satisfies Provider<
        typeof LinkKind,
        typeof link
    >;
    const server = new ObjectServer({
        objects: { box, link },
        database,
        directory: new DirectoryStore(universe.database),
        callKey: testCallKey,
        origin: { package: box.package, service: "test" },
        provisioned: { providers: [boxes.provider(), links], machine: machineId },
    });

    // reconcile one resource as its kind's control loop does
    const reconcile = async (name: "box" | "link", id: string) => {
        const controller = present(
            server.controllers().find((each) => each.name === name),
            `the ${name} controller`,
        );

        return controller.reconcile(
            canonicalize({ id }),
            reconciliation(AbortSignal.timeout(5000)),
        );
    };
    const read = async (id: typeof ids.first) =>
        (await database.select().from(box.table).where(eq(box.table.id, id)))[0];

    // write a box's desired states and approval threshold as its space does
    const want = async (
        id: typeof ids.first,
        states: { size: number }[],
        drainingStates: { size: number }[] = [],
    ) => {
        const row = present(await read(id), `box ${id}`);
        await server.executeAsSystem(
            box,
            "specify",
            [SystemCall.of(row, { states, drainingStates, approval: declared.approval })],
            Date.now(),
        );
    };

    return { server, database, reconcile, read, want };
}

test("provision a box, apply its consumers' growth, and apply a destructive shrink only once its plan is approved", async () => {
    const boxes = new Boxes();
    const { database, reconcile, read, want } = await serve(boxes);
    const now = Date.now();
    await database.insert(box.table).values({
        ...declared,
        approval: null,
        id: ids.first,
        name: "first",
        createdAt: now,
        updatedAt: now,
    });

    // leave the box alone until its space sets its approval threshold, and while a transfer fences its space
    await reconcile("box", ids.first);
    const unset = present(await read(ids.first), "the box without a threshold");
    await want(ids.first, [{ size: 2 }]);
    await Scope.fence(database, spaceId, "region-elsewhere", now);
    await reconcile("box", ids.first);
    const fenced = present(await read(ids.first), "the fenced box");

    // provision it and grow it to the size its consumer requires
    await Scope.unfence(database, spaceId);
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
    await database
        .update(box.table)
        .set({ approvedPlan: await Plan.digest(plan) })
        .where(eq(box.table.id, ids.first));
    await reconcile("box", ids.first);
    const shrunk = present(await read(ids.first), "the shrunk box");

    // record the provider, the reference and the host, and apply each size in turn
    expect({
        unset: [unset.reference, unset.observedGeneration, unset.conditions],
        fenced: [fenced.reference, fenced.observedGeneration, fenced.conditions],
        grown: [
            grown.provider,
            grown.reference,
            grown.machineId,
            grown.conditions["ready"]?.reason,
        ],
        waiting: [waiting.conditions["ready"]?.reason, plan.steps],
        unchanged,
        shrunk: [shrunk.conditions["ready"]?.reason, shrunk.approvedPlan, shrunk.plan],
        sizes: [...boxes.sizes],
    }).toEqual({
        unset: [null, 0, {}],
        fenced: [null, 0, {}],
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
    });
});

test("recreate a box without the states only draining consumers need once approved, draining until they stop", async () => {
    const boxes = new Boxes();
    const { database, reconcile, read, want } = await serve(boxes);
    const now = Date.now();
    await database.insert(box.table).values({
        ...declared,
        id: ids.first,
        name: "first",
        createdAt: now,
        updatedAt: now,
    });
    await want(ids.first, [{ size: 2 }]);
    await reconcile("box", ids.first);

    // plan a recreation once the active consumers require a size the draining ones conflict with
    await want(ids.first, [{ size: 3 }], [{ size: 2 }]);
    await reconcile("box", ids.first);
    const waiting = present(await read(ids.first), "the waiting box");
    const plan = present(waiting.plan, "the recreation plan");

    // report draining once approved, leaving the size until the draining consumers stop
    await database
        .update(box.table)
        .set({ approvedPlan: await Plan.digest(plan) })
        .where(eq(box.table.id, ids.first));
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

test("retain a deleted box for its window, keep it while an installation consumes it, then destroy it", async () => {
    const boxes = new Boxes();
    const { database, reconcile, read } = await serve(boxes);
    const now = Date.now();
    await database.insert(box.table).values({
        ...declared,
        id: ids.first,
        name: "first",
        retention: { within: { days: 1 } },
        createdAt: now,
        updatedAt: now,
    });
    await reconcile("box", ids.first);

    // keep the box and its content within its day
    await database
        .update(box.table)
        .set({ deletionRequestedAt: now })
        .where(eq(box.table.id, ids.first));
    const wait = present(await reconcile("box", ids.first), "the wait until the window ends");
    const retained = present(await read(ids.first), "the retained box").conditions["ready"];

    // keep it past its day while an installation consumes it
    await database
        .update(box.table)
        .set({ deletionRequestedAt: now - 2 * 86_400_000 })
        .where(eq(box.table.id, ids.first));
    const consumer = Relationship.encode(
        {
            id: schema
                .identifier("relationship")
                .parse("relationship-01996ab0-0000-7000-8000-000000000058"),
            object: box.reference(spaceId, ids.first),
            relation: CONSUMER,
            subject: principal.installation.reference(spaceId, installationId),
            createdAt: now,
            expiresAt: null,
        },
        spaceId,
    );
    await database.insert(accessRelationship).values(consumer);
    await reconcile("box", ids.first);
    const used = present(await read(ids.first), "the used box").conditions["ready"];

    // destroy its content and finish its deletion once unused
    await database.delete(accessRelationship).where(eq(accessRelationship.id, consumer.id));
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

test("record a failed provisioning, provision on the next attempt, and observe a box declared at a reference without provisioning it", async () => {
    const boxes = new Boxes();
    const { database, reconcile, read } = await serve(boxes);
    const now = Date.now();
    await database.insert(box.table).values([
        {
            ...declared,
            id: ids.first,
            name: "first",
            createdAt: now,
            updatedAt: now,
        },
        {
            ...declared,
            id: ids.second,
            name: "second",
            origin: "declared",
            provider: "memory",
            reference: "memory:elsewhere",
            createdAt: now,
            updatedAt: now,
        },
    ]);

    // record and throw the failure and provision on the next attempt
    boxes.failures = 1;
    const failed = await reconcile("box", ids.first).then(
        () => "reconciled",
        (error: unknown) => (error instanceof Error ? error.message : error),
    );
    const failure = present(await read(ids.first), "the failed box").conditions["ready"];
    await reconcile("box", ids.first);
    const provisioned = present(await read(ids.first), "the provisioned box").conditions["ready"];

    // observe the declared box ready at its reference
    await reconcile("box", ids.second);
    const ready = present(await read(ids.second), "the declared box").conditions["ready"];
    expect({
        failed: [failed, failure?.reason, failure?.message],
        provisioned: provisioned?.reason,
        declared: [ready?.status, ready?.reason, ready?.message],
        sizes: [...boxes.sizes.keys()],
    }).toEqual({
        failed: ["the disk is full", "ProvisioningFailed", "the disk is full"],
        provisioned: "Applied",
        declared: ["true", "Declared", "declared at memory:elsewhere"],
        sizes: [ids.first],
    });
});

test("refuse provisioning through a provider that provisions nothing, and finish a deletion without destroying", async () => {
    const { database, reconcile } = await serve(new Boxes());
    const now = Date.now();
    await database.insert(link.table).values({
        ...declared,
        id: ids.link,
        name: "link",
        definitionName: "link",
        retention: "delete",
        createdAt: now,
        updatedAt: now,
    });
    const readLink = async () =>
        (await database.select().from(link.table).where(eq(link.table.id, ids.link)))[0];

    // observe that the provider provisions nothing
    await reconcile("link", ids.link);
    const refused = present(await readLink(), "the link").conditions["ready"];

    // finish its deletion without destroying anything
    await database
        .update(link.table)
        .set({ deletionRequestedAt: now })
        .where(eq(link.table.id, ids.link));
    await reconcile("link", ids.link);
    expect([refused?.reason, refused?.message, await readLink()]).toEqual([
        "NoProvisioning",
        "provider memory of link provisions nothing",
        undefined,
    ]);
});

test("name a resource after another kind's resource in its space, refusing a second of its own kind", async () => {
    const { server } = await serve(new Boxes());
    const { scope, approval: _approval, ...written } = declared;
    const create = (kind: typeof box | typeof link) =>
        server.executeAsSystem(
            kind,
            "create",
            [{ scope, input: { ...written, name: "main" } }],
            Date.now(),
        );

    // create a box and a link named main, and refuse another box of the name
    const [created] = await create(box);
    const [linked] = await create(link);
    const taken = await refusal(create(box));

    expect({ names: [created?.name, linked?.name], taken }).toEqual({
        names: ["main", "main"],
        taken: ["DUPLICATE", "a record with the same unique key exists"],
    });
});

/** Read a box's ready reason and draining status. */
function conditions(row: { readonly conditions: ProvisionedRecord["conditions"] }) {
    return [row.conditions["ready"]?.reason, row.conditions[DRAINING]?.status] as const;
}
