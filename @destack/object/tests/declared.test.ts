import { testSimulation } from "../src/test/index.ts";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { TestDatabase } from "@destack/db/test";
import { defineDatabase, Snapshot } from "@destack/db";
import { testCallKey } from "@destack/object/test";
import { anyone, Manager, principal, Caller } from "@destack/access";
import { Identifier, schema } from "@destack/schema";
import { ObjectReference, Scope, Subject } from "@destack/sync";
import { StackTarget, defineObject, field } from "../src/index.ts";
import { type StackOptions, Stack } from "../src/server/index.ts";
import { serversOf, testEvents } from "../src/test/index.ts";
import { objectDatabase } from "./schema.ts";
import { openSpace, space, spacePrincipal } from "../src/test/space.ts";
import { TestObjectStore } from "../src/test/index.ts";
import { nested } from "../src/index.ts";
import { argument, plural, t } from "@destack/locale";

/** The space the stack declares in. */
const spaceId = schema.identifier("space").parse(`space-01996ab0-0000-7000-8000-00000000a00c`);

/** Tags a stack declares by name. */
const tag = defineObject(
    {
        name: "tag",
        plural: "tags",
        title: t`${plural(argument("count"), { one: "Tag", other: "Tags" })}`,
        scope: space,
        fields: { name: field.string(), color: field.string() },
        access: {
            permissions: ["write"],
        },
    },
    (method) => ({
        create: method.create("write", { fields: ["name", "color"] }),
        get: method.get("write"),
    }),
);

/** Stamps a stack declares, each on the tag its parent reference refers to by name. */
const stamp = defineObject(
    {
        name: "stamp",
        plural: "stamps",
        title: t`${plural(argument("count"), { one: "Stamp", other: "Stamps" })}`,
        scope: space,
        fields: { mark: field.string() },
        access: {
            permissions: ["write"],
        },
        traits: [nested({ in: tag, delete: "restrict", add: "write" })],
    },
    (method) => ({
        create: method.create("write", { fields: ["mark"] }),
        get: method.get("write"),
    }),
);

/** Markers a stack declares on a tag it declares by name or selects by identifier. */
const marker = defineObject(
    {
        name: "marker",
        plural: "markers",
        title: t`${plural(argument("count"), { one: "Marker", other: "Markers" })}`,
        scope: space,
        fields: { tag: field.reference(() => tag), note: field.string() },
        access: {
            permissions: ["write"],
        },
    },
    (method) => ({
        create: method.create("write", { fields: ["tagId", "note"] }),
        get: method.get("write"),
    }),
);

/** Badges a stack declares, each of a subject and placed on an object. */
const badge = defineObject(
    {
        name: "badge",
        plural: "badges",
        title: t`${plural(argument("count"), { one: "Badge", other: "Badges" })}`,
        scope: space,
        fields: {
            holder: field.json(StackTarget.on(Subject, () => "subject")),
            place: field.json(StackTarget.on(ObjectReference, () => "object")),
        },
        access: {
            permissions: ["write"],
        },
    },
    (method) => ({
        create: method.create("write", { fields: ["holder", "place"] }),
        get: method.get("write"),
    }),
);

/** Seals a stack declares after every other type, as bindings are. */
const seal = defineObject(
    {
        name: "seal",
        plural: "seals",
        title: t`${plural(argument("count"), { one: "Seal", other: "Seals" })}`,
        scope: space,
        fields: { name: field.string() },
        access: {
            permissions: ["write"],
        },
    },
    (method) => ({
        create: method.create("write", { fields: ["name"] }),
        get: method.get("write"),
    }),
).implement({ declaration: { after: "every" } });

/** Crates whose policy the stack knows but whose records it neither applies nor reads. */
const crate = defineObject({
    name: "crate",
    plural: "crates",
    title: t`${plural(argument("count"), { one: "Crate", other: "Crates" })}`,
    scope: space,
    fields: { size: field.integer() },
    access: {
        permissions: ["write"],
    },
});

/** The installation and package declaring the records. */
const manager = Manager.schema.omit({ name: true }).parse({
    installationId: "installation-01996ab0-0000-7000-8000-000000000001",
    packageId: "package-01996ab0-0000-7000-8000-000000000002",
});

test("declare records as their create inputs in reference order, the name filling their name and references resolved by name, identifier, scope or wildcard", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // open the space over a database with the declared types' tables
    const types = [badge, seal, stamp, marker, tag];
    const tables = [...objectDatabase.tables, ...[...types, crate].flatMap((type) => type.tables)];
    const storage = await TestDatabase.create(defineDatabase({ name: "main", tables }), {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const { database } = storage;
    await openSpace(database, world, spaceId);
    await database.transaction(
        async (transaction) =>
            await transaction
                .insert(space.table)
                .values({ id: spaceId, scope: Scope.universe.id, createdAt: 0, updatedAt: 0 }),
    );
    const store = await TestObjectStore.receive(space.reference(Scope.universe.id, spaceId), {
        chain: async () => undefined,
        environment: world,
        principal: spacePrincipal(spaceId),
        servers: serversOf([...types, crate]),
        policies: [space],
        database,
        callKey: testCallKey,
        events: await testEvents(database, world),
        origin: { package: tag.package },
    });
    let applied: StackOptions["applied"] = {};
    const apply = async (document: StackOptions["document"]) => {
        // apply the document against the one applied last
        const plan = await new Stack({
            database,
            isHome: false,
            objects: types,
            policies: [crate.policy],
            manager,
            as: spacePrincipal(spaceId),
            declarer: () => Caller.of(spacePrincipal(spaceId)),
            scope: spaceId,
            document,
            applied,
            store,
        }).apply(world.clock.now());
        applied = document;

        return plan;
    };

    // declare a tag, then a stamp, markers and badges of each subject form on it
    const red = { color: "#f00" };
    await apply({ tags: { red } });
    const [redTag] = await Snapshot.live(database).read(tag.table).findMany();
    const user = Identifier.create("user", world);
    const crateId = Identifier.create("crate", world);
    await apply({
        tags: { red },
        seals: { wax: {} },
        stamps: { first: { parent: "red", mark: "x" } },
        markers: {
            named: { tag: "red", note: "by name" },
            selected: { tag: { id: redTag?.id }, note: "by id" },
        },
        badges: {
            person: { holder: { id: user } },
            declared: {
                holder: { kind: "tag", name: "red" },
                place: { kind: "tag", name: "red" },
            },
            related: { holder: { kind: "tag", name: "red", relation: "write" } },
            sealed: { holder: { anyone: true }, place: { kind: "seal", name: "wax" } },
            scoped: { holder: { scope: "self", relation: "write" } },
            everyone: { holder: { anyone: true } },
            crated: { holder: { id: crateId } },
        },
    });

    // fill the tag's name from its declaration's name, and resolve each reference
    const read = Snapshot.live(database);
    const spaceObject = await Scope.object(read, spaceId);
    const tagObject = {
        packageId: tag.package.id,
        type: "tag",
        scope: spaceId,
        id: redTag?.id,
    };
    const [wax] = await read.read(seal.table).findMany();
    const sealObject = {
        packageId: seal.package.id,
        type: "seal",
        scope: spaceId,
        id: wax?.id,
    };
    expect({
        tags: (await read.read(tag.table).findMany()).map((row) => [row.name, row.color]),
        stamps: (await read.read(stamp.table).findMany()).map((row) => [
            row.managerName,
            row.parentId === redTag?.id,
        ]),
        markers: (await read.read(marker.table).findMany())
            .map((row) => [row.managerName, row.tagId === redTag?.id, row.note])
            .toSorted((left, right) => String(left[0]).localeCompare(String(right[0]))),
        badges: Object.fromEntries(
            (await read.read(badge.table).findMany()).map((row): [string, unknown] => [
                String(row.managerName),
                [row.holder, row.place],
            ]),
        ),
    }).toEqual({
        tags: [["red", "#f00"]],
        stamps: [["first", true]],
        markers: [
            ["named", true, "by name"],
            ["selected", true, "by id"],
        ],
        badges: {
            person: [principal.user.reference(Scope.universe.id, user), spaceObject],
            declared: [tagObject, tagObject],
            related: [{ ...tagObject, relation: "write" }, spaceObject],
            scoped: [{ ...spaceObject, relation: "write" }, spaceObject],
            everyone: [
                { packageId: anyone.package.id, type: "anyone", scope: "*", id: "*" },
                spaceObject,
            ],
            sealed: [
                { packageId: anyone.package.id, type: "anyone", scope: "*", id: "*" },
                sealObject,
            ],
            crated: [crate.reference(spaceId, crateId), spaceObject],
        },
    });

    // refuse an unknown name, an unknown identifier of an applied type and a scoped global principal
    const missing = Identifier.create("tag", world);
    const refused = async (document: StackOptions["document"]) =>
        await refusal(apply({ tags: { red }, ...document }));
    expect([
        await refused({ markers: { lost: { tag: "blue", note: "x" } } }),
        await refused({ badges: { lost: { holder: { id: missing } } } }),
        await refused({ badges: { lost: { holder: { id: redTag?.id, scope: "enclosing" } } } }),
        await refused({ badges: { lost: { holder: { id: user, scope: "self" } } } }),
    ]).toEqual([
        ["BAD_REQUEST", "DECLARATION_INVALID", "tag 'blue' is invalid: it is unknown"],
        ["BAD_REQUEST", "DECLARATION_INVALID", `tag '${missing}' is invalid: it is unknown`],
        ["BAD_REQUEST", "DECLARATION_INVALID", `tag '${redTag?.id}' is invalid: it is unknown`],
        [
            "BAD_REQUEST",
            "DECLARATION_INVALID",
            `user '${user}' is invalid: it is global and has no scope`,
        ],
    ]);
});
