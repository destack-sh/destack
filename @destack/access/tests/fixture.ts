import {
    defineObject,
    relation,
    union,
    permission,
    through,
    intersection,
    exclusion,
    compare,
    attribute,
    literal,
} from "../src/index.ts";
import { defineDatabaseSchema, integer, table, text } from "@destack/db";
import type { ObjectMapping } from "../src/database/index.ts";
import { accessSchema } from "../src/stack/index.ts";
import { defineTree } from "@destack/db/tree";
import { PackageId } from "@destack/package";

/** Declare objects under a fixed test package. */
export function testModule(number: number) {
    const id = PackageId.parse(`package-01996ab0-0000-7000-8000-00000000000${number}`);

    return { package: { id, name: `@example/package-${number}`, version: "2026.9.0" } };
}
/** The first test package. */
export const module1 = testModule(1);
/** The second test package. */
export const module2 = testModule(2);
/** The third test package. */
export const module3 = testModule(3);

/** Notes and nested nodes with direct and inherited sharing. */
export const node = defineObject(
    {
        name: "node",
        attributes: {},
        relations: {
            owner: { kind: "subject", subjects: ["user"] },
            parent: { kind: "object", type: "node" },
            viewer: {
                kind: "grant",
                subjects: ["user", "group", "share-token", "everyone"],
                permission: "share",
            },
            editor: {
                kind: "grant",
                subjects: ["user", "group", "service-account", "share-token"],
                permission: "share",
            },
            "subtree-editor": {
                kind: "grant",
                subjects: ["user", "share-token"],
                permission: "share",
            },
        },
        permissions: {
            share: relation("owner"),
            "edit-descendant": union(relation("owner"), relation("subtree-editor")),
            edit: union(
                relation("owner"),
                relation("editor"),
                relation("subtree-editor"),
                through("parent", "edit-descendant", true),
            ),
            read: union(permission("edit"), relation("viewer")),
        },
    },
    module1,
);

/** Cells constrained by an agent's allowed row and column interval. */
export const cell = defineObject(
    {
        name: "cell",
        attributes: { row: "number", column: "number", locked: "number" },
        relations: { owner: { kind: "subject", subjects: ["user"] } },
        permissions: {
            read: relation("owner"),
            edit: intersection(
                relation("owner"),
                compare(
                    "gte",
                    attribute("object", "row", "number"),
                    attribute("context", "first-row", "number"),
                ),
                compare(
                    "lte",
                    attribute("object", "row", "number"),
                    attribute("context", "last-row", "number"),
                ),
                compare(
                    "gte",
                    attribute("object", "column", "number"),
                    attribute("context", "first-column", "number"),
                ),
                compare(
                    "lte",
                    attribute("object", "column", "number"),
                    attribute("context", "last-column", "number"),
                ),
                compare("eq", attribute("object", "locked", "number"), literal(0)),
            ),
        },
    },
    module2,
);

/** World entities combine team membership with authoritative phase and protection. */
export const entity = defineObject(
    {
        name: "entity",
        attributes: { team: "number", protected: "number" },
        relations: { owner: { kind: "subject", subjects: ["user"] } },
        permissions: {
            read: relation("owner"),
            edit: exclusion(
                intersection(
                    compare(
                        "eq",
                        attribute("object", "team", "number"),
                        attribute("context", "team", "number"),
                    ),
                    compare("eq", attribute("context", "phase", "string"), literal("edit")),
                ),
                compare("eq", attribute("object", "protected", "number"), literal(1)),
            ),
        },
    },
    module3,
);

/** Real application records shared by the three compact integration examples. */
export const item = table("example_item", {
    id: text("id").primaryKey().notNull(),
    scope: text("scope").notNull(),
    owner: text("owner").notNull(),
    parent: text("parent"),
    row: integer("row").notNull(),
    column: integer("column").notNull(),
    locked: integer("locked").notNull(),
    team: integer("team").notNull(),
    protected: integer("protected").notNull(),
});

/** Example mappings keep ownership and hierarchy in their existing columns. */
export const nodeTree = defineTree({
    name: "parent",
    table: item,
    id: "id",
    scope: "scope",
    parent: "parent",
});

/** Map application records and the indexed parent tree to access declarations. */
export const mappings: ObjectMapping[] = [node, cell, entity].map((type): ObjectMapping => ({
    type,
    table: item,
    id: "id",
    scope: "scope",
    attributes: Object.fromEntries(
        Object.keys(type.definition.attributes).map((name) => [name, name]),
    ),
    subjects: { owner: { column: "owner", kind: "user", authority: "global" } },
    objects: type === node ? { parent: "parent" } : {},
    trees: type === node ? { parent: nodeTree } : {},
}));

/** Compose application and access migrations through the regular database lifecycle. */
export const fixtureSchema = defineDatabaseSchema({
    name: "access-example",
    tables: { item, ancestors: nodeTree.ancestors, revision: nodeTree.revision },
    trees: [nodeTree],
    dependencies: [accessSchema],
    migrations: new URL("./migration/", import.meta.url),
});

/** Application records spanning two isolated scopes. */
export const rows = [
    {
        id: "a",
        scope: "personal",
        owner: "alice",
        parent: null,
        row: 1,
        column: 1,
        locked: 0,
        team: 1,
        protected: 0,
    },
    {
        id: "b",
        scope: "personal",
        owner: "alice",
        parent: "a",
        row: 2,
        column: 1,
        locked: 0,
        team: 1,
        protected: 1,
    },
    {
        id: "c",
        scope: "personal",
        owner: "alice",
        parent: "b",
        row: 3,
        column: 1,
        locked: 1,
        team: 2,
        protected: 0,
    },
    {
        id: "d",
        scope: "work",
        owner: "alice",
        parent: null,
        row: 1,
        column: 1,
        locked: 0,
        team: 1,
        protected: 0,
    },
];
