import { defineTable, identifier, index, integer, json, text } from "@destack/db";
import { schema } from "@destack/schema";
import { Run } from "../sequence/index.ts";

/** The pieces of owners' texts, in position order per field. */
export const chunk = defineTable(
    "chunk",
    {
        /** The chunk's identifier. */
        id: identifier("id", "chunk").primaryKey(),
        /** The scope with the owner. */
        scope: text("scope").notNull(),
        /** The package declaring the owner's type. */
        parentPackageId: identifier("parent_package_id", "package").notNull(),
        /** The owner's type. */
        parentType: text("parent_type").notNull(),
        /** The owner's identifier. */
        parentId: text("parent_id").notNull(),
        /** The owner's text field. */
        field: text("field").notNull(),
        /** The chunk's place among the field's chunks. */
        position: text("position").notNull(),
        /** The chunk's runs in sequence order. */
        runs: json("runs", schema.array(Run)).notNull(),
    },
    {
        log: {},
        constraints: (columns) => [
            index("chunk_parent").on(
                columns.parentPackageId,
                columns.parentType,
                columns.parentId,
                columns.field,
                columns.position,
            ),
        ],
    },
);

/** The server's index of which chunk contains each run's elements, unsynced. */
export const chunkRun = defineTable(
    "chunk_run",
    {
        /** The chunk with the elements. */
        chunk: identifier("chunk", "chunk")
            .notNull()
            .references(() => chunk.id, { onDelete: "cascade" }),
        /** The scope with the owner. */
        scope: text("scope").notNull(),
        /** The package declaring the owner's type. */
        parentPackageId: identifier("parent_package_id", "package").notNull(),
        /** The owner's type. */
        parentType: text("parent_type").notNull(),
        /** The owner's identifier. */
        parentId: text("parent_id").notNull(),
        /** The owner's text field. */
        field: text("field").notNull(),
        /** The run the elements belong to. */
        run: text("run").notNull(),
        /** The first element's offset. */
        start: integer("start").notNull(),
        /** The offset after the last element. */
        end: integer("end").notNull(),
    },
    {
        constraints: (columns) => [
            index("chunk_run_parent").on(
                columns.parentPackageId,
                columns.parentType,
                columns.parentId,
                columns.field,
                columns.run,
            ),
            index("chunk_run_chunk").on(columns.chunk),
        ],
    },
);
