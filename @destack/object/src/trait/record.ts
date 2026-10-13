import { accessRelationship } from "@destack/access";
import { type Column, identifier, index, integer, json, text } from "@destack/db";
import { eventColumns } from "@destack/event";
import { validated } from "../field/field.ts";
import { present, schema } from "@destack/schema";
import { ObjectType, type ObjectStorage } from "../object/object.ts";
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
        /** The creating call's subject key, absent for the platform's own writes. */
        createdBy: text("created_by"),
        /** Last modification time in UTC epoch milliseconds. */
        updatedAt: integer("updated_at").notNull(),
        /** The last modifying call's subject key, absent for the platform's own writes. */
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

/** Declare the columns every event has beside its scope and data: identity and time. */
export function eventRecordColumns() {
    const { id, time } = eventColumns();

    return { id, time };
}

/** The columns every record has, an event's in the event log. */
export type RecordBuilderMap<
    Prefix extends string = string,
    Key = undefined,
    Storage = "durable",
> = Storage extends "events"
    ? ReturnType<typeof eventRecordColumns>
    : Key extends schema.Schema<string>
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

/** The record trait every object takes, by the storage keeping the objects. */
export const record: Trait<ObjectStorage> = {
    options: (definition) => ObjectType.storageKind(definition.storage),
    columns: (_options, object) =>
        object.storage === "events"
            ? eventRecordColumns()
            : {
                  ...recordColumns(object.identity),
                  ...(object.key === undefined ? {} : { id: naturalKeyColumn(object.key) }),
                  ...(object.storage === "ephemeral" ? writerColumns() : {}),
              },
    // index the event log by time, and by each key that keeps no map
    constraints: (storage, table, columns) =>
        storage !== "events"
            ? []
            : [
                  index(`${table}_time`).on(
                      present(columns["scope"], "the scope column"),
                      present(columns["time"], "the time column"),
                      present(columns["id"], "the identity column"),
                  ),
                  ...Object.entries(columns)
                      .filter(([name, column]) => isKeyColumn(name, column))
                      .map(([, column]) =>
                          index(`${table}_${column.definition.name}`).on(
                              present(columns["scope"], "the scope column"),
                              column,
                              present(columns["time"], "the time column"),
                          ),
                      ),
              ],
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

/** Report whether an event column is a key an index covers: beside the event columns, no map. */
function isKeyColumn(name: string, column: Column): boolean {
    return !Object.hasOwn(eventColumns(), name) && column.definition.kind !== "json";
}
