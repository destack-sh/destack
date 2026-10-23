import type { RowImage, Select, Transaction, Snapshot } from "@destack/db";
import { found } from "@destack/schema";
import { SyncError } from "../error/index.ts";
import type { ObjectReference } from "./reference.ts";
import { scopeTable } from "./table.ts";

/** The package declaring sync's type: the universe. */
export const SYNC_PACKAGE = import.meta.destack.package;

/** The identifier of the universe and its scope. */
const UNIVERSE_ID = "universe";

/** The universe: the deployment, and the root scope every other scope is inside of. */
const universe: ObjectReference & {
    readonly scope: typeof UNIVERSE_ID;
    readonly id: typeof UNIVERSE_ID;
} = {
    packageId: SYNC_PACKAGE.id,
    type: "universe",
    scope: UNIVERSE_ID,
    id: UNIVERSE_ID,
};

/** The universe's scope row, which every chain ends at and no database stores. */
const UNIVERSE_ROW: RowImage<typeof scopeTable> = {
    scope: UNIVERSE_ID,
    parent: UNIVERSE_ID,
    ancestors: [],
    partition: UNIVERSE_ID,
    packageId: universe.packageId,
    type: universe.type,
    suspendedAt: null,
    cappedAt: null,
    fencedAt: null,
    movedTo: null,
};

/** A scope's chain, storage cap and transfer fence, read from and written to its row. */
export const Scope = {
    table: scopeTable,
    universe,

    /** Read some scopes' rows by identifier, and the universe's when asked, stored nowhere. */
    async read(
        reader: Pick<Snapshot, "read">,
        ids: readonly string[],
    ): Promise<RowImage<typeof scopeTable>[]> {
        // read the stored rows, and the universe's when asked
        const rows =
            ids.length === 0
                ? []
                : await reader.read(scopeTable).findMany({ where: { scope: { in: [...ids] } } });

        return ids.includes(UNIVERSE_ID) ? [...rows, UNIVERSE_ROW] : rows;
    },

    /** Report whether a scope is or lies within another, the universe enclosing every scope. */
    within(row: Pick<RowImage<typeof scopeTable>, "scope" | "ancestors">, outer: string): boolean {
        return outer === row.scope || outer === UNIVERSE_ID || row.ancestors.includes(outer);
    },

    /** Read a scope and every scope below it, nearest first and by key, one level per read. */
    async subtree(reader: Pick<Snapshot, "read">, scope: string): Promise<string[]> {
        // start at the scope and walk down a level at a time
        const subtree = new Set([scope]);
        let level = [scope];
        while (level.length > 0) {
            // read the scopes whose parent is on the last level
            const rows = await reader
                .read(scopeTable)
                .findMany({ where: { parent: { in: level } }, orderBy: { scope: "asc" } });
            level = rows.map((row) => row.scope).filter((child) => !subtree.has(child));
            for (const child of level) {
                subtree.add(child);
            }
        }

        return [...subtree];
    },

    /** Find the fenced scope in a chain, absent when none is fenced. */
    fenced(links: readonly ScopeLink[]): ScopeLink | undefined {
        return links.find((link) => link.movedTo !== undefined);
    },

    /** Read a scope and the scopes enclosing it, nearest first. */
    async chain(reader: Pick<Snapshot, "read">, scope: string): Promise<ScopeLink[]> {
        const chained = await Scope.chains(reader, [scope]);

        return found(chained, scope);
    },

    /** Read the chains of some scopes, nearest first, reading each ancestor level once for all. */
    async chains(
        reader: Pick<Snapshot, "read">,
        scopes: readonly string[],
    ): Promise<Map<string, ScopeLink[]>> {
        // read the scopes' rows and their ancestors' rows by key
        const rows = new Map<string, RowImage<typeof scopeTable>>();
        for (let wanted = [...new Set(scopes)]; wanted.length > 0;) {
            const stored = await Scope.read(reader, wanted);
            for (const row of stored) {
                rows.set(row.scope, row);
            }
            wanted = [...new Set(stored.flatMap((row) => row.ancestors))].filter(
                (id) => !rows.has(id) && !wanted.includes(id),
            );
        }

        return new Map(scopes.map((scope) => [scope, Scope.linked(rows, scope)]));
    },

    /** Link a scope's chain from read scope rows, from the scope up. */
    linked(rows: ReadonlyMap<string, RowImage<typeof scopeTable>>, scope: string): ScopeLink[] {
        // order the rows from the scope up
        const links: ScopeLink[] = [];
        for (
            let current = rows.get(scope);
            current !== undefined && current.scope !== UNIVERSE_ID;
        ) {
            const id = current.scope;
            if (links.some((link) => link.object.id === id)) {
                throw SyncError.of("INVALID_SCOPE", `cyclic scope: ${id}`);
            }
            links.push(Scope.link(current));
            current = current.parent === UNIVERSE_ID ? undefined : rows.get(current.parent);
        }

        // end a chain at the universe once it arrives at the top
        const isRooted = scope === UNIVERSE_ID || links.at(-1)?.parent === UNIVERSE_ID;

        return isRooted ? [...links, Scope.link(UNIVERSE_ROW)] : links;
    },

    /** Link a scope's object to its parent from its row. */
    link(row: RowImage<typeof scopeTable>): ScopeLink {
        return {
            object: { packageId: row.packageId, type: row.type, scope: row.parent, id: row.scope },
            parent: row.parent,
            partition: row.partition,
            isSuspended: row.suspendedAt !== null,
            isCapped: row.cappedAt !== null,
            movedTo: row.movedTo ?? undefined,
        };
    },

    /** Read a scope's object and refuse an unknown scope. */
    async object(reader: Pick<Snapshot, "read">, id: string): Promise<ObjectReference> {
        const [link] = await Scope.chain(reader, id);
        if (link === undefined) {
            throw SyncError.of("NOT_FOUND", `unknown scope: ${id}`);
        }

        return link.object;
    },

    /** Cap a scope's storage from a time, refusing writes but deletes, or lift it with null. */
    async cap(transaction: Transaction, scope: string, cappedAt: number | null): Promise<void> {
        await Scope.change(transaction, scope, { cappedAt });
    },

    /** Send a scope and its subtree to another machine, failing commits that read its chain. */
    async fence(
        transaction: Transaction,
        scope: string,
        machine: string,
        now: number,
    ): Promise<void> {
        await Scope.change(transaction, scope, { fencedAt: now, movedTo: machine });
    },

    /** Lift a scope's fence on the database that now stores it. */
    async unfence(transaction: Transaction, scope: string): Promise<void> {
        await Scope.change(transaction, scope, { fencedAt: null, movedTo: null });
    },

    /** Write some columns of a scope's row in a transaction, refusing an unknown scope. */
    async change(
        transaction: Transaction,
        scope: string,
        changes: Partial<Select<typeof scopeTable>>,
    ): Promise<void> {
        const row = await transaction.read(scopeTable).findFirst({ where: { scope } });
        if (row === undefined) {
            throw SyncError.of("NOT_FOUND", `unknown scope: ${scope}`);
        }
        await transaction.write({ table: scopeTable, before: row, after: { ...row, ...changes } });
    },
};

/** One scope of a chain. */
export interface ScopeLink {
    /** The scope's object. */
    readonly object: ObjectReference;
    /** The containing scope. */
    readonly parent: string;
    /** The partition the scope's objects live in. */
    readonly partition: string;
    /** Whether the scope is suspended. */
    readonly isSuspended: boolean;
    /** Whether the scope's storage is capped. */
    readonly isCapped: boolean;
    /** The machine a transfer moves the scope to, while fenced. */
    readonly movedTo: string | undefined;
}
