import { present, schema } from "@destack/schema";
import { type ColumnBuilder, integer, unique } from "@destack/db";
import type { FieldColumn } from "../field/field.ts";
import type { MethodContext } from "../method/context.ts";
import type { ObjectType } from "../object/object.ts";
import type { Trait } from "./trait.ts";

/** The options of versioned objects. */
export type VersionsDefinition = true;

/** The column numbering versions within their parent. */
export type VersionBuilderMap<Versions> = Versions extends VersionsDefinition
    ? { number: ColumnBuilder<FieldColumn<number, true, false>> }
    : {};

/** Immutable versions of a parent, numbered within it. */
export const versioned: Trait<VersionsDefinition> & {
    /** Read the number following a parent's latest version. */
    readonly next: (
        object: ObjectType,
        parentId: string,
        context: MethodContext,
    ) => Promise<number>;
} = {
    key: "versioned",
    storages: ["durable"],
    options: (definition) => definition.versioned,
    columns: () => ({
        number: integer("number").notNull(),
    }),
    constraints: (_options, table, columns) => [
        unique(`${table}_number`).on(
            present(columns["parentId"], "the parent column"),
            present(columns["number"], "the number column"),
        ),
    ],
    methods: () => ({}),
    validate: (_options, object, definition) => {
        // require versions beneath a parent of one type
        if (!definition.nested || definition.nested.in === "any") {
            throw new TypeError(`versions of ${object.name} need a parent of one type`);
        }

        // refuse updates of versions
        if (definition.methods && "update" in definition.methods) {
            throw new TypeError(`versions of ${object.name} are immutable and cannot be updated`);
        }
    },
    async next(object: ObjectType, parentId: string, context: MethodContext): Promise<number> {
        // read the parent's highest number and count on from it
        const [latest] = await context
            .read(object.table)
            .findMany({ where: { parentId }, orderBy: { number: "desc" }, limit: 1 });

        return latest === undefined ? 1 : schema.number().parse(latest["number"]) + 1;
    },
};
