import { principal, relation } from "@destack/access";
import { defineDatabase } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { RequestId, ObjectReference } from "@destack/sync";
import { testCallKey } from "@destack/object/test";
import { schema } from "@destack/schema";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { defineObject, field } from "../src/index.ts";
import { ObjectStore } from "../src/server/index.ts";
import { testEvents, serversOf } from "../src/test/index.ts";
import { request } from "./fixture/request.ts";
import { openSpace, space, spacePrincipal } from "./fixture/space.ts";
import { userContext, users } from "./fixture/user.ts";
import { Clock } from "@destack/environment";
import { tablesOf } from "../src/index.ts";
import objectPackage from "../src/package.ts";

/** The space with the markers. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000007");

/** Labels a marker may point at. */
const label = defineObject(
    {
        name: "label",
        plural: "labels",
        scope: space,
        fields: { owner: field.reference(principal.user).caller() },
        permissions: { read: relation("owner"), write: relation("owner") },
    },
    (method) => ({ create: method.create("write", { fields: [] }) }),
);

/** Notes a marker may not point at. */
const note = defineObject(
    {
        name: "note",
        plural: "notes",
        scope: space,
        fields: { owner: field.reference(principal.user).caller() },
        permissions: { read: relation("owner"), write: relation("owner") },
    },
    (method) => ({ create: method.create("write", { fields: [] }) }),
);

/** Markers pointing at a label each, by its reference. */
const marker = defineObject(
    {
        name: "marker",
        plural: "markers",
        scope: space,
        fields: {
            owner: field.reference(principal.user).caller(),
            target: field.object(label),
        },
        permissions: { read: relation("owner"), write: relation("owner") },
    },
    (method) => ({
        create: method.create("write", {
            fields: ["targetPackageId", "targetType", "targetScope", "targetId"],
        }),
        list: method.list("read"),
    }),
);

test("store a marker's target as its label's reference columns, refuse a note, and list the markers of one label", async () => {
    // open the space's store of labels, notes and markers
    const storage = await TestDatabase.create(
        defineDatabase({
            name: "main",
            ...tablesOf({
                packages: [objectPackage, { objects: { label, note, marker }, tables: [] }],
                follows: [],
                tables: [],
            }),
        }),
        { isMigrated: true },
    );
    onTestFinished(() => storage.close());
    await openSpace(storage.database, spaceId);
    const store = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        servers: serversOf([label, note, marker]),
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database),
        origin: { package: marker.package },
    });
    const context = userContext(users.frank, spaceId);
    const create = (target: ObjectReference) =>
        request(
            store,
            marker,
            "create",
            spaceId,
            {
                requestId: RequestId.create(Date.now()),
                ...ObjectReference.columns("target", target),
            },
            context,
        );

    // mark two labels, one of them twice, and refuse a note
    const first = label.reference(spaceId, label.generateId());
    const second = label.reference(spaceId, label.generateId());
    await create(first);
    await create(first);
    await create(second);
    const refused = await refusal(create(note.reference(spaceId, note.generateId())));

    // list the first label's markers by its key, reading each target back
    const listed = await request(
        store,
        marker,
        "list",
        spaceId,
        { where: ObjectReference.columns("target", first) },
        context,
    );
    expect({
        refused,
        targets: listed.items.map((row) => ObjectReference.at("target", row)),
    }).toEqual({
        refused: [
            "BAD_REQUEST",
            "INVALID_INPUT",
            'invalid input: targetType: invalid input: expected "label"',
        ],
        targets: [first, first],
    });
});
