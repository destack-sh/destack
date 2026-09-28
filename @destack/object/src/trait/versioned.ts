import {
    type ColumnBuilder,
    type DatabaseConnection,
    desc,
    eq,
    integer,
    type Table,
    unique,
} from "@destack/db";
import type { ObjectType } from "../object/object.ts";
import type { Trait } from "./trait.ts";

/** The options of versioned objects. */
export type VersionsDefinition = true;

/** The column numbering versions within their parent. */
export type VersionBuilderMap<Versions> = Versions extends VersionsDefinition
    ? { number: ColumnBuilder<number, true, false> }
    : {};

/** Immutable versions of a parent, numbered within it. */
export const versioned: Trait<VersionsDefinition> & {
    /** Read the number following a parent's latest version. */
    next(object: ObjectType, parentId: string, database: DatabaseConnection): Promise<number>;
} = {
    key: "versioned",
    isDurable: true,
    options: (definition) => definition.versioned,
    columns: () => ({
        number: integer("number").notNull(),
    }),
    constraints: (_options, table, columns) => [
        unique(`${table}_number`).on(columns.parentId!, columns.number!),
    ],
    methods: () => ({}),
    validate: (_options, object, definition) => {
        // keep versions beneath a parent of one type
        if (!definition.nested || definition.nested.in === "any") {
            throw new TypeError(`versions of ${object.name} need a parent of one type`);
        }

        // keep versions immutable
        if (definition.methods && "update" in definition.methods) {
            throw new TypeError(`versions of ${object.name} are immutable and cannot be updated`);
        }
    },
    async next(
        object: ObjectType,
        parentId: string,
        database: DatabaseConnection,
    ): Promise<number> {
        // read the parent's highest number and count on from it
        const table = object.table as Table & Record<string, never>;
        const [latest] = (await database
            .select({ number: table.number })
            .from(table)
            .where(eq(table.parentId, parentId))
            .orderBy(desc(table.number))
            .limit(1)) as { number: number }[];

        return latest === undefined ? 1 : latest.number + 1;
    },
};
