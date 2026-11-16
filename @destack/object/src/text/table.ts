import { defineTable, identifier, index, json, TABLE, text } from "@destack/db";
import { schema } from "@destack/schema";
import { TextRun, TextMark } from "../sequence/index.ts";

/** The include with each owner's chunks. */
export const CHUNKS = "chunks";

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
        runs: json("runs", schema.array(TextRun)).notNull(),
        /** The marks starting at an element of the chunk. */
        marks: json("marks", schema.array(TextMark)).notNull().default([]),
        /** The chunk's visible characters, which searches read. */
        text: text("text").notNull(),
    },
    {
        log: {},
        fullText: ["text"],
        constraints: (columns) => [
            // index pieces by text and run, and nodes by type and value
            index("chunk_run").on(
                columns.parentId,
                columns.field,
                columns.runs.element("run", "text"),
                columns.runs.element("start", "integer"),
            ),
            index("chunk_node")
                .on(
                    columns.runs.element("node.type", "text"),
                    columns.runs.element("node.value", "json"),
                )
                .sparse(),

            // order each text's chunks
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

/** The pieces of every chunk by text, run and first offset: where a run's elements are. */
export const runEntry = chunk[TABLE].entryTable("chunk_run");

/** The nodes of every chunk by type and value, such as the mentions of a subject. */
export const nodeEntry = chunk[TABLE].entryTable("chunk_node");
