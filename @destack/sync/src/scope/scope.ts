import { eq, inArray, sql, Snapshot, type DatabaseConnection, type RowImage } from "@destack/db";
import { found, schema } from "@destack/schema";
import type {} from "@destack/package/import-meta";
import { SyncError } from "../error/index.ts";
import type { ObjectReference } from "./reference.ts";
import { scopeTable } from "./table.ts";

/** The package declaring sync's own type: the universe. */
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
    packageId: universe.packageId,
    type: universe.type,
    suspendedAt: null,
    cappedAt: null,
    fencedAt: null,
    movedTo: null,
};

/** A scope's chain, storage cap and transfer fence, read from and written to its own row. */
export const Scope = {
    table: scopeTable,
    universe,
    read,
    chain,
    chains,
    object,
    cap,
    fence,
    guard,
    unfence,
};

/** One scope of a chain. */
export interface ScopeLink {
    /** The scope's object. */
    readonly object: ObjectReference;
    /** The containing scope. */
    readonly parent: string;
    /** Whether the scope is suspended. */
    readonly isSuspended: boolean;
    /** Whether the scope's storage is capped. */
    readonly isCapped: boolean;
    /** The machine a transfer moves the scope to, while fenced. */
    readonly movedTo: string | undefined;
}

/** A scope row's parent and fence state as a write locks it. */
const LockedScope = schema.object({
    /** The scope. */
    scope: schema.string(),
    /** The containing scope. */
    parent: schema.string(),
    /** When the scope was suspended. */
    suspended_at: schema.unknown(),
    /** When the scope's storage was capped. */
    capped_at: schema.unknown(),
    /** The machine a transfer moves the scope to. */
    moved_to: schema.string().nullable(),
});

/** A scope row's parent and fence state as a write locks it. */
type LockedScope = schema.Infer<typeof LockedScope>;

/** Read the rows of some scopes by identifier, the universe's among them when asked, which no database stores. */
async function read(
    snapshot: Snapshot,
    ids: readonly string[],
): Promise<RowImage<typeof scopeTable>[]> {
    // read the stored rows, and the universe's when asked
    const rows = await snapshot.select(
        scopeTable,
        ["scope"],
        ids.map((id) => [id]),
    );

    return ids.includes(UNIVERSE_ID) ? [...rows, UNIVERSE_ROW] : rows;
}

/** Read a scope and the scopes enclosing it, nearest first. */
async function chain(snapshot: Snapshot, scope: string): Promise<ScopeLink[]> {
    const chained = await chains(snapshot, [scope]);

    return found(chained, scope);
}

/** Read the chains of some scopes, nearest first, by scope, reading each level of ancestors once for all of them. */
async function chains(
    snapshot: Snapshot,
    scopes: readonly string[],
): Promise<Map<string, ScopeLink[]>> {
    // read the scopes' rows and their ancestors' rows by key
    const rows = new Map<string, RowImage<typeof scopeTable>>();
    for (let wanted = [...new Set(scopes)]; wanted.length > 0;) {
        const stored = await read(snapshot, wanted);
        for (const row of stored) {
            rows.set(row.scope, row);
        }
        wanted = [...new Set(stored.flatMap((row) => row.ancestors))].filter(
            (id) => !rows.has(id) && !wanted.includes(id),
        );
    }

    return new Map(scopes.map((scope) => [scope, linked(rows, scope)]));
}

/** Link a scope's chain from read scope rows, from the scope up. */
function linked(
    rows: ReadonlyMap<string, RowImage<typeof scopeTable>>,
    scope: string,
): ScopeLink[] {
    // order the rows from the scope up
    const links: ScopeLink[] = [];
    for (let current = rows.get(scope); current !== undefined && current.scope !== UNIVERSE_ID;) {
        const id = current.scope;
        if (links.some((link) => link.object.id === id)) {
            throw new SyncError("INVALID_SCOPE", `cyclic scope: ${id}`);
        }
        links.push(linkOf(current));
        current = current.parent === UNIVERSE_ID ? undefined : rows.get(current.parent);
    }

    // end a chain reaching the top at the universe
    const isRooted = scope === UNIVERSE_ID || links.at(-1)?.parent === UNIVERSE_ID;

    return isRooted ? [...links, linkOf(UNIVERSE_ROW)] : links;
}

/** Link a scope's object to its parent from its row. */
function linkOf(row: RowImage<typeof scopeTable>): ScopeLink {
    return {
        object: { packageId: row.packageId, type: row.type, scope: row.parent, id: row.scope },
        parent: row.parent,
        isSuspended: row.suspendedAt !== null,
        isCapped: row.cappedAt !== null,
        movedTo: row.movedTo ?? undefined,
    };
}

/** Read a scope's own object and refuse an unknown scope. */
async function object(snapshot: Snapshot, id: string): Promise<ObjectReference> {
    const [link] = await chain(snapshot, id);
    if (link === undefined) {
        throw new SyncError("NOT_FOUND", `unknown scope: ${id}`);
    }

    return link.object;
}

/** Cap a scope's storage from a time, refusing writes within it but deletes, or lift the cap with null. */
async function cap(
    database: DatabaseConnection,
    scope: string,
    cappedAt: number | null,
): Promise<void> {
    const [capped] = await database
        .update(scopeTable)
        .set({ cappedAt })
        .where(eq(scopeTable.scope, scope))
        .returning({ scope: scopeTable.scope });
    if (capped === undefined) {
        throw new SyncError("NOT_FOUND", `unknown scope: ${scope}`);
    }
}

/** Send a scope and the scopes below it to another machine once the writes guarding it commit. */
async function fence(
    database: DatabaseConnection,
    scope: string,
    machine: string,
    now: number,
): Promise<void> {
    // mark the scope moved, waiting on its row lock for the writes guarding it
    const [fenced] = await database
        .update(scopeTable)
        .set({ fencedAt: now, movedTo: machine })
        .where(eq(scopeTable.scope, scope))
        .returning({ scope: scopeTable.scope });
    if (fenced === undefined) {
        throw new SyncError("NOT_FOUND", `unknown scope: ${scope}`);
    }
}

/**
 * Lock a write's scope chain against fences until it commits, and return the chain as locked.
 *
 * A write whose snapshot predates a fence committed meanwhile fails as a concurrent update on PostgreSQL.
 */
async function guard(database: DatabaseConnection, scope: string): Promise<ScopeLink[]> {
    for (;;) {
        // take the chain as read on SQLite's serialized writes
        const links = await chain(Snapshot.live(database), scope);
        if (database.dialect !== "postgresql") {
            return links;
        }

        // lock the chain's rows, and read the chain again once they changed since
        const locked = await lock(
            database,
            links.map((link) => link.object.id).filter((id) => id !== UNIVERSE_ID),
        );
        const isCurrent = links.every((link) => {
            const row = locked.get(link.object.id);

            return (
                link.object.id === UNIVERSE_ID ||
                (row !== undefined &&
                    row.parent === link.parent &&
                    (row.suspended_at !== null) === link.isSuspended &&
                    (row.capped_at !== null) === link.isCapped &&
                    (row.moved_to ?? undefined) === link.movedTo)
            );
        });
        if (isCurrent) {
            return links;
        }
    }
}

/** Share-lock scope rows for the transaction, returning their fence state as locked by scope. */
async function lock(
    database: DatabaseConnection,
    scopes: readonly string[],
): Promise<Map<string, LockedScope>> {
    const rows = await database.execute(
        sql`SELECT ${scopeTable.scope} AS scope, ${scopeTable.parent} AS parent, ${scopeTable.suspendedAt} AS suspended_at, ${scopeTable.cappedAt} AS capped_at, ${scopeTable.movedTo} AS moved_to FROM ${scopeTable} WHERE ${inArray(scopeTable.scope, scopes)} ORDER BY ${scopeTable.scope} FOR SHARE`,
        LockedScope,
    );

    return new Map(rows.map((row) => [row.scope, row]));
}

/** Lift a scope's fence on the database now keeping it. */
async function unfence(database: DatabaseConnection, scope: string): Promise<void> {
    await database
        .update(scopeTable)
        .set({ fencedAt: null, movedTo: null })
        .where(eq(scopeTable.scope, scope));
}
