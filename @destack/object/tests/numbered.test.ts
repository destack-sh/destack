import { principal } from "@destack/access";
import { RequestId, Scope } from "@destack/sync";
import { expect, refusal, test } from "@destack/test";
import { Snapshot } from "@destack/db";
import { present, schema } from "@destack/schema";
import {
    defineObject,
    definePackage,
    field,
    numbered,
    Numbered,
    recoverable,
    shareable,
} from "../src/index.ts";
import { TestObjectStore, testIdentity } from "../src/test/index.ts";
import { space } from "../src/test/space.ts";
import { users } from "./fixture/user.ts";
import { argument, plural, t } from "@destack/locale";
import objectPackage from "../src/package.ts";

/** A team addressed by its key, such as TRK, whose issues count from 1. */
const team = defineObject(
    {
        name: "team",
        plural: "teams",
        title: t`${plural(argument("count"), { one: "Team", other: "Teams" })}`,
        scope: space,
        key: "key",
        fields: { key: field.string(schema.string().min(1)) },
        traits: [shareable({})],
    },
    (method) => ({
        create: method.create("manage"),
        update: method.update("manage", { fields: ["key"] }),
    }),
);

/** An issue of a team, numbered within it as TRK-214 and moved between teams. */
const issue = defineObject(
    {
        name: "issue",
        plural: "issues",
        title: t`${plural(argument("count"), { one: "Issue", other: "Issues" })}`,
        scope: space,
        fields: {
            team: field.reference(team, { delete: "restrict" }),
            title: field.string(schema.string().min(1)),
        },
        traits: [
            numbered({ within: "teamId", identifier: "live" }),
            recoverable({ within: { days: 30 }, by: "manage", purge: "manage" }),
            shareable({}),
        ],
    },
    (method) => ({
        create: method.create("manage"),
        createMany: method.createMany("manage", { fields: ["teamId", "title"] }),
        update: method.update("manage", { fields: ["teamId", "title"] }),
    }),
);

/** An order of a space, numbered from #1001. */
const order = defineObject(
    {
        name: "order",
        plural: "orders",
        title: t`${plural(argument("count"), { one: "Order", other: "Orders" })}`,
        scope: space,
        fields: { total: field.integer() },
        traits: [numbered({ start: 1001, identifier: "live", prefix: "#" }), shareable({})],
    },
    (method) => ({ create: method.create("manage") }),
);

/** A team's invoice, numbered once finalized, its identifier printed as issued. */
const invoice = defineObject(
    {
        name: "invoice",
        plural: "invoices",
        title: t`${plural(argument("count"), { one: "Invoice", other: "Invoices" })}`,
        scope: space,
        fields: {
            team: field.reference(team, { delete: "restrict" }),
            status: field.state({
                initial: "draft",
                transitions: {
                    finalize: { from: ["draft"], to: "open", permission: "manage" },
                },
            }),
        },
        traits: [
            numbered({ within: "teamId", at: "finalize", identifier: "frozen", prefix: "INV-" }),
            shareable({}),
        ],
    },
    (method) => ({
        create: method.create("manage"),
        update: method.update("manage", { fields: ["teamId"] }),
        delete: method.delete("manage"),
    }),
);

/** Teams with their issues and invoices, orders, and object's roles granting their permissions. */
const issuesPackage = definePackage({
    ...testIdentity,
    objects: [...Object.values(objectPackage.objects), team, issue, order, invoice],
});

test("number issues within their team, never reusing a purged one's, and number a moved one anew in its new team", async () => {
    // serve alice's two teams
    await using objects = await TestObjectStore.open({
        packages: [issuesPackage],
        scope: space.reference(Scope.universe.id, "space-01996ab0-0000-7000-8000-000000000320"),
        owners: [principal.user.reference(Scope.universe.id, users.alice)],
    });
    const client = objects.as(users.alice).client(issuesPackage);
    const spaceId = objects.scope;
    const request = () => ({ spaceId, requestId: RequestId.create(objects.environment) });
    const tracker = await client.team.create({ ...request(), key: "TRK" });
    const engine = await client.team.create({ ...request(), key: "ENG" });
    const create = (teamId: string, title: string) =>
        client.issue.create({ ...request(), teamId: team.identifier(teamId), title });

    // number two issues in the tracker and one in the engine, then purge the tracker's last
    await create(tracker.id, "Sign-up");
    const usage = await create(tracker.id, "Usage page");
    await create(engine.id, "Sync");
    await client.issue.delete({ ...request(), id: usage.id });
    await client.issue.purge({ ...request(), id: usage.id });

    // number the tracker's next issue past the purged one, and two more at once
    const billing = await create(tracker.id, "Billing");
    await client.issue.createMany({
        ...request(),
        items: [
            { teamId: tracker.id, title: "Export" },
            { teamId: tracker.id, title: "Import" },
        ],
    });

    // move billing to the engine, which numbers it after the engine's last
    const moved = await client.issue.update({ ...request(), id: billing.id, teamId: engine.id });
    const after = await create(tracker.id, "Search");

    // read each issue's identifier, and find issues by identifier, billing also by its old one
    const reader = Snapshot.live(objects.database);
    const identifiers = await Promise.all(
        (await client.issue.list({ spaceId })).items.map((item) =>
            Numbered.identifier(issue, item, reader),
        ),
    );
    const titleOf = async (named: string) =>
        (await Numbered.find(issue, spaceId, named, reader))?.["title"];
    const found = await Promise.all(["TRK-4", "ENG-2", "TRK-3", "TRK-2", "TRK"].map(titleOf));

    // rename the engine's key, which renames its issues' identifiers
    await client.team.update({ ...request(), id: engine.id, key: "EN" });
    const renamed = [await Numbered.identifier(issue, moved, reader), await titleOf("EN-2")];

    expect({
        billing: billing.number,
        moved: moved.number,
        after: after.number,
        found,
        identifiers: identifiers
            .map((each) => present(each, "an issued identifier"))
            .toSorted((left, right) => left.localeCompare(right)),
        renamed,
    }).toEqual({
        billing: 3,
        moved: 2,
        after: 6,
        found: ["Export", "Billing", "Billing", undefined, undefined],
        renamed: ["EN-2", "Billing"],
        identifiers: ["ENG-1", "ENG-2", "TRK-1", "TRK-4", "TRK-5", "TRK-6"],
    });
});

test("let an import keep its numbers, counting on past the highest, and refuse a number taken", async () => {
    // serve alice's tracker team
    await using objects = await TestObjectStore.open({
        packages: [issuesPackage],
        scope: space.reference(Scope.universe.id, "space-01996ab0-0000-7000-8000-000000000321"),
        owners: [principal.user.reference(Scope.universe.id, users.alice)],
    });
    const client = objects.as(users.alice).client(issuesPackage);
    const spaceId = objects.scope;
    const request = () => ({ spaceId, requestId: RequestId.create(objects.environment) });
    const tracker = await client.team.create({ ...request(), key: "TRK" });
    const create = (title: string, number?: number) =>
        client.issue.create({
            ...request(),
            teamId: tracker.id,
            title,
            ...(number === undefined ? {} : { number }),
        });

    // import issue 214, then one at 120 below it, and create two after them
    const imported = [await create("Usage page", 214), await create("Billing", 120)];
    const created = [await create("Export"), await create("Import")];
    const taken = await refusal(create("Search", 120));

    // let bob plan issues in the space without assigning numbers, and refuse his number
    const scoped = () => ({ scope: spaceId, requestId: RequestId.create(objects.environment) });
    const planner = await client.role.create({
        ...scoped(),
        name: "planner",
        description: "Plans issues",
        permissions: [issue.permission("manage")],
    });
    await client.relationship.create({
        ...scoped(),
        object: space.reference(Scope.universe.id, spaceId),
        role: planner.id,
        subject: principal.user.reference(Scope.universe.id, users.bob),
    });
    const bob = objects.as(users.bob).client(issuesPackage);
    const planned = (number?: number) =>
        bob.issue.create({
            ...request(),
            teamId: tracker.id,
            title: "Plan",
            ...(number === undefined ? {} : { number }),
        });
    const assigned = await refusal(planned(300));
    const unassigned = await planned();

    expect({
        numbers: [...imported, ...created].map((item) => [item.title, item.number]),
        taken,
        assigned,
        bob: unassigned.number,
    }).toEqual({
        numbers: [
            ["Usage page", 214],
            ["Billing", 120],
            ["Export", 215],
            ["Import", 216],
        ],
        taken: [
            "CONFLICT",
            "OBJECT_EXISTS",
            "'issue' with the same 'team and number' already exists",
        ],
        assigned: ["FORBIDDEN", "FIELD_DENIED", "field 'number' is forbidden: it is not writable"],
        bob: 217,
    });
});

test("number orders across the space from #1001, and find one by its identifier", async () => {
    // serve alice's space, and take three orders
    await using objects = await TestObjectStore.open({
        packages: [issuesPackage],
        scope: space.reference(Scope.universe.id, "space-01996ab0-0000-7000-8000-000000000322"),
        owners: [principal.user.reference(Scope.universe.id, users.alice)],
    });
    const client = objects.as(users.alice).client(issuesPackage);
    const spaceId = objects.scope;
    const request = () => ({ spaceId, requestId: RequestId.create(objects.environment) });
    const orders = [
        await client.order.create({ ...request(), total: 40 }),
        await client.order.create({ ...request(), total: 75 }),
    ];

    // read their identifiers, and find the second by its own
    const reader = Snapshot.live(objects.database);
    const identifiers = await Promise.all(
        orders.map((each) => Numbered.identifier(order, each, reader)),
    );
    const found = await Numbered.find(order, spaceId, "#1002", reader);

    expect({ identifiers, found: found?.["total"] }).toEqual({
        identifiers: ["#1001", "#1002"],
        found: 75,
    });
});

test("number an invoice once finalized, printing its identifier as issued and refusing to move it", async () => {
    // serve alice's ACME team with two draft invoices, and delete one
    await using objects = await TestObjectStore.open({
        packages: [issuesPackage],
        scope: space.reference(Scope.universe.id, "space-01996ab0-0000-7000-8000-000000000323"),
        owners: [principal.user.reference(Scope.universe.id, users.alice)],
    });
    const client = objects.as(users.alice).client(issuesPackage);
    const spaceId = objects.scope;
    const request = () => ({ spaceId, requestId: RequestId.create(objects.environment) });
    const acme = await client.team.create({ ...request(), key: "ACME" });
    const other = await client.team.create({ ...request(), key: "OTHER" });
    const kept = await client.invoice.create({ ...request(), teamId: acme.id });
    const dropped = await client.invoice.create({ ...request(), teamId: acme.id });
    await client.invoice.delete({ ...request(), id: dropped.id });

    // finalize the kept one, then rename the team and try to move the invoice
    const finalized = await client.invoice.finalize({ ...request(), id: kept.id });
    await client.team.update({ ...request(), id: acme.id, key: "ACMECO" });
    const moved = await refusal(
        client.invoice.update({ ...request(), id: kept.id, teamId: other.id }),
    );
    const reader = Snapshot.live(objects.database);

    expect({
        draft: [kept.number, kept.identifier],
        finalized: [finalized.number, finalized.identifier],
        printed: await Numbered.identifier(invoice, finalized, reader),
        found: (await Numbered.find(invoice, spaceId, "INV-ACME-1", reader))?.["id"],
        moved,
    }).toEqual({
        draft: [null, null],
        finalized: [1, "INV-ACME-1"],
        printed: "INV-ACME-1",
        found: kept.id,
        moved: [
            "PRECONDITION_FAILED",
            "OBJECT_BLOCKED",
            "cannot move invoice: its identifier is frozen",
        ],
    });
});

test("refuse numbering within a column some objects lack, and moving many numbered objects at once", () => {
    // declare issues numbered within an optional team, and issues moved between teams in bulk
    const declare = (within: { optional: boolean }, methods: "update" | "updateMany") => () =>
        defineObject(
            {
                name: "ticket",
                plural: "tickets",
                title: t`${plural(argument("count"), { one: "Ticket", other: "Tickets" })}`,
                scope: space,
                fields: {
                    team: within.optional
                        ? field.reference(team, { delete: "restrict" }).optional()
                        : field.reference(team, { delete: "restrict" }),
                    title: field.string(),
                },
                traits: [numbered({ within: "teamId" }), shareable({})],
            },
            (method) =>
                methods === "update"
                    ? { update: method.update("manage", { fields: ["teamId"] }) }
                    : {
                          move: method.updateMany("manage", {
                              fields: ["teamId"],
                              match: ["title"],
                          }),
                      },
        ).requireValid();

    expect([
        refusalOf(declare({ optional: true }, "update")),
        refusalOf(declare({ optional: false }, "updateMany")),
        refusalOf(declare({ optional: false }, "update")),
    ]).toEqual([
        "object 'ticket' numbers within 'teamId', which every object must have",
        "object ticket numbers within 'teamId', so it cannot apply many updates to it at once",
        "declared",
    ]);
});

/** Read the message a declaration throws. */
function refusalOf(declare: () => unknown): string {
    try {
        declare();

        return "declared";
    } catch (error) {
        return error instanceof TypeError ? error.message : "not a type error";
    }
}
