import { accessRelationship } from "@destack/access";
import { identifier, integer, json, text } from "@destack/db";
import { validated } from "../field/field.ts";
import { schema } from "@destack/schema";
import type { Trait } from "./trait.ts";

/** User-defined labels indexed by name. */
export const TagMap = schema.record(schema.string().min(1).max(128), schema.string().max(256));
/** User-defined labels indexed by name. */
export type TagMap = schema.Infer<typeof TagMap>;

/** Declare a label map with an empty database default. */
export function tags(name = "tags") {
    return json(name, TagMap).notNull().default({});
}

/** Declare the columns every record has. */
export function recordColumns<const Prefix extends string>(prefix: Prefix) {
    return {
        /** The immutable record identifier. */
        id: identifier("id", prefix).primaryKey(),
        /** Creation time in UTC epoch milliseconds. */
        createdAt: integer("created_at").notNull(),
        /** The subject whose call created the record, by subject key, absent for the platform's own writes. */
        createdBy: text("created_by"),
        /** Last modification time in UTC epoch milliseconds. */
        updatedAt: integer("updated_at").notNull(),
        /** The subject whose call last modified the record, by subject key, absent for the platform's own writes. */
        updatedBy: text("updated_by"),
        /** The revision used by conditional updates. */
        revision: integer("revision").notNull().default(1),
        /** User-defined labels. */
        tags: tags(),
    };
}

/** Declare the identifier column of objects identified by their natural key. */
export function naturalKeyColumn<Key extends schema.Schema<string>>(key: Key) {
    return validated("id", key).primaryKey();
}

/** The columns every record has, its identifier the natural key when the objects declare one. */
export type RecordBuilderMap<Prefix extends string = string, Key = undefined> =
    Key extends schema.Schema<string>
        ? Omit<ReturnType<typeof recordColumns<Prefix>>, "id"> & {
              readonly id: ReturnType<typeof naturalKeyColumn<Key>>;
          }
        : ReturnType<typeof recordColumns<Prefix>>;

/** Declare the column holding an ephemeral object's writer. */
export function writerColumns() {
    return {
        /** The identifier of the client instance that wrote the object. */
        writerId: text("writer_id").notNull(),
    };
}

/** The column holding an ephemeral object's writer. */
export type WriterBuilderMap<Storage> = Storage extends "ephemeral"
    ? ReturnType<typeof writerColumns>
    : {};

/** The record trait every object takes. */
export const record: Trait<true> = {
    options: () => true,
    columns: (_options, object) => ({
        ...recordColumns(object.identity),
        ...(object.key === undefined ? {} : { id: naturalKeyColumn(object.key) }),
        ...(object.storage === "ephemeral" ? writerColumns() : {}),
    }),
    constraints: () => [],
    // cascade durable objects' relationships
    table: (_options, object) =>
        object.storage !== "durable"
            ? {}
            : {
                  dependents: [
                      {
                          from: () => accessRelationship,
                          key: "objectId",
                          where: { packageId: object.packageId, type: object.name },
                          onDelete: "cascade",
                      },
                  ],
              },
    methods: () => ({}),
};
