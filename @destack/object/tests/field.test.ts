import { expect, expectTypeOf, onTestFinished, test } from "@destack/test";
import { eq, TABLE } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { schema, present, type Identifier } from "@destack/schema";
import { relation } from "@destack/access";
import { v7 } from "uuid";
import { describeObject } from "../src/inspect/index.ts";
import { defineObject, field, type Field, type ObjectType } from "../src/index.ts";
import { space } from "./fixture/space.ts";
import { objectDatabase, task } from "./schema.ts";

test.each(TEST_DIALECTS)(
    "keep qualified references to objects of another scope on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, objectDatabase, { isMigrated: true });
        onTestFinished(() => storage.close());
        const database = storage.database;

        // store a task referring to a task of another space by scope and identifier
        const origin = {
            scope: schema.identifier("space").parse(`space-${v7()}`),
            id: schema.identifier("task").parse(`task-${v7()}`),
        };
        const id = schema.identifier("task").parse(`task-${v7()}`);
        await database.insert(task.table).values({
            id,
            scope: schema.identifier("space").parse(`space-${v7()}`),
            ownerId: "user-1",
            title: "Follow up",
            origin,
            createdAt: Date.now(),
            updatedAt: Date.now(),
        });
        const [stored] = await database.select().from(task.table).where(eq(task.table.id, id));
        expect(present(stored, "the stored task").origin).toEqual(origin);

        // reject a reference to an object of another type
        expect(
            task.table[TABLE].insertSchema().shape.origin.safeParse({
                scope: origin.scope,
                id: `comment-${v7()}`,
            }).success,
        ).toBe(false);
        expect(describeObject(task).fields).toEqual({ estimate: { read: "plan", write: "plan" } });
    },
);

/** The value a field keeps. */
type ValueOf<Declared> =
    Declared extends Field<infer Configuration> ? Configuration["value"] : never;

/** Notes naming an author of the identity their reference resolves to. */
const note = defineObject({
    name: "note",
    plural: "notes",
    scope: space,
    fields: { author: field.reference("person", (): ObjectType => person) },
    permissions: { read: relation("scope") },
});

/** Notes naming an author of an identity their reference does not resolve to. */
const misnamed = defineObject({
    name: "misnamed",
    plural: "misnameds",
    scope: space,
    fields: { author: field.reference("writer", (): ObjectType => person) },
    permissions: { read: relation("scope") },
});

/** People that notes name as authors. */
const person = defineObject({
    name: "person",
    plural: "people",
    scope: space,
    fields: { name: field.string() },
    permissions: { read: relation("scope") },
});

test("type a later reference by the identity it names, and refuse a type of another identity when its table is built", () => {
    const author = field.reference("person", (): ObjectType => person);
    expectTypeOf<ValueOf<typeof author>>().toEqualTypeOf<Identifier<"person">>();
    expect(
        note.table[TABLE].constraints("postgresql").map((constraint) => constraint.kind),
    ).toEqual(["foreignKey"]);
    expect(() => misnamed.table[TABLE].constraints("postgresql")).toThrow(
        "a reference to writer resolves to object type person of identity person",
    );
});
