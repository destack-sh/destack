import {
    TABLE,
    type DatabaseConnection,
    DatabaseError,
    type LogPosition,
    type Overlay,
    type Snapshot,
} from "@destack/db";
import { ObjectReference } from "@destack/sync";
import { PackageId } from "@destack/package";
import { schema, Uuid } from "@destack/schema";
import * as sync from "@destack/sync";
import { MethodContext } from "../method/index.ts";

import type { ObjectType } from "../object/index.ts";
import { BranchEntry } from "./entry.ts";
import { type BranchChange, BranchWrite } from "./write.ts";
import type { BranchType } from "./type.ts";

/** The columns of a stored branch entry: the call and its author's reference. */
const STORED_ENTRY = sync.Call.extend({
    authorPackageId: PackageId,
    authorType: schema.string(),
    authorScope: schema.string(),
    authorId: schema.string(),
}).loose();

/** The fields of a branch that its rows depend on. */
export interface BranchState {
    /** The log position the branch's rows were last built at. */
    readonly built: LogPosition;
    /** The SQL names of the tables the branch's calls read or write. */
    readonly tables: readonly string[];
}

/** One branch of a scope, opened on a database: its calls, and the rows they change. */
export class Branch {
    /** The scope type's branch types. */
    readonly types: BranchType;
    /** The reads of the branch's rows: a call's, or a snapshot of its database. */
    readonly reader: Pick<Snapshot, "read">;
    /** The branch's identifier. */
    readonly id: string;

    /** Open a branch over the reads of its rows. */
    constructor(types: BranchType, reader: Pick<Snapshot, "read">, id: string) {
        this.types = types;
        this.reader = reader;
        this.id = id;
    }

    /** Count the branch's entries. */
    async count(): Promise<number> {
        return (
            await this.reader
                .read(this.types.entry.table)
                .findMany({ where: { parentId: this.id } })
        ).length;
    }

    /** Read the branch's entries in order, each call with its author. */
    async entries(): Promise<BranchEntry[]> {
        const stored = await this.reader
            .read(this.types.entry.table)
            .findMany({ where: { parentId: this.id } });

        return stored
            .map((entry) => STORED_ENTRY.parse(entry))
            .toSorted((left, right) => Number(left["position"]) - Number(right["position"]))
            .map((entry) => ({
                type: entry.type,
                method: entry.method,
                input: entry.input,
                release: entry.release,
                author: ObjectReference.at("author", entry),
            }));
    }

    /** Read the rows the branch changes. */
    async rows(): Promise<BranchWrite[]> {
        const stored = await this.reader
            .read(this.types.change.table)
            .findMany({ where: { parentId: this.id } });

        return stored.map((entry) => {
            const { id: _id, ...row } = BranchWrite.read(entry);

            return row;
        });
    }

    /** Read the changes the branch makes, one per object, with the written fields that differ. */
    async changes(objects: readonly ObjectType[]): Promise<BranchChange[]> {
        const byTable = new Map(objects.map((object) => [object.table[TABLE].sqlName, object]));

        return (await this.rows()).flatMap((entry) => {
            const object = byTable.get(entry.table);
            const change = object === undefined ? undefined : BranchWrite.change(entry, object);

            return change === undefined ? [] : [change];
        });
    }

    /** Append calls after the branch's last one as the invoking caller's, returning them. */
    async append(context: MethodContext, calls: readonly sync.Call[]): Promise<BranchEntry[]> {
        // number the calls after the last one
        const last = (await this.entries()).length;
        const author = context.caller.subject;
        for (const [index, entry] of calls.entries()) {
            await context.mutate(context.caller, [
                this.types.entry.call("create", {
                    id: `${this.types.entry.identity}-${Uuid.v7(context.environment)}`,
                    parentId: this.id,
                    position: last + index + 1,
                    type: entry.type,
                    method: entry.method,
                    input: entry.input,
                    release: entry.release,
                    ...ObjectReference.columns("author", author),
                }),
            ]);
        }

        return calls.map((entry) => ({ ...entry, author }));
    }

    /** Replay calls over rows and store the rows they change. */
    async build(
        context: MethodContext,
        state: BranchState,
        calls: readonly BranchEntry[],
        over: readonly BranchWrite[],
    ): Promise<{ readonly tables: string[]; readonly built: LogPosition }> {
        const replayed = await this.#replay(context, calls, over);
        await this.store(context, replayed.rows);

        return {
            tables: [...new Set([...state.tables, ...replayed.tables])].toSorted(),
            built: context.base,
        };
    }

    /** Replace the branch's stored rows with some rows, writing only those that changed. */
    async store(context: MethodContext, replaced: readonly BranchWrite[]): Promise<void> {
        // match the stored rows by table and key
        const stored = await this.reader
            .read(this.types.change.table)
            .findMany({ where: { parentId: this.id } });
        const byName = new Map(
            stored
                .map((entry) => BranchWrite.read(entry))
                .map((entry) => [Branch.#named(entry), entry]),
        );

        // update or create each row and delete the rows no longer changed
        for (const entry of replaced) {
            const existing = byName.get(Branch.#named(entry));
            byName.delete(Branch.#named(entry));
            if (existing !== undefined) {
                await context.mutate(context.caller, [
                    this.types.change.call("update", {
                        id: existing.id,
                        after: entry.after,
                        before: entry.before,
                    }),
                ]);
            } else {
                await context.mutate(context.caller, [
                    this.types.change.call("create", {
                        id: `${this.types.change.identity}-${Uuid.v7(context.environment)}`,
                        parentId: this.id,
                        table: entry.table,
                        key: entry.key,
                        ...(entry.after === null ? {} : { after: entry.after }),
                        ...(entry.before === null ? {} : { before: entry.before }),
                    }),
                ]);
            }
        }
        for (const entry of byName.values()) {
            await context.mutate(context.caller, [
                this.types.change.call("delete", { id: entry.id }),
            ]);
        }
    }

    /** Report whether the main line changed a branch table since the branch's rows were built. */
    async isStale(
        database: DatabaseConnection,
        objects: readonly ObjectType[],
        state: BranchState,
    ): Promise<boolean> {
        // read the touched tables, and require the same log epoch
        const tables = [...this.types.tables(objects).values()].filter((table) =>
            state.tables.includes(table[TABLE].sqlName),
        );
        if (tables.length === 0) {
            return false;
        } else if ((await database.log.position()).epoch !== state.built.epoch) {
            return true;
        }

        // look for one change after the build, rebuilding when the log no longer has them
        try {
            const read = await database.log.read({
                tables,
                after: state.built.sequence,
                limit: 1,
            });

            return read.changes.length > 0;
        } catch (error) {
            if (DatabaseError.is(error, "CHANGES_COMPACTED")) {
                return true;
            }
            throw error;
        }
    }

    /** Read the rows the branch puts over the main line as a snapshot shows them, for reads. */
    async overlay(snapshot: Snapshot, objects: readonly ObjectType[]): Promise<Overlay> {
        const stored = await snapshot
            .read(this.types.change.table)
            .findMany({ where: { parentId: this.id } });
        const rows = stored.map((entry) => BranchWrite.read(entry));

        return BranchWrite.overlay(rows, this.types.tables(objects));
    }

    /** Write the branch's rows over the database's, as a client's prediction shows them. */
    async apply(database: DatabaseConnection, objects: readonly ObjectType[]): Promise<void> {
        await BranchWrite.write(database, this.types.tables(objects), await this.rows());
    }

    /** Replay calls as their authors over rows, returning the rows they leave changed. */
    async #replay(
        context: MethodContext,
        calls: readonly BranchEntry[],
        over: readonly BranchWrite[],
    ): Promise<{ readonly rows: BranchWrite[]; readonly tables: string[] }> {
        // resolve each call in the branch's scope, leaving out calls doing external work
        const touched = new Set(over.map((row) => row.table));
        const replayed = calls.flatMap((entry) => {
            // resolve the call and note the table it touches
            const resolved = this.types.resolve(context.objects, entry, context.scope);
            const method = resolved.object.method(resolved.name);
            touched.add(resolved.object.table[TABLE].sqlName);

            // record the call as its author's unless it does external work
            return method.prepare === undefined
                ? [
                      {
                          ...resolved.object.call(resolved.name, resolved.input),
                          author: entry.author,
                      },
                  ]
                : [];
        });

        // rehearse them over the rows, taking the rows of the branch's tables they leave changed
        const changed = await context.rehearse(over, replayed);
        const names = new Set(this.types.tables(context.objects).keys());
        const rows = changed.filter((row) => names.has(row.table));

        return {
            rows,
            tables: [...new Set([...touched, ...rows.map((row) => row.table)])].toSorted(),
        };
    }

    /** Build the key of a branch row from its table and key. */
    static #named(entry: { readonly table: string; readonly key: unknown }): string {
        return JSON.stringify([entry.table, entry.key]);
    }
}
