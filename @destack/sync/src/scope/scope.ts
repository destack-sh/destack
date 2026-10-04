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

/** The universe's link, the last of every chain. */
const UNIVERSE_LINK: ScopeLink = {
    object: universe,
    parent: UNIVERSE_ID,
    isSuspended: false,
    movedTo: undefined,
};

/** A scope's chain and transfer fence, read from and written to its own row. */
export const Scope = { table: scopeTable, universe, chain, chains, object, fence, guard, unfence };

/** One scope of a chain. */
export interface ScopeLink {
    /** The scope's object. */
    readonly object: ObjectReference;
    /** The containing scope. */
    readonly parent: string;
    /** Whether the scope is suspended. */
    readonly isSuspended: boolean;
    /** The cell a transfer moves the scope to, while fenced. */
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
    /** The cell a transfer moves the scope to. */
    moved_to: schema.string().nullable(),
});

/** A scope row's parent and fence state as a write locks it. */
type LockedScope = schema.Infer<typeof LockedScope>;

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
    // read the scopes' rows, then their ancestors' rows by key
    const rows = new Map<string, RowImage<typeof scopeTable>>();
    for (let wanted = [...new Set(scopes)]; wanted.length > 0;) {
        const read = await snapshot.select(
            scopeTable,
            ["scope"],
            wanted.map((id) => [id]),
        );
        for (const row of read) {
            rows.set(row.scope, row);
        }
        wanted = [...new Set(read.flatMap((row) => row.ancestors))].filter(
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
    for (let current = rows.get(scope); current !== undefined;) {
        const id = current.scope;
        if (links.some((link) => link.object.id === id)) {
            throw new SyncError("INVALID_SCOPE", `cyclic scope: ${id}`);
        }
        links.push({
            object: {
                packageId: current.packageId,
                type: current.type,
                scope: current.parent,
                id: current.scope,
            },
            parent: current.parent,
            isSuspended: current.suspendedAt !== null,
            movedTo: current.movedTo ?? undefined,
        });
        current = current.parent === UNIVERSE_ID ? undefined : rows.get(current.parent);
    }

    // end a chain reaching the top at the universe
    const isRooted = scope === UNIVERSE_ID || links.at(-1)?.parent === UNIVERSE_ID;

    return isRooted ? [...links, UNIVERSE_LINK] : links;
}

/** Read a scope's own object and refuse an unknown scope. */
async function object(snapshot: Snapshot, id: string): Promise<ObjectReference> {
    const [link] = await chain(snapshot, id);
    if (link === undefined) {
        throw new SyncError("NOT_FOUND", `unknown scope: ${id}`);
    }

    return link.object;
}

/** Send a scope and the scopes below it to another cell once the writes guarding it commit. */
async function fence(
    database: DatabaseConnection,
    scope: string,
    cell: string,
    now: number,
): Promise<void> {
    // mark the scope moved, waiting on its row lock for the writes guarding it
    const [fenced] = await database
        .update(scopeTable)
        .set({ fencedAt: now, movedTo: cell })
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
        sql`SELECT ${scopeTable.scope} AS scope, ${scopeTable.parent} AS parent, ${scopeTable.suspendedAt} AS suspended_at, ${scopeTable.movedTo} AS moved_to FROM ${scopeTable} WHERE ${inArray(scopeTable.scope, scopes)} ORDER BY ${scopeTable.scope} FOR SHARE`,
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
