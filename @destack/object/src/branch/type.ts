import type { Method } from "../method/method.ts";
import { TABLE, type Row, type Table, Condition } from "@destack/db";
import { ServiceError } from "@destack/service/error";
import { schema } from "@destack/schema";
import type * as sync from "@destack/sync";
import { Call } from "../method/index.ts";
import type { ObjectController } from "../object/controller.ts";
import { ObjectKey, type ObjectType } from "../object/index.ts";

/** The columns of every branch type's rows: identifier, state and the branch's row tables. */
const BRANCH_RECORD = schema.looseObject({
    id: schema.string(),
    state: schema.string(),
    tables: schema.array(schema.string()),
});

/** A branch as its row records it. */
type BranchRecord = schema.Infer<typeof BRANCH_RECORD>;

/** The type of a branch's calls, which branches append by creating them. */
type BranchCallType = ObjectType & { readonly methods: { readonly create: Method } };

/** The type of a branch's changes over the main line, which branches create, update, delete. */
type BranchChangeType = ObjectType & {
    readonly methods: { readonly create: Method; readonly update: Method; readonly delete: Method };
};

/** A scope type's branches: the branch type, the type of its calls, and the type of its changes. */
export class BranchType {
    /** The branch type, with push, merge, discard and rebuild methods. */
    readonly object: ObjectType;
    /** The type of a branch's calls, nested in the branch and ordered by position. */
    readonly call: BranchCallType;
    /** The type of the changes a branch makes, nested in the branch. */
    readonly change: BranchChangeType;

    /** Bring a scope type's branch types together. */
    constructor(types: {
        readonly object: ObjectType;
        readonly call: BranchCallType;
        readonly change: BranchChangeType;
    }) {
        this.object = types.object;
        this.call = types.call;
        this.change = types.change;
    }

    /** Read a branch row's identifier, state and row tables, as every branch type keeps them. */
    static record(row: Row): BranchRecord {
        return BRANCH_RECORD.parse(row);
    }

    /** Require a branch to be open, refusing one merged or discarded for good. */
    requireOpen<Target extends { readonly state: string }>(target: Target): Target {
        if (target.state !== "open") {
            throw new ServiceError("CONFLICT", { message: `branch is ${target.state}` });
        }

        return target;
    }

    /** Resolve a call a branch keeps: a durable object's mutating method, in the branch's scope. */
    resolve(
        objects: readonly ObjectType[],
        entry: sync.Call,
        scope: string,
    ): ReturnType<typeof Call.resolve> {
        // resolve the call in the branch's scope, refusing one in another scope
        const type = objects.find((served) => entry.method.startsWith(`${served.name}.`));
        const field = type?.route.field;
        const named = field === undefined ? undefined : entry.input[field];
        if (named !== undefined && named !== scope) {
            throw new ServiceError("BAD_REQUEST", {
                message: `${entry.method} acts in another scope than its branch`,
            });
        }
        const input = field === undefined ? entry.input : { ...entry.input, [field]: scope };
        const resolved = Call.resolve(objects, { ...entry, input }, true, true);

        // refuse ephemeral objects and branch bookkeeping, whose writes no branch keeps
        if (resolved.object.storage === "ephemeral" || this.keeps(resolved.object)) {
            throw new ServiceError("BAD_REQUEST", {
                message: `branches keep no calls of ${resolved.object.name}`,
            });
        }

        return resolved;
    }

    /** Report whether an object type keeps branches, whose calls stay on the main line. */
    keeps(object: ObjectType): boolean {
        return [this.object, this.call, this.change].some((type) => type.same(object));
    }

    /** Read the tables branches change, by SQL name. */
    tables(objects: readonly ObjectType[]): ReadonlyMap<string, Table> {
        return new Map(
            objects
                .filter((object) => object.storage === "durable" && !this.keeps(object))
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
                    // select the open branches of the row's scope reaching its table
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
                await reconciliation.mutate(
                    reconciliation.rows.map((row) =>
                        Call.record(
                            this.object,
                            "rebuild",
                            this.object.target(ObjectKey.parse(row)),
                        ),
                    ),
                );

                return undefined;
            },
        };
    }
}
