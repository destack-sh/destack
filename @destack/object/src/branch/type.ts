import type { Method } from "../method/method.ts";
import { ObjectError } from "../error/error.ts";
import { TABLE, LogPosition, type Row, Snapshot, type Table, Condition } from "@destack/db";

import { schema } from "@destack/schema";
import * as sync from "@destack/sync";
import { CallSite } from "../method/index.ts";
import type { ObjectController } from "../object/controller.ts";
import { Branch } from "./branch.ts";
import { ObjectKey, ObjectType } from "../object/index.ts";

import type {} from "@destack/package/import-meta";

/** The columns of every branch type's rows: identifier, state, row tables and build position. */
const BRANCH_RECORD = schema.looseObject({
    id: schema.string(),
    state: schema.string(),
    tables: schema.array(schema.string()),
    built: LogPosition,
});

/** A branch as its row records it. */
type BranchRecord = schema.Infer<typeof BRANCH_RECORD>;

/** The type of a branch's entries, which branches append by creating them. */
type BranchEntryType = ObjectType & { readonly methods: { readonly create: Method } };

/** The type of a branch's changes over the main line, which branches create, update, delete. */
type BranchChangeType = ObjectType & {
    readonly methods: { readonly create: Method; readonly update: Method; readonly delete: Method };
};

/** A scope type's branches: the branch type, its entries' type and its changes' type. */
export class BranchType {
    /** The branch type, with push, merge, discard and rebuild methods. */
    readonly object: ObjectType;
    /** The type of a branch's entries, nested in the branch and ordered by position. */
    readonly entry: BranchEntryType;
    /** The type of the changes a branch makes, nested in the branch. */
    readonly change: BranchChangeType;

    /** Bring a scope type's branch types together. */
    constructor(types: {
        readonly object: ObjectType;
        readonly entry: BranchEntryType;
        readonly change: BranchChangeType;
    }) {
        this.object = types.object;
        this.entry = types.entry;
        this.change = types.change;
    }

    /** Read a branch row's identifier, state and row tables, as every branch type stores them. */
    static record(row: Row): BranchRecord {
        return BRANCH_RECORD.parse(row);
    }

    /** Require a branch to be open, refusing one merged or discarded for good. */
    requireOpen<Target extends { readonly state: string }>(target: Target): Target {
        if (target.state !== "open") {
            throw ObjectError.of("BRANCH_NOT_OPEN", `branch is ${target.state}`);
        }

        return target;
    }

    /** Resolve a call a branch stores: a durable object's mutating method, in the branch scope. */
    resolve(
        objects: readonly ObjectType[],
        entry: sync.Call,
        scope: string,
    ): ReturnType<typeof ObjectType.resolve> {
        // resolve the call in the branch's scope, refusing one in another scope or of an unknown method
        const type = ObjectType.find(objects, entry);
        const field =
            type !== undefined && Object.hasOwn(type.methods, entry.method)
                ? type.route.field
                : undefined;
        const named = field === undefined ? undefined : entry.input[field];
        if (named !== undefined && named !== scope) {
            throw ObjectError.of(
                "CALL_OUTSIDE_BRANCH",
                `${sync.Call.label(entry)} is in a different scope from its branch`,
            );
        }
        const input = field === undefined ? entry.input : { ...entry.input, [field]: scope };
        const resolved = ObjectType.resolve(objects, { ...entry, input }, true, CallSite.client);

        // refuse ephemeral objects and branch bookkeeping
        if (resolved.object.storage === "ephemeral" || this.has(resolved.object)) {
            throw ObjectError.of(
                "UNBRANCHABLE_CALL",
                `branches cannot store calls of ${resolved.object.name}`,
            );
        }

        return resolved;
    }

    /** Report whether an object type is a branch type, whose calls stay on the main line. */
    has(object: ObjectType): boolean {
        return [this.object, this.entry, this.change].some((type) => type.same(object));
    }

    /** Read the tables branches change, by SQL name. */
    tables(objects: readonly ObjectType[]): ReadonlyMap<string, Table> {
        return new Map(
            objects
                .filter((object) => object.storage === "durable" && !this.has(object))
                .map((object) => [object.table[TABLE].sqlName, object.table]),
        );
    }

    /** Rebuild open branches whose rows a main-line write may have changed. */
    controller(objects: readonly ObjectType[]): ObjectController {
        const branches = this.object.table;

        return {
            pending: { state: "open" },
            watches: [...this.tables(objects).values()].map((table) => ({
                table,
                keys: async (row, database) => {
                    // select the open branches of the row's scope that touch its table
                    const open = await database
                        .select()
                        .from(branches)
                        .where(
                            Condition.render(
                                {
                                    AND: [
                                        { state: "open" },
                                        ...(typeof row["scope"] === "string"
                                            ? [{ scope: row["scope"] }]
                                            : []),
                                    ],
                                },
                                branches,
                            ),
                        );

                    return open
                        .map((entry) => BranchType.record(entry))
                        .filter((entry) => entry.tables.includes(table[TABLE].sqlName))
                        .map((entry) => ({ id: entry.id }));
                },
            })),
            reconcile: async (reconciliation) => {
                // rebuild the open branches whose rows the main line moved under
                const { database } = reconciliation;
                const stale = [];
                for (const row of reconciliation.rows) {
                    const state = BranchType.record(row);
                    const opened = new Branch(this, Snapshot.live(database), state.id);
                    if (await opened.isStale(database, objects, state)) {
                        stale.push(row);
                    }
                }
                if (stale.length > 0) {
                    await reconciliation.mutate(
                        stale.map((row) =>
                            this.object.call("rebuild", this.object.target(ObjectKey.parse(row))),
                        ),
                    );
                }

                return undefined;
            },
        };
    }
}
