import { principal, relation } from "@destack/access";
import { defineDatabase } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { RequestId } from "@destack/runtime/request";
import { testCallKey } from "@destack/runtime/test";
import { schema } from "@destack/schema";
import { ObjectReference } from "@destack/sync";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { defineObject, field } from "../src/index.ts";
import { ObjectStore } from "../src/server/index.ts";
import { callTables, testEvents, serversOf } from "../src/test/index.ts";
import { request } from "./fixture/request.ts";
import { openSpace, space, spacePrincipal } from "./fixture/space.ts";
import { userContext, users } from "./fixture/user.ts";
import { Clock } from "@destack/environment";

/** The space with the pins. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000007");

/** Labels a pin may point at. */
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

/** Notes a pin may not point at. */
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

/** Pins pointing at a label each, by its reference. */
const pin = defineObject(
    {
        name: "pin",
        plural: "pins",
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

test("store a pin's target as its label's reference columns, refuse a note, and list the pins of one label", async () => {
    // open the space's store of labels, notes and pins
    const storage = await TestDatabase.create(
        "sqlite",
        defineDatabase({
            name: "main",
            tables: [...callTables, ...label.tables, ...note.tables, ...pin.tables],
        }),
        { isMigrated: true },
    );
    onTestFinished(() => storage.close());
    await openSpace(storage.database, spaceId);
    const store = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        servers: serversOf([label, note, pin]),
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database),
        origin: { package: pin.package },
    });
    const context = userContext(users.frank, spaceId);
    const create = (target: ObjectReference) =>
        request(
            store,
            pin,
            "create",
            spaceId,
            {
                requestId: RequestId.create(Date.now()),
                ...ObjectReference.columns("target", target),
            },
            context,
        );

    // pin two labels, one of them twice, and refuse a note
    const first = label.reference(spaceId, label.generateId());
    const second = label.reference(spaceId, label.generateId());
    await create(first);
    await create(first);
    await create(second);
    const refused = await refusal(create(note.reference(spaceId, note.generateId())));

    // list the first label's pins by its key, reading each target back
    const listed = await request(
        store,
        pin,
        "list",
        spaceId,
        { where: ObjectReference.columns("target", first) },
        context,
    );
    expect({
        refused,
        targets: listed.items.map((row) => ObjectReference.at("target", row)),
    }).toEqual({
        refused: ["BAD_REQUEST", 'invalid input: targetType: invalid input: expected "label"'],
        targets: [first, first],
    });
});
