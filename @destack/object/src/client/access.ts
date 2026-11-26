import type { OrderBy, Row } from "@destack/db";
import type { Duration, JsonObject } from "@destack/schema";
import type { Item } from "@destack/sync";
import type * as syncDataflow from "@destack/sync/dataflow";
import type { BranchChange } from "../branch/index.ts";
import type { CallableName, MethodInput, MethodOutput, MutationName } from "../method/procedure.ts";
import type { ObjectType } from "../object/object.ts";
import type { ObjectClient } from "../device/client.ts";
import type { ObjectNode } from "./node.ts";
import type { ObjectQueryBuilder, RelationalQuery } from "./query.ts";

/** A mutation a client predicted. */
export interface Submission<Value> {
    /** The result the local prediction returned, once the mutation is queued. */
    readonly predicted: Promise<Value>;
    /** Settles once the replica has the server's changes, or rejects with its failure. */
    readonly confirmed: Promise<void>;
}

/** A live query a client reads from its replica, predictions included. */
export interface LiveQuery<Value = readonly Item[]> {
    /** Settles once the replica has the query, or once the query closes. */
    readonly ready: Promise<void>;
    /** Read the query's value: its items with their includes, or its groups when aggregating. */
    readonly read: () => Promise<Value>;
    /** Read the query again after every local commit until the signal aborts. */
    readonly watch: (signal: AbortSignal) => AsyncGenerator<Value>;
    /** Stop following the query. */
    readonly close: () => Promise<void>;
}

/** The changes a branch makes over the main line, as a client follows them. */
export interface BranchDiff {
    /** Settles once the replica has the branch's rows. */
    readonly ready: Promise<void>;
    /** Read the changes, one per object. */
    readonly read: () => Promise<BranchChange[]>;
    /** Read the changes again after every change of the branch's rows, until the signal aborts. */
    readonly watch: (signal: AbortSignal) => AsyncGenerator<BranchChange[]>;
    /** Stop following the branch. */
    readonly close: () => Promise<void>;
}

/** The mutating methods of an object type, one pending mutation per call. */
export type Mutator<Object extends ObjectType> = {
    readonly [Name in MutationName<Object>]: (
        input: MethodInput<Object, Name>,
    ) => Submission<MethodOutput<Object, Name>>;
};

/** The mutating methods of an object type within a mutation, returning predictions. */
export type Writer<Object extends ObjectType> = {
    readonly [Name in MutationName<Object>]: (
        input: MethodInput<Object, Name>,
    ) => Promise<MethodOutput<Object, Name>>;
};

/** The read-only methods of an object type, read from the server. */
export type Reader<Object extends ObjectType> = {
    readonly [Name in QueryName<Object>]: (
        input: MethodInput<Object, Name>,
    ) => Promise<MethodOutput<Object, Name>>;
};

/** The context of several calls a client makes as one atomic mutation. */
export interface MutationContext {
    /** Call an object type's mutating methods within the mutation. */
    readonly call: <Object extends ObjectType>(object: Object) => Writer<Object>;
    /** Call an object type's mutating method by name in the mutation, returning its prediction. */
    readonly invoke: (object: ObjectType, name: string, input: JsonObject) => Promise<unknown>;
}

/** What a client replicates and follows. */
export interface ClientInspection {
    /** The scope whose objects it replicates. */
    readonly scope: string;
    /** The rows its replica has, by object type name. */
    readonly rows: Readonly<Record<string, number>>;
    /** The most object rows the replica stores, absent for no limit. */
    readonly storage?: { readonly rows: number };
    /** The queries the store follows, open, lapsing and kept forever. */
    readonly followed: readonly {
        /** The query's name. */
        readonly name: string;
        /** The follower: live queries, ones kept forever, or a stream. */
        readonly follower: string;
        /** The time the follow lapses, in UTC epoch milliseconds, null while followed. */
        readonly expiresAt: number | null;
    }[];
    /** The replica of the scope. */
    readonly replica: syncDataflow.ReplicaInspection;
    /** The queued mutations by outcome. */
    readonly mutations: {
        /** The mutations waiting for the server. */
        readonly pending: number;
        /** The mutations the server executed. */
        readonly executed: number;
        /** The mutations the server rejected. */
        readonly rejected: number;
    };
    /** The local feed serving live queries. */
    readonly feed: syncDataflow.FeedInspection;
}

/** The relational queries of a client's object types, by key. */
export type ObjectQueries<Objects extends Readonly<Record<string, ObjectType>>> = {
    readonly [Key in keyof Objects]: ObjectQueryBuilder<Objects[Key], Objects[keyof Objects]>;
};

/** How a union orders and limits its members' rows, and how long it stays followed once closed. */
export interface UnionOptions<Read = Row> {
    /** How the merged rows sort, by fields every member's rows share. */
    readonly orderBy: OrderBy<Read>;
    /** The most rows the union has. */
    readonly limit: number;
    /** How long the closed union stays followed. */
    readonly ttl?: Duration;
}

/** The items of a union's members, whose shared fields the union orders by. */
export type MemberItem<
    Members extends Readonly<Record<string, RelationalQuery<readonly object[]>>>,
> = Members[keyof Members] extends RelationalQuery<readonly (infer Selected)[]> ? Selected : never;

/** One entry of a union: a member's name and a row its query selects. */
export type UnionEntry<
    Members extends Readonly<Record<string, RelationalQuery<readonly object[]>>>,
> = {
    readonly [Name in keyof Members & string]: {
        readonly name: Name;
        readonly row: Members[Name] extends RelationalQuery<readonly (infer Selected)[]>
            ? Selected
            : never;
    };
}[keyof Members & string];

/** The mutators of object types, by key. */
export type ObjectMutators<Objects extends Readonly<Record<string, ObjectType>>> = {
    readonly [Key in keyof Objects]: Mutator<Objects[Key]>;
};

/** Some of a client's object types by key, with the verbs of the client's scope. */
export interface ObjectAccess<Objects extends Readonly<Record<string, ObjectType>>> {
    /** The relational queries, by key. */
    readonly query: ObjectQueries<Objects>;
    /** The mutators, by key. */
    readonly mutate: ObjectMutators<Objects>;
    /** Make several calls as one atomic mutation, predicted at once and undone as one step. */
    readonly mutation: ObjectClient["mutation"];
    /** The party's undo and redo stacks. */
    readonly undoStack: ObjectNode["undoStack"];
    /** Call an object type's mutating method by name as one mutation. */
    readonly call: ObjectClient["call"];
    /** Check whether the caller has a permission on an object. */
    readonly check: ObjectClient["check"];
}

/** The names of an object type's methods that change nothing. */
export type QueryName<Object extends ObjectType> = {
    [Name in CallableName<Object>]: Object["methods"][Name] extends { mutates: false }
        ? Name
        : never;
}[CallableName<Object>];
