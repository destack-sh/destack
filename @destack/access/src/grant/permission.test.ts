import { expect, onTestFinished, test } from "@destack/test";
import {
    and,
    defineTable,
    eq,
    text,
    type DatabaseConnection,
    type Table,
    Snapshot,
} from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { Identifier } from "@destack/schema";
import {
    accessTables,
    Authorizer,
    permission,
    Policy,
    principal,
    relation,
    through,
    type AccessContext,
    GrantPermission,
    type PermissionReference,
    type TableMapping,
} from "../index.ts";
import { module1 } from "../test/fixture.ts";

/** Pages with text derived from reading them. */
const page = new Policy(module1.package, {
    name: "page",
    relations: { owner: { subjects: [principal.user] } },
    permissions: { read: relation("owner"), edit: relation("owner"), text: permission("read") },
});

/** Sheets with text derived from reading them. */
const sheet = new Policy(module1.package, {
    name: "sheet",
    relations: { owner: { subjects: [principal.user] } },
    permissions: { read: relation("owner"), text: permission("read") },
});

/** Pieces of a page's or a sheet's text, read through their parent's text. */
const piece = new Policy(module1.package, {
    name: "piece",
    relations: { parent: { subjects: [page, sheet], grantedBy: null } },
    permissions: { read: through("parent", "text") },
});

/** Pages and sheets, told apart by the table each lives in. */
const pageTable = defineTable("example_page", {
    id: text("id").primaryKey(),
    scope: text("scope").notNull(),
    owner: text("owner").notNull(),
});

/** Sheets. */
const sheetTable = defineTable("example_sheet", {
    id: text("id").primaryKey(),
    scope: text("scope").notNull(),
    owner: text("owner").notNull(),
});

/** Pieces that keep their parent's type in columns. */
const pieceTable = defineTable("example_piece", {
    id: text("id").primaryKey(),
    scope: text("scope").notNull(),
    parentId: text("parent_id").notNull(),
    parentPackageId: text("parent_package_id").notNull(),
    parentType: text("parent_type").notNull(),
});

/** Map a type onto its table, with an owner in the universe. */
function ownedMapping(policy: Policy, table: Table): TableMapping {
    return {
        policy,
        table,
        id: "id",
        scope: "scope",
        attributes: {},
        relations: { owner: { column: "owner", scope: "universe" } },
    };
}

/** Map each type onto its table. */
const mappings: TableMapping[] = [
    ownedMapping(page, pageTable),
    ownedMapping(sheet, sheetTable),
    {
        policy: piece,
        table: pieceTable,
        id: "id",
        scope: "scope",
        attributes: {},
        relations: {
            parent: {
                column: "parentId",
                subject: { packageId: "parentPackageId", type: "parentType", scope: "scope" },
            },
        },
    },
];

/** Grant alice's token one permission, on one object or a whole scope. */
function restricted(
    source: PermissionReference,
    entry: Omit<GrantPermission, keyof PermissionReference>,
): AccessContext {
    const alice = principal.user.reference("universe", "alice");

    return {
        subject: alice,
        groups: [],
        actors: [principal.personalAccessToken.reference("alice", "token-alice")],
        attributes: {},
        now: 1000,
        grant: {
            id: Identifier.create("grant"),
            scope: "alice",
            permissions: [{ ...source, ...entry }],
        },
        contacts: [],
        linkSecrets: [],
    };
}

test.each(TEST_DIALECTS)(
    "allow a derived permission exactly where a grant allows its source, in memory and in SQL, on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(
            dialect,
            [pageTable, sheetTable, pieceTable, ...accessTables],
            { isMigrated: true },
        );
        onTestFinished(() => storage.close());
        const database = storage.database;
        const authorizer = new Authorizer([page, sheet, piece], mappings);

        // insert two pages and a sheet of alice's, with one piece of text each
        await database.insert(pageTable).values([
            { id: "one", scope: "personal", owner: "alice" },
            { id: "two", scope: "personal", owner: "alice" },
        ]);
        await database.insert(sheetTable).values({ id: "grid", scope: "personal", owner: "alice" });
        await database.insert(pieceTable).values(
            [
                { id: "first", parentId: "one", parentType: "page" },
                { id: "second", parentId: "two", parentType: "page" },
                { id: "cell", parentId: "grid", parentType: "sheet" },
            ].map((row) => ({ ...row, scope: "personal", parentPackageId: module1.package.id })),
        );

        // grant alice's token one permission, on one object or the whole scope
        const everyPage = restricted(page.permission("read"), { scope: "personal" });
        const onePage = restricted(page.permission("read"), { scope: "personal", objectId: "one" });
        const editing = restricted(page.permission("edit"), { scope: "personal" });
        const elsewhere = restricted(page.permission("read"), { scope: "other" });

        // allow pieces and page text as the pages' reading is allowed, and never a sheet's
        const contexts = [everyPage, onePage, editing, elsewhere];
        expect(
            await Promise.all(
                contexts.map((context) => decided(database, authorizer, context, "piece")),
            ),
        ).toEqual([
            {
                listed: ["first", "second"],
                permitted: ["first", "second"],
                checked: ["first", "second"],
            },
            { listed: ["first"], permitted: ["first"], checked: ["first"] },
            { listed: [], permitted: [], checked: [] },
            { listed: [], permitted: [], checked: [] },
        ]);
        expect(
            await Promise.all(
                contexts.map((context) => decided(database, authorizer, context, "page")),
            ),
        ).toEqual([
            { listed: ["one", "two"], permitted: ["one", "two"], checked: ["one", "two"] },
            { listed: ["one"], permitted: ["one"], checked: ["one"] },
            { listed: [], permitted: [], checked: [] },
            { listed: [], permitted: [], checked: [] },
        ]);
    },
);

/** Decide a derived permission per row: listed in SQL, permitted in SQL, checked in memory. */
async function decided(
    database: DatabaseConnection,
    authorizer: Authorizer,
    context: AccessContext,
    type: "piece" | "page",
) {
    // read the rows and resolve the caller
    const [policy, table, derived] =
        type === "piece"
            ? [piece, pieceTable, piece.permission("read")]
            : [page, pageTable, page.permission("text")];
    const snapshot = Snapshot.live(database);
    const access = await authorizer.resolve(snapshot, "personal", context);
    const column = table.id;
    const ids = (await database.select({ id: column }).from(table).orderBy(column)).map(
        (row) => row.id,
    );

    // decide each way
    const listed = await database
        .select({ id: column })
        .from(table)
        .where(authorizer.where(derived, access, table))
        .orderBy(column);
    const permitted: string[] = [];
    const checked: string[] = [];
    for (const id of ids) {
        const target = policy.reference("personal", id);
        const [found] = await database
            .select({ id: column })
            .from(table)
            .where(and(eq(column, id), authorizer.permits(derived, target, access)));
        if (found !== undefined) {
            permitted.push(id);
        }
        if ((await authorizer.check(snapshot, derived, target, access)).isAllowed) {
            checked.push(id);
        }
    }

    return { listed: listed.map((row) => row.id), permitted, checked };
}

test("intersect two grants on the narrower object and method, keeping either side alone when the other gives everything", () => {
    const read = { packageId: module1.package.id, type: "page", name: "read", scope: "space-1" };
    const write = { ...read, name: "write" };
    const one: GrantPermission[] = [{ ...read, objectId: "page-1" }, write];
    const every: GrantPermission[] = [read];

    expect([
        GrantPermission.intersect(one, every),
        GrantPermission.intersect(undefined, every),
        GrantPermission.intersect(undefined, undefined),
        GrantPermission.intersect([write], [{ ...write, scope: "space-2" }]),
        GrantPermission.intersect([{ ...read, method: "get" }], [{ ...read, method: "list" }]),
        GrantPermission.intersect([{ ...read, method: "get" }], every),
    ]).toEqual([
        [{ ...read, objectId: "page-1" }],
        every,
        undefined,
        [],
        [],
        [{ ...read, method: "get" }],
    ]);
});

test("reach every method of a type a grant binds no method of, else only its bound methods", () => {
    const read = { packageId: module1.package.id, type: "page", name: "read", scope: "space-1" };
    const pages = { packageId: module1.package.id, type: "page" };
    const sheets = { packageId: module1.package.id, type: "sheet" };
    const loan: GrantPermission[] = [{ ...read, method: "get" }];

    expect([
        GrantPermission.reaches(pages, "space-1", "get", loan),
        GrantPermission.reaches(pages, "space-1", "list", loan),
        GrantPermission.reaches(pages, "space-2", "get", loan),
        GrantPermission.reaches(sheets, "space-1", "get", loan),
        GrantPermission.reaches(pages, "space-1", "list", [read]),
        GrantPermission.reaches(pages, "space-1", "list", undefined),
    ]).toEqual([true, false, true, true, true, true]);
});
