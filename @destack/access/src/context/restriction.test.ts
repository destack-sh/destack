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
import {
    accessTables,
    Authorizer,
    permission,
    Policy,
    principal,
    relation,
    through,
    type AccessContext,
    type PermissionReference,
    type Restriction,
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

/** Restrict alice to one permission, on one object or a whole scope. */
function restricted(
    source: PermissionReference,
    restriction: Omit<Restriction, keyof PermissionReference>,
): AccessContext {
    return {
        subjects: [principal.user.reference("universe", "alice")],
        attributes: {},
        now: 1000,
        permissions: [{ ...source, ...restriction }],
    };
}

test.each(TEST_DIALECTS)(
    "allow a derived permission exactly where a credential allows its source, in memory and in SQL, on %s",
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

        // restrict alice to one permission, on one object or the whole scope
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

/** Decide a type's derived permission for every row: listed in SQL, permitted one at a time in SQL, and checked in memory. */
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
