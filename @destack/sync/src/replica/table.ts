import {
    defineTable,
    index,
    integer,
    identifier,
    json,
    text,
    Scalar,
    Table,
    LogPosition,
} from "@destack/db";
import { schema } from "@destack/schema";
import { Page } from "../query/page.ts";
import { PublicationAudience, subscriptionTable } from "./publication.ts";
import { Subject } from "../call/subject.ts";

/** The replicas a database stores: what each replicates and from which subscription. */
export const replica = defineTable(
    "replica",
    {
        /** The replica's name. */
        name: text("name").primaryKey(),
        /** The scope whose rows the replica stores. */
        scope: text("scope").primaryKey(),
        /** The scopes the replica reads, nearest first, absent until its source sends them. */
        scopes: json("scopes", schema.array(schema.string())),
        /** The subjects its source decides the rows for, absent until the source sends them. */
        subjects: json("subjects", schema.array(Subject)),
        /** The layout digest of each replicated table's logged columns, by SQL name. */
        layouts: json("layouts", schema.record(schema.string(), schema.string())),
        /** The replicated tables' SQL names, recorded when the replica registers. */
        tables: json("tables", schema.array(schema.string())).notNull(),
    },
    {
        log: {},
    },
);

/** The scopes each replica stores a table's rows of: a row per replica, table and scope. */
export const replicaScope = defineTable(
    "replica_scope",
    {
        /** The replica's name. */
        name: text("name").primaryKey(),
        /** The replica's scope. */
        scope: text("scope").primaryKey(),
        /** The replicated table, by SQL name. */
        table: text("table").primaryKey(),
        /** A scope the table's rows live in. */
        living: text("living").primaryKey(),
    },
    {
        log: {},
        constraints: (row) => [index("replica_scope_living").on(row.living, row.table)],
    },
);

/** The home position a replica reflects, and the tables it holds. */
export interface Reflection {
    /** The home position. */
    readonly position: LogPosition;
    /** The replicated tables' SQL names. */
    readonly tables: readonly string[];
}

/** The position each replica's rows reached at their source, logged for relays to publish. */
export const replicaPosition = defineTable(
    "replica_position",
    {
        /** The replica's name. */
        name: text("name").primaryKey(),
        /** The scope whose rows the replica stores. */
        scope: text("scope").primaryKey(),
        /** The source log, absent before the first complete snapshot. */
        log: text("log"),
        /** The source log's epoch, absent before the first complete snapshot. */
        epoch: text("epoch"),
        /** The source log sequence, absent before the first complete snapshot. */
        sequence: integer("sequence"),
        /** The home log the rows reflect, absent when the source is their home. */
        originLog: text("origin_log"),
        /** The home log's epoch the rows reflect, absent when the source is their home. */
        originEpoch: text("origin_epoch"),
        /** The home log sequence the rows reflect, absent when the source is their home. */
        originSequence: integer("origin_sequence"),
    },
    {
        log: {},
    },
);

/** The subscription a replica follows, recorded as it registers and by each completed run. */
export const replicaSubscription = defineTable(
    "replica_subscription",
    {
        /** The replica's name. */
        name: text("name").primaryKey(),
        /** The scope whose rows the replica stores. */
        scope: text("scope").primaryKey(),
        /** The publication the replica follows. */
        publication: text("publication").notNull(),
        /** The scope the replica is for. */
        target: text("target").notNull(),
        /** The package of the principal the source decides the rows for, absent for none. */
        subjectPackageId: identifier("subject_package_id", "package"),
        /** The type of the principal decided for, absent for none. */
        subjectType: text("subject_type"),
        /** The scope of the principal decided for, absent for none. */
        subjectScope: text("subject_scope"),
        /** The identifier of the principal decided for, absent for none. */
        subjectId: text("subject_id"),
        /** The relation whose members the subject set is, absent for one principal. */
        subjectRelation: text("subject_relation"),
        /** The subscription's parameters, which its publication parses. */
        parameters: json("parameters", schema.jsonObject()).notNull(),
        /** Whose access its source decides the rows by. */
        audience: text("audience").validate(PublicationAudience).notNull(),
    },
    {
        log: {},
    },
);

/** The staged pages of a replica's latest unfinished run. */
export const replicaPage = defineTable("replica_page", {
    /** The replica's name. */
    name: text("name").primaryKey(),
    /** The replica's scope. */
    scope: text("scope").primaryKey(),
    /** The page's place in the run, from zero. */
    index: integer("index").primaryKey(),
    /** The page as the source sent it. */
    page: json("page", Page).notNull(),
});

/** The aggregate groups a replica stores. */
export const replicaResult = defineTable(
    "replica_result",
    {
        /** The replica's name. */
        name: text("name").primaryKey(),
        /** The replica's scope. */
        scope: text("scope").primaryKey(),
        /** The aggregate query, by its path of names. */
        query: text("query").primaryKey(),
        /** The group's values as canonical JSON text. */
        group: text("group").primaryKey(),
        /** The group's measures by name. */
        values: json("values", schema.record(schema.string(), Scalar)).notNull(),
        /** The rows of the group. */
        rows: integer("rows").notNull(),
        /** Each average's sum and count of present values, by measure name. */
        averages: json(
            "averages",
            schema.record(
                schema.string(),
                schema.object({ sum: Scalar, count: schema.number().int() }),
            ),
        ),
    },
    {
        log: {},
    },
);

/** The keys of the rows each replica stores on the follower, with the columns it hides on each. */
export const replicaKey = defineTable(
    "replica_key",
    {
        /** The replica's name. */
        name: text("name").primaryKey(),
        /** The replica's scope. */
        scope: text("scope").primaryKey(),
        /** The row's table, by SQL name. */
        table: text("table").primaryKey(),
        /** The row's key. */
        key: text("key").primaryKey(),
        /** The columns the replica hides on the row, by property. */
        concealed: json("concealed", schema.array(schema.string())).notNull(),
    },
    {
        constraints: (row) => [index("replica_key_table").on(row.table, row.key)],
        moved: { table: "replica_row" },
    },
);

/** The tables of a replica's record, whose commits decide whether it is current. */
export const replicaRecordTables: readonly Table[] = [
    replica,
    replicaScope,
    replicaPosition,
    replicaSubscription,
];

/** The tables of a database with replicas. */
export const replicaTables: readonly Table[] = [
    subscriptionTable,
    replica,
    replicaScope,
    replicaSubscription,
    replicaPosition,
    replicaPage,
    replicaResult,
    replicaKey,
];
