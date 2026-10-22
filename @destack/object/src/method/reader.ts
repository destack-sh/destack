import type { RelationalQueryBuilder, Select, Table } from "@destack/db";
import type { Decision, PermissionReference } from "@destack/access";
import type { JsonValue } from "@destack/schema";
import type * as sync from "@destack/sync";
import type { EventRequest, LookupRequest } from "../hook/record.ts";
import type { BranchEntry } from "../branch/entry.ts";
import type { BranchWrite } from "../branch/write.ts";

/** The columns a read returns: the logged ones, or every one with the sensitive ones. */
export type ReadColumns = "logged" | "all";

/** A hook's session with its store in its mutation: reads, decisions, sends and positions. */
export interface Session {
    /** Read a table's logged columns in the mutation. */
    readonly read: <Definition extends Table>(
        table: Definition,
    ) => RelationalQueryBuilder<Definition>;
    /** Read the whole rows of a type of the hook's package in the mutation. */
    readonly reveal: <Definition extends Table>(
        table: Definition,
    ) => RelationalQueryBuilder<Definition, Select<Definition>>;
    /** Read events of a type of the hook's package, answered as JSON. */
    readonly events: (table: Table, read: EventRequest) => Promise<JsonValue>;
    /** Decide whether the hook's caller has a permission on an object. */
    readonly check: (
        permission: PermissionReference,
        target: sync.ObjectReference,
    ) => Promise<Decision>;
    /** Require the hook's caller to have a permission on an object. */
    readonly require: (
        permission: PermissionReference,
        target: sync.ObjectReference,
    ) => Promise<void>;
    /** List a page of the subjects of a type with a permission on an object. */
    readonly lookup: (request: LookupRequest) => Promise<readonly sync.Subject[]>;
    /** Run calls as their authors over some rows and undo them, answering the rows they change. */
    readonly rehearse: (
        over: readonly BranchWrite[],
        calls: readonly BranchEntry[],
    ) => Promise<readonly BranchWrite[]>;
    /** Take the position of the mutation's next derived identifier. */
    readonly takePosition: () => Promise<number>;
}
