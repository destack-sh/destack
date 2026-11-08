import type { Row, Snapshot, Table } from "@destack/db";
import type { PermissionReference } from "../declare/policy.ts";
import type { Access } from "../authorizer/access.ts";
import type { Authorizer } from "../authorizer/authorizer.ts";

/** The most rows a listing reads: every row of a test's tables. */
const LISTED_ROWS = 10_000;

/** Read the rows of a table a caller may list under a permission: candidates, each decided. */
export async function listRows(
    authorizer: Authorizer,
    snapshot: Snapshot,
    permission: PermissionReference,
    access: Access,
    table?: Table,
): Promise<Row[]> {
    // read the mapped table, or one sharing its rows, through the caller's visibility
    const mapping = authorizer.requireMapping(permission);
    const source = table ?? mapping.table;

    return snapshot.ordered(source, {
        where: {},
        orderBy: { [mapping.id]: "asc" },
        limit: LISTED_ROWS,
        visible: await authorizer.visibility(snapshot, permission, access, source),
    });
}

/** Report whether a caller may list one row of a table under a permission. */
export async function isListed(
    authorizer: Authorizer,
    snapshot: Snapshot,
    permission: PermissionReference,
    access: Access,
    table: Table,
    id: string,
): Promise<boolean> {
    const mapping = authorizer.requireMapping(permission);
    const rows = await listRows(authorizer, snapshot, permission, access, table);

    return rows.some((row) => row[mapping.id] === id);
}
