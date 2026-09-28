import { eq, sql, type DatabaseConnection } from "@destack/db";
import type { Snapshot } from "@destack/db/log";
import type { PackageId } from "@destack/package";
import { GLOBAL_SCOPE } from "../context/context.ts";
import { AccessError } from "../error/index.ts";
import type { ObjectReference } from "../policy/policy.ts";
import { accessScope } from "./table.ts";

/** The prefix of every scope's fence lock key. */
const FENCE_LOCK = "destack-fence";

/** A scope's chain and transfer fence, read from and written to its own row. */
export const Scope = { chain, object, fence, guard, unfence };

/** One scope of a chain. */
export interface ScopeLink {
    /** The scope's object. */
    readonly object: ObjectReference;
    /** The containing scope. */
    readonly parent: string;
    /** Whether the scope is suspended. */
    readonly isSuspended: boolean;
    /** The holder a transfer moves the scope to, while fenced. */
    readonly movedTo: string | undefined;
}

/** Read a scope and the scopes enclosing it, nearest first. */
async function chain(snapshot: Snapshot, scope: string): Promise<ScopeLink[]> {
    // read the scope's row, then its ancestors' rows by key
    const rows = new Map<string, ScopeRow>();
    for (let wanted = [scope]; wanted.length > 0;) {
        const read = (await snapshot.select(
            accessScope,
            ["scope"],
            wanted.map((id) => [id]),
        )) as ScopeRow[];
        for (const row of read) {
            rows.set(row.scope, row);
        }
        wanted = [...new Set(read.flatMap((row) => row.ancestors))].filter(
            (id) => !rows.has(id) && !wanted.includes(id),
        );
    }

    // order them from the scope up
    const links: ScopeLink[] = [];
    for (let current = rows.get(scope); current !== undefined;) {
        if (links.some((link) => link.object.id === current!.scope)) {
            throw new AccessError("INVALID_CONTEXT", `cyclic scope: ${current.scope}`);
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
        current = current.parent === GLOBAL_SCOPE ? undefined : rows.get(current.parent);
    }

    return links;
}

/** Read a scope's own object, refusing a scope the database does not know. */
async function object(snapshot: Snapshot, id: string): Promise<ObjectReference> {
    const [link] = await chain(snapshot, id);
    if (link === undefined) {
        throw new AccessError("NOT_FOUND", `unknown scope: ${id}`);
    }

    return link.object;
}

/** Send a scope's writes to another holder once the writes guarding it commit; the scopes below it follow. */
async function fence(
    database: DatabaseConnection,
    scope: string,
    holder: string,
    now: number,
): Promise<void> {
    await database.transaction(async (transaction) => {
        // wait for the writes holding the guard
        await lock(transaction, scope, "exclusive");

        // mark the scope moved
        const [fenced] = await transaction
            .update(accessScope)
            .set({ fencedAt: now, movedTo: holder })
            .where(eq(accessScope.scope, scope))
            .returning({ scope: accessScope.scope });
        if (fenced === undefined) {
            throw new AccessError("NOT_FOUND", `unknown scope: ${scope}`);
        }
    });
}

/** Keep a write's scopes unfenced until it commits. */
async function guard(database: DatabaseConnection, scopes: readonly string[]): Promise<void> {
    for (const scope of scopes) {
        await lock(database, scope, "shared");
    }
}

/** Take a scope's fence lock for the transaction; SQLite serializes writes already. */
async function lock(
    database: DatabaseConnection,
    scope: string,
    mode: "shared" | "exclusive",
): Promise<void> {
    if (database.dialect === "postgresql") {
        const key = sql`hashtextextended(${`${FENCE_LOCK}:${scope}`}, 0)`;
        await database.execute(
            mode === "shared"
                ? sql`SELECT pg_advisory_xact_lock_shared(${key})`
                : sql`SELECT pg_advisory_xact_lock(${key})`,
        );
    }
}

/** Lift a scope's fence on the database now holding it. */
async function unfence(database: DatabaseConnection, scope: string): Promise<void> {
    await database
        .update(accessScope)
        .set({ fencedAt: null, movedTo: null })
        .where(eq(accessScope.scope, scope));
}

/** A scope row as a snapshot reads it. */
type ScopeRow = {
    /** The scope. */
    readonly scope: string;
    /** The containing scope. */
    readonly parent: string;
    /** The containing scopes the row lists, nearest first. */
    readonly ancestors: readonly string[];
    /** The package declaring the scope object's type. */
    readonly packageId: PackageId;
    /** The scope object's type. */
    readonly type: string;
    /** When the scope was suspended. */
    readonly suspendedAt: number | string | null;
    /** The holder a fenced scope's database moves to. */
    readonly movedTo: string | null;
};
