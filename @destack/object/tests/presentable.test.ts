import { expect, onTestFinished, test } from "@destack/test";
import { testCallKey } from "@destack/service/test";
import { principal, relation } from "@destack/access";
import { journal } from "@destack/audit/stack";
import { defineDatabase, TABLE } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { ResourceDeclaration } from "@destack/resource";
import { present, schema } from "@destack/schema";
import { RequestId } from "@destack/service/request";
import { defineObject, field } from "../src/index.ts";
import { describeObject } from "../src/inspect/index.ts";
import { ObjectServer } from "../src/server/index.ts";
import { openSpace, space } from "./fixture/space.ts";
import { userContext } from "./fixture/user.ts";

/** The space containing the boards. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-0000000000c1");

/** The bucket keeping the boards' covers. */
const media = new ResourceDeclaration<unknown, { name: string; kind: "bucket"; spec: {} }>(
    space.package,
    {
        name: "media",
        kind: "bucket",
        spec: {},
    },
);

/** Boards presented by their name with a chosen icon, an accent and a cover, found by name and ordered by rank. */
const board = defineObject({
    name: "board",
    plural: "boards",
    scope: space,
    fields: {
        owner: field.reference(principal.user).caller(),
        name: field.string(),
        summary: field.string().optional(),
        rank: field.integer().default(0),
    },
    presentable: { subtitle: "summary", cover: media },
    search: { searchable: ["name", "summary"], filterable: ["ownerId"], sortable: ["rank"] },
    permissions: { read: relation("owner"), write: relation("owner") },
    methods: (method) => ({
        get: method.get("read"),
        create: method.create("write", { fields: ["name", "icon", "color", "cover"] }),
    }),
});

test("present an object by its name, subtitle, chosen icon, accent and cover, described for pickers with its query roles and an index per filtered or sorted field", async () => {
    // keep a board with an icon, an accent and a cover
    const storage = await TestDatabase.create(
        present(TEST_DIALECTS.at(-1), "the last test dialect"),
        defineDatabase({ name: "board", tables: [...board.tables, journal] }),
        { isMigrated: true },
    );
    onTestFinished(() => storage.close());
    await openSpace(storage.database, spaceId);
    const server = new ObjectServer({
        objects: { board },
        database: storage.database,
        callKey: testCallKey,
        origin: { package: board.package, service: "test" },
    });
    const created = await server.call(
        board,
        "create",
        {
            spaceId,
            requestId: RequestId.create(),
            name: "Roadmap",
            icon: { emoji: "🗺️" },
            color: "teal",
            cover: { file: { key: "covers/roadmap.png" }, focus: 0.4 },
        },
        userContext("user-1", spaceId),
    );
    const described = describeObject(board);

    expect({
        stored: [created.name, created.icon, created.color, created.cover],
        presentation: described.presentation,
        search: described.search,
        indexes: board.table[TABLE]
            .constraints(storage.database.dialect)
            .flatMap((constraint) => (constraint.kind === "index" ? [constraint.name] : []))
            .toSorted(),
    }).toEqual({
        stored: [
            "Roadmap",
            { emoji: "🗺️" },
            "teal",
            { file: { key: "covers/roadmap.png" }, focus: 0.4 },
        ],
        presentation: {
            title: "name",
            subtitle: "summary",
            icon: { field: "icon" },
            color: { field: "color" },
            cover: { field: "cover", bucket: { package: space.package.id, name: "media" } },
        },
        search: { searchable: ["name", "summary"], filterable: ["ownerId"], sortable: ["rank"] },
        indexes: ["board_ownerId", "board_rank"],
    });
});

/** Declare a card of a body and further definition, deferred for a refusal to throw. */
function declare(definition: Record<string, unknown>) {
    return () =>
        defineObject({
            name: "card",
            plural: "cards",
            scope: space,
            fields: { body: field.json(schema.json()) },
            ...definition,
        });
}

test("refuse a presentation without a title field, a field the presentation adds itself, and a query role over a field it cannot read", () => {
    expect(declare({ presentable: true })).toThrow("a title or name field of card");
    expect(
        declare({
            fields: { title: field.string(), color: field.string() },
            presentable: true,
        }),
    ).toThrow("object card declares field color, which its presentation adds");
    expect(declare({ search: { sortable: ["body"] } })).toThrow(
        "object card declares body sortable, which is no sortable field",
    );
});
