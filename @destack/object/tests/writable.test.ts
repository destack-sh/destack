import { testSimulation } from "../src/test/index.ts";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { principal, relation } from "@destack/access";
import { TestDatabase } from "@destack/db/test";
import { schema } from "@destack/schema";
import { RequestId, Scope } from "@destack/sync";
import type { Call } from "@destack/sync";
import { testCallKey } from "@destack/object/test";
import { defineObject, field } from "../src/index.ts";
import { testEvents, serversOf } from "../src/test/index.ts";
import { openSpace, space, spacePrincipal } from "./fixture/space.ts";
import { userContext, users } from "./fixture/user.ts";
import { tablesOf } from "../src/index.ts";
import objectPackage from "../src/package.ts";
import { TestObjectStore } from "../src/test/index.ts";

/** The public space the labels live in. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000061");

/** Labels their owners write, whose color is chosen once and which are never deleted. */
const label = defineObject(
    {
        name: "label",
        plural: "labels",
        scope: space,
        fields: {
            owner: field.reference(principal.user).caller(),
            name: field.string(),
            color: field.string().immutable(),
        },
        permissions: { read: relation("owner"), write: relation("owner") },
    },
    () => ({ delete: false }),
);

test("derive create and update from the write permission, leaving removed methods and immutable fields out", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // serve the labels in a space alice reads
    const storage = await TestDatabase.create(
        tablesOf({
            packages: [objectPackage, { objects: { label }, tables: [] }],
            follows: [],
            tables: [],
        }).tables,
        { environment: world, isMigrated: true },
    );
    onTestFinished(() => storage.close());
    await openSpace(storage.database, world, spaceId);
    const store = await TestObjectStore.receive(space.reference(Scope.universe.id, spaceId), {
        chain: async () => undefined,
        environment: world,
        principal: spacePrincipal(spaceId),
        servers: serversOf([label]),
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database, world),
        origin: { package: label.package },
    });
    const alice = userContext(users.alice, spaceId, { environment: world }).access();
    const mutate = <const Calls extends readonly Call[]>(calls: Calls) =>
        store.mutate(alice, { id: RequestId.create(world), calls });

    // create a label with its color, rename it, and refuse a change of its color
    const [created] = await mutate([label.call("create", { spaceId, name: "Bug", color: "red" })]);
    const rename = label.call("update", { spaceId, id: created.id, name: "Defect" });
    const [renamed] = await mutate([rename]);
    const recolored = await refusal(
        mutate([{ ...rename, input: { ...rename.input, color: "blue" } }]),
    );

    // the type has create, get, list and update, and the label has its first color
    expect({
        methods: Object.keys(label.methods).toSorted(),
        label: [renamed.name, renamed.color],
        recolored: recolored[0],
    }).toEqual({
        methods: ["create", "get", "list", "update"],
        label: ["Defect", "red"],
        recolored: "BAD_REQUEST",
    });
});

/** Declare a recoverable label updating its immutable color, or removing its derived delete. */
function declare(methods: "update" | "delete"): () => unknown {
    return () =>
        defineObject(
            {
                name: "label",
                plural: "labels",
                scope: space,
                fields: { name: field.string(), color: field.string().immutable() },
                permissions: {
                    read: relation("owner"),
                    write: relation("owner"),
                    manage: relation("owner"),
                },
                recoverable: { within: { days: 30 }, by: "manage" },
            },
            (method) =>
                methods === "update"
                    ? { update: method.update("write", { fields: ["name", "color"] }) }
                    : { delete: false },
        );
}

test("refuse an update of an immutable field, and the removal of a method another trait derives", () => {
    // both declarations throw
    expect(declare("update")).toThrow(
        new TypeError("object label updates an immutable field in method update"),
    );
    expect(declare("delete")).toThrow(
        new TypeError("object label declares or removes method delete, but a trait derives it"),
    );
});
