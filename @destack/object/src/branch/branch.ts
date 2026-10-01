import { keySubject, subjectKey, type Subject } from "@destack/access";
import { TABLE, type DatabaseConnection, type Table } from "@destack/db";
import { DatabaseError } from "@destack/db/error";
import { Condition, Order } from "@destack/db/query";
import type { LogPosition, Overlay, Snapshot } from "@destack/db/log";
import { Journal } from "@destack/service/database";
import type * as sync from "@destack/sync";
import { v7 } from "uuid";
import type { Call } from "../method/index.ts";
import type { Method } from "../method/method.ts";
import type { ObjectType } from "../object/index.ts";
import { type BranchChange, BranchRow } from "./row.ts";
import type { BranchType } from "./type.ts";

/** A call a branch keeps, with the principal who made it. */
export interface BranchCall extends sync.Call {
    /** The principal the call records as its caller. */
    readonly author: Subject;
}

/** The fields of a branch that its rows depend on. */
export interface BranchState {
    /** The log position the branch's rows were last built at. */
    readonly built: LogPosition;
    /** The SQL names of the tables the branch's calls read or write. */
    readonly reach: readonly string[];
}

/** One branch of a scope, opened on a database: its calls, and the rows they change. */
export class Branch {
    /** The scope type's branch types. */
    readonly types: BranchType;
    /** The database keeping the branch. */
    readonly database: DatabaseConnection;
    /** The branch's identifier. */
    readonly id: string;

    /** Open a branch on a database. */
    constructor(types: BranchType, database: DatabaseConnection, id: string) {
        this.types = types;
        this.database = database;
        this.id = id;
    }

    /** Read the branch's calls in order, each with its author. */
    async calls(): Promise<BranchCall[]> {
        const table = this.types.call.table as Table;
        const stored = (await this.database
            .select()
            .from(table)
            .where(this.#children(table))
            .orderBy(
                ...Order.render([{ column: "position", direction: "asc" }], table),
            )) as unknown as (sync.Call & { readonly author: string })[];

        return stored.map(({ method, input, release, author }) => ({
            method,
            input,
            release,
            author: keySubject(author),
        }));
    }

    /** Read the rows the branch changes. */
    async rows(): Promise<BranchRow[]> {
        const table = this.types.row.table as Table;
        const stored = (await this.database
            .select()
            .from(table)
            .where(this.#children(table))) as unknown as (BranchRow & {
            readonly row: BranchRow["row"] | undefined;
            readonly before: BranchRow["before"] | undefined;
        })[];

        return stored.map((entry) => ({
            table: entry.table,
            key: entry.key,
            row: entry.row ?? null,
            before: entry.before ?? null,
        }));
    }

    /** Read the changes the branch makes, one per object, with the written fields that differ. */
    async changes(objects: readonly ObjectType[]): Promise<BranchChange[]> {
        const byTable = new Map(
            objects.map((object) => [(object.table as Table)[TABLE].sqlName, object]),
        );

        return (await this.rows()).flatMap((entry) => {
            const object = byTable.get(entry.table);
            const change = object === undefined ? undefined : BranchRow.change(entry, object);

            return change === undefined ? [] : [change];
        });
    }

    /** Append calls after the branch's last one as an invoking call's caller's, returning them authored. */
    async append(call: Call, calls: readonly sync.Call[]): Promise<BranchCall[]> {
        // number the calls after the last one
        const last = (await this.calls()).length;
        const author = call.caller!;
        for (const [index, entry] of calls.entries()) {
            await call.invoke(this.types.call, "create", {
                id: `${this.types.call.identity}-${v7()}`,
                parentId: this.id,
                position: last + index + 1,
                method: entry.method,
                input: entry.input,
                release: entry.release,
                author: subjectKey(author),
            });
        }

        return calls.map((entry) => ({ ...entry, author }));
    }

    /** Replay calls over rows and store the rows they change. */
    async build(
        call: Call,
        state: BranchState,
        calls: readonly BranchCall[],
        over: readonly BranchRow[],
    ): Promise<{ readonly reach: string[]; readonly built: LogPosition }> {
        const replayed = await this.#replay(call, calls, over);
        await this.store(call, replayed.rows);

        return {
            reach: [...new Set([...state.reach, ...replayed.reach])].sort(),
            built: await this.database.log.position(),
        };
    }

    /** Replace the branch's stored rows with some rows, writing only those that changed. */
    async store(call: Call, replaced: readonly BranchRow[]): Promise<void> {
        // match the stored rows by table and key
        const named = (entry: { readonly table: string; readonly key: unknown }) =>
            JSON.stringify([entry.table, entry.key]);
        const table = this.types.row.table as Table;
        const stored = (await this.database
            .select()
            .from(table)
            .where(this.#children(table))) as unknown as (BranchRow & { readonly id: string })[];
        const byName = new Map(stored.map((entry) => [named(entry), entry]));

        // update or create each row, then delete the rows no longer changed
        for (const entry of replaced) {
            const existing = byName.get(named(entry));
            byName.delete(named(entry));
            if (existing !== undefined) {
                await call.invoke(this.types.row, "update", {
                    id: existing.id,
                    row: entry.row,
                    before: entry.before,
                });
            } else {
                await call.invoke(this.types.row, "create", {
                    id: `${this.types.row.identity}-${v7()}`,
                    parentId: this.id,
                    table: entry.table,
                    key: entry.key,
                    ...(entry.row === null ? {} : { row: entry.row }),
                    ...(entry.before === null ? {} : { before: entry.before }),
                });
            }
        }
        for (const entry of byName.values()) {
            await call.invoke(this.types.row, "delete", { id: entry.id });
        }
    }

    /** Report whether the main line changed a table the branch reaches since its rows were built. */
    async isStale(objects: readonly ObjectType[], state: BranchState): Promise<boolean> {
        // read the reached tables, and require the same log epoch
        const tables = [...this.types.tables(objects).values()].filter((table) =>
            state.reach.includes(table[TABLE].sqlName),
        );
        if (tables.length === 0) {
            return false;
        } else if ((await this.database.log.position()).epoch !== state.built.epoch) {
            return true;
        }

        // look for one change after the build, rebuilding when the log no longer has them
        try {
            const read = await this.database.log.read({
                tables,
                after: state.built.sequence,
                limit: 1,
            });

            return read.changes.length > 0;
        } catch (error) {
            if (error instanceof DatabaseError && error.code === "CHANGES_COMPACTED") {
                return true;
            }
            throw error;
        }
    }

    /** Read the rows the branch puts over the main line as a snapshot shows them, for reads. */
    async overlay(snapshot: Snapshot, objects: readonly ObjectType[]): Promise<Overlay> {
        const stored = (await snapshot.rows(
            this.types.row.table as Table,
            Condition.eq("parentId", this.id),
        )) as unknown as (BranchRow & {
            readonly row: BranchRow["row"] | undefined;
            readonly before: BranchRow["before"] | undefined;
        })[];
        const rows = stored.map((entry) => ({
            ...entry,
            row: entry.row ?? null,
            before: entry.before ?? null,
        }));

        return BranchRow.overlay(rows, this.types.tables(objects));
    }

    /** Write the branch's rows over the database's, as a client's prediction shows them. */
    async apply(objects: readonly ObjectType[]): Promise<void> {
        await BranchRow.write(this.database, this.types.tables(objects), await this.rows());
    }

    /** Replay calls with the system's authority as their authors over rows, returning the rows they leave changed. */
    async #replay(
        call: Call,
        calls: readonly BranchCall[],
        over: readonly BranchRow[],
    ): Promise<{ readonly rows: BranchRow[]; readonly reach: string[] }> {
        // replay in a savepoint rolled back after reading what it wrote
        const tables = this.types.tables(call.objects);
        let replayed: { rows: BranchRow[]; reach: string[] } | undefined;
        await this.database.transaction(async (savepoint) => {
            // write the rows, then each call apart, skipping refused calls and external work
            const before = (await savepoint.log.written([...tables.values()])).length;
            await BranchRow.write(savepoint, tables, over);
            const reach = new Set(over.map((row) => row.table));
            for (const entry of calls) {
                const resolved = this.types.resolve(call.objects, entry, call.scope);
                const method = (resolved.object.methods as Readonly<Record<string, Method>>)[
                    resolved.name
                ]!;
                reach.add((resolved.object.table as Table)[TABLE].sqlName);
                if (method.prepare === undefined) {
                    await Branch.#attempt(savepoint, () =>
                        call.run!(resolved.object, resolved.name, resolved.input, {
                            as: entry.author,
                            authority: "system",
                        }),
                    );
                }
            }

            // keep the rows the replay leaves changed
            const rows = BranchRow.capture(
                (await savepoint.log.written([...tables.values()])).slice(before),
            );
            replayed = {
                rows,
                reach: [...new Set([...reach, ...rows.map((row) => row.table)])].sort(),
            };
            savepoint.rollback();
        });

        return replayed!;
    }

    /** Match the rows nested in the branch. */
    #children(table: Table) {
        return Condition.render(Condition.eq("parentId", this.id), Condition.bind(table));
    }

    /** Run a replayed call in its own savepoint, skipping a refusal the server would answer for good. */
    static async #attempt(
        database: DatabaseConnection,
        run: () => Promise<unknown>,
    ): Promise<void> {
        try {
            await database.transaction(run);
        } catch (error) {
            if (Journal.failure(error) === undefined) {
                throw error;
            }
        }
    }
}
