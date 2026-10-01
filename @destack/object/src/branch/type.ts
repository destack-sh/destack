import { TABLE, type Table } from "@destack/db";
import { Condition } from "@destack/db/query";
import { ServiceError } from "@destack/service/error";
import type * as sync from "@destack/sync";
import { Call } from "../method/index.ts";
import type { ObjectController } from "../object/controller.ts";
import type { ObjectType } from "../object/index.ts";

/** A scope type's branches: the branch type, the type of its calls, and the type of the rows it changes. */
export class BranchType {
    /** The branch type, with push, merge, discard and rebuild methods. */
    readonly object: ObjectType;
    /** The type of a branch's calls, nested in the branch and ordered by position. */
    readonly call: ObjectType;
    /** The type of the rows a branch changes, nested in the branch. */
    readonly row: ObjectType;

    /** Bring a scope type's branch types together. */
    constructor(types: {
        readonly object: ObjectType;
        readonly call: ObjectType;
        readonly row: ObjectType;
    }) {
        this.object = types.object;
        this.call = types.call;
        this.row = types.row;
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
        // resolve the call in the branch's scope, refusing one naming another
        const type = objects.find((served) => entry.method.startsWith(`${served.name}.`));
        const field = type?.route.field;
        const named = field === undefined ? undefined : entry.input[field];
        if (named !== undefined && named !== scope) {
            throw new ServiceError("BAD_REQUEST", {
                message: `${entry.method} acts in another scope than its branch`,
            });
        }
        const input = field === undefined ? entry.input : { ...entry.input, [field]: scope };
        const resolved = Call.resolve(objects, { ...entry, input }, true);

        // refuse ephemeral objects and branch bookkeeping, whose writes no branch keeps
        if (resolved.object.storage === "ephemeral" || this.keeps(resolved.object)) {
            throw new ServiceError("BAD_REQUEST", {
                message: `branches keep no calls of ${resolved.object.name}`,
            });
        }

        return resolved;
    }

    /** Report whether an object type keeps branches themselves, whose calls stay on the main line. */
    keeps(object: ObjectType): boolean {
        return [this.object, this.call, this.row].some((type) => type.same(object));
    }

    /** Read the tables branches change, by SQL name. */
    tables(objects: readonly ObjectType[]): ReadonlyMap<string, Table> {
        return new Map(
            objects
                .filter((object) => object.storage === "durable" && !this.keeps(object))
                .map((object) => [(object.table as Table)[TABLE].sqlName, object.table as Table]),
        );
    }

    /** Rebuild open branches whose rows a main-line write may have changed. */
    controller(objects: readonly ObjectType[]): ObjectController {
        const branches = this.object.table as Table;

        return {
            pending: Condition.eq("state", "open"),
            watches: [...this.tables(objects).values()].map((table) => ({
                table,
                keys: async (row, database) => {
                    // select the open branches of the row's scope reaching its table
                    const open = (await database
                        .select()
                        .from(branches)
                        .where(
                            Condition.render(
                                Condition.all(
                                    Condition.eq("state", "open"),
                                    ...(typeof row.scope === "string"
                                        ? [Condition.eq("scope", row.scope)]
                                        : []),
                                ),
                                Condition.bind(branches),
                            ),
                        )) as unknown as { readonly id: string; readonly reach: string[] }[];

                    return open
                        .filter((entry) => entry.reach.includes(table[TABLE].sqlName))
                        .map((entry) => ({ id: entry.id }));
                },
            })),
            reconcile: async ({ rows, execute }) => {
                await execute("rebuild", rows);

                return undefined;
            },
        };
    }
}
