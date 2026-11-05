import { accessRelationship, Manager } from "@destack/access";
import {
    type ColumnBuilder,
    identifier,
    index,
    integer,
    json,
    type Table,
    type TableColumnMap,
    text,
} from "@destack/db";
import { eventColumns } from "@destack/event";
import type { FieldColumn } from "../field/field.ts";
import { TagMap } from "@destack/resource";
import { present } from "@destack/schema";
import { ObjectType, type ObjectStorage } from "../object/object.ts";
import type { Trait } from "./trait.ts";

/** Declare the columns every record has. */
export function recordColumns<const Prefix extends string>(prefix: Prefix) {
    return {
        /** The immutable record identifier. */
        id: identifier("id", prefix).primaryKey(),
        /** Creation time in UTC epoch milliseconds. */
        createdAt: integer("created_at").notNull(),
        /** The package of the creating subject, absent for the platform's writes. */
        createdByPackageId: identifier("created_by_package_id", "package"),
        /** The type of the creating call's subject. */
        createdByType: text("created_by_type"),
        /** The scope of the creating call's subject. */
        createdByScope: text("created_by_scope"),
        /** The identifier of the creating call's subject. */
        createdById: text("created_by_id"),
        /** Last modification time in UTC epoch milliseconds. */
        updatedAt: integer("updated_at").notNull(),
        /** The package of the last modifying subject, absent for the platform's writes. */
        updatedByPackageId: identifier("updated_by_package_id", "package"),
        /** The type of the last modifying call's subject. */
        updatedByType: text("updated_by_type"),
        /** The scope of the last modifying call's subject. */
        updatedByScope: text("updated_by_scope"),
        /** The identifier of the last modifying call's subject. */
        updatedById: text("updated_by_id"),
        /** The revision used by conditional updates. */
        revision: integer("revision").notNull().default(1),
        /** User-defined labels. */
        tags: json("tags", TagMap).notNull().default({}),
    };
}

/** Declare the columns of a durable record's manager: the declaration managing it, if any. */
export function managedColumns() {
    return {
        /** The installation that applied the declaration managing the record, null for none. */
        managerInstallationId: identifier("manager_installation_id", "installation"),
        /** The package declaring the record. */
        managerPackageId: identifier("manager_package_id", "package"),
        /** The declaration's name within the declaring package. */
        managerName: text("manager_name"),
        /** The time the declaration stopped managing the record, null while managed. */
        detachedAt: integer("detached_at"),
    };
}

/** The columns of a declared record in any table: identity, revision and manager. */
type ManagedColumns = TableColumnMap<
    Pick<RecordBuilderMap, "id" | "createdAt" | "updatedAt" | "revision"> & {
        scope: ColumnBuilder<FieldColumn<string, true, false>>;
    } & ReturnType<typeof managedColumns>
>;

/** A table of declared records, a derived object's or another package's. */
export type ManagedTable = Table<string, ManagedColumns> & ManagedColumns;

/** Declare the columns every event has beside its scope and data: its identifier and time. */
export function eventRecordColumns<const Prefix extends string>(prefix: Prefix) {
    const { time } = eventColumns();

    return { id: identifier("id", prefix).primaryKey(), time };
}

/** The columns every record has, an event's in the event log. */
export type RecordBuilderMap<
    Prefix extends string = string,
    Storage = "durable",
> = Storage extends "events"
    ? ReturnType<typeof eventRecordColumns<Prefix>>
    : ReturnType<typeof recordColumns<Prefix>> & ManagedBuilderMap<Storage>;

/** The columns of a durable record's manager. */
type ManagedBuilderMap<Storage> = Storage extends "durable"
    ? ReturnType<typeof managedColumns>
    : {};

/** Declare the column with an ephemeral object's writer. */
export function writerColumns() {
    return {
        /** The identifier of the client instance that wrote the object. */
        writerId: text("writer_id").notNull(),
    };
}

/** The column with an ephemeral object's writer. */
export type WriterBuilderMap<Storage> = Storage extends "ephemeral"
    ? ReturnType<typeof writerColumns>
    : {};

/** The record trait every object takes, by the objects' storage. */
export const record: Trait<{
    readonly kind: ObjectStorage;
    readonly indexes: readonly string[];
}> = {
    options: (definition) => ({
        kind: ObjectType.storageKind(definition.storage),
        indexes:
            typeof definition.storage === "object" && definition.storage.kind === "events"
                ? (definition.storage.indexes ?? [])
                : [],
    }),
    columns: (_options, object) =>
        object.storage === "events"
            ? eventRecordColumns(object.identity)
            : {
                  ...recordColumns(object.identity),
                  ...(object.storage === "ephemeral" ? writerColumns() : {}),
                  ...(object.storage === "durable" ? managedColumns() : {}),
              },
    // check a durable record's manager, and index the event log by time and by each field it declares indexed
    constraints: ({ kind: storage, indexes }, table, columns) =>
        storage === "durable"
            ? Manager.constraints(table, {
                  managerInstallationId: present(columns["managerInstallationId"], "the manager"),
                  managerPackageId: present(columns["managerPackageId"], "the manager package"),
                  managerName: present(columns["managerName"], "the manager name"),
                  detachedAt: present(columns["detachedAt"], "the detachment"),
              })
            : storage !== "events"
              ? []
              : [
                    index(`${table}_time`).on(
                        present(columns["scope"], "the scope column"),
                        present(columns["time"], "the time column"),
                        present(columns["id"], "the identity column"),
                    ),
                    ...indexes.map((name) => {
                        const column = present(columns[name], `the indexed field ${name}`);

                        return index(`${table}_${column.definition.name}`).on(
                            present(columns["scope"], "the scope column"),
                            column,
                            present(columns["time"], "the time column"),
                        );
                    }),
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
                          where: { objectPackageId: object.packageId, objectType: object.name },
                          onDelete: "cascade",
                      },
                  ],
              },
    methods: () => ({}),
};
