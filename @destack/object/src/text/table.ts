import { defineTable, identifier, index, integer, json, TABLE, text, unique } from "@destack/db";
import { schema } from "@destack/schema";
import { TextRun, TextMark } from "../sequence/index.ts";

/** The include with each owner's chunks. */
export const CHUNKS = "textChunks";

/** The pieces of owners' texts, in position order per field. */
export const textChunkTable = defineTable(
    "text_chunk",
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
            index("text_chunk_run").on(
                columns.parentId,
                columns.field,
                columns.runs.element("run", "text"),
                columns.runs.element("start", "integer"),
            ),
            // order each text's chunks
            index("text_chunk_parent").on(
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
export const runEntry = textChunkTable[TABLE].entryTable("text_chunk_run");

/** The nodes of owners' texts, one row per field and node, such as a mention. */
export const textNodeTable = defineTable(
    "text_node",
    {
        /** The row's identifier. */
        id: identifier("id", "node").primaryKey(),
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
        /** The node's type, such as `mention`. */
        type: text("type").notNull(),
        /** The node's value in canonical form, which tells nodes apart. */
        key: text("key").notNull(),
        /** The node's value, such as the mentioned subject. */
        value: json("value", schema.json()).notNull(),
        /** How many times the field holds the node. */
        count: integer("count").notNull(),
    },
    {
        log: {},
        constraints: (columns) => [
            // hold each node once per field, and find the texts holding a node
            unique("text_node_field").on(
                columns.parentPackageId,
                columns.parentType,
                columns.parentId,
                columns.field,
                columns.type,
                columns.key,
            ),
            index("text_node_value").on(columns.type, columns.key),
        ],
    },
);
