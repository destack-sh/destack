import { testSimulation } from "@destack/test/world";
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
import { TestDatabase } from "@destack/db/test";
import type { AuthorizationEnvironment } from "../authorizer/authorization.ts";
import { type ObjectReference, Scope } from "@destack/sync";
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
import { AccessFixture } from "../test/access.ts";

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

/** Profiles in the universe, read by their owner. */
const profile = new Policy(module1.package, {
    name: "profile",
    relations: { owner: { subjects: [principal.user] } },
    permissions: { read: relation("owner") },
});

/** People: global scopes in the universe, read by their owner in their own scope. */
const person = new Policy(module1.package, {
    name: "person",
    relations: { owner: { subjects: [principal.user] } },
    permissions: { read: relation("owner") },
    scope: true,
    isGlobal: true,
});

/** Pages, sheets, profiles and people, told apart by the table each lives in. */
const pageTable = ownedTable("example_page");

/** Sheets. */
const sheetTable = ownedTable("example_sheet");

/** Profiles. */
const profileTable = ownedTable("example_profile");

/** People. */
const personTable = ownedTable("example_person");

/** Pieces that store their parent's type in columns. */
const pieceTable = defineTable("example_piece", {
    id: text("id").primaryKey(),
    scope: text("scope").notNull(),
    parentId: text("parent_id").notNull(),
    parentPackageId: text("parent_package_id").notNull(),
    parentType: text("parent_type").notNull(),
});

/** Define a table of owned objects: their identifier, scope and owner. */
function ownedTable(name: string) {
    return defineTable(name, {
        id: text("id").primaryKey(),
        scope: text("scope").notNull(),
        owner: text("owner").notNull(),
    });
}

/** Map a type onto its table, with an owner in the universe. */
function ownerMapping(policy: Policy, table: Table): TableMapping {
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
    ownerMapping(page, pageTable),
    ownerMapping(sheet, sheetTable),
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

/** Grant one of alice's tokens one permission, on one object or a whole scope. */
async function restricted(
    database: DatabaseConnection,
    environment: AuthorizationEnvironment,
    token: string,
    source: PermissionReference,
    entry: Omit<GrantPermission, keyof PermissionReference>,
): Promise<AccessContext> {
    // store alice's grant to the token
    const alice = principal.user.reference("universe", "alice");
    const client = principal.client.reference(Scope.universe.id, token);
    const id = await new AccessFixture(database, environment).replicateGrant(alice, client, [
        { ...source, ...entry },
    ]);

    return {
        subject: alice,
        groups: [],
        actors: [client],
        attributes: {},
        now: 1000,
        grants: [{ id, scope: "alice" }],
        contacts: [],
        linkSecrets: [],
    };
}

test("allow a derived permission exactly where a grant allows its source, in memory and in SQL", async () => {
    await using simulation = testSimulation();
    const environment = simulation.environment("access");
    const storage = await TestDatabase.create(
        [pageTable, sheetTable, pieceTable, ...accessTables],
        { environment: environment, isMigrated: true },
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
    const read = page.permission("read");
    const everyPage = await restricted(database, environment, "client-1", read, {
        scope: "personal",
    });
    const onePage = await restricted(database, environment, "client-2", read, {
        scope: "personal",
        objectId: "one",
    });
    const editing = await restricted(database, environment, "client-3", page.permission("edit"), {
        scope: "personal",
    });
    const elsewhere = await restricted(database, environment, "client-4", read, { scope: "other" });

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
});

test("cap every permission by the grant, lending a global type's permission in every scope, as relations decide", async () => {
    await using simulation = testSimulation();
    const environment = simulation.environment("access");
    const tables = [pageTable, profileTable, personTable, ...accessTables];
    const storage = await TestDatabase.create(tables, { environment, isMigrated: true });
    onTestFinished(() => storage.close());
    const database = storage.database;
    const authorizer = new Authorizer(
        [page, profile, person],
        [
            ownerMapping(page, pageTable),
            ownerMapping(profile, profileTable),
            ownerMapping(person, personTable),
        ],
    );

    // insert a page of alice's, her profile in the universe, and her and bob's people, global scopes
    const universe = Scope.universe.id;
    await database.insert(pageTable).values({ id: "one", scope: "personal", owner: "alice" });
    await database.insert(profileTable).values({ id: "alice", scope: universe, owner: "alice" });
    await database.insert(personTable).values([
        { id: "alice", scope: universe, owner: "alice" },
        { id: "bob", scope: universe, owner: "bob" },
    ]);

    // lend alice's token reading her pages and, in every scope, reading people
    const alice = principal.user.reference(universe, "alice");
    const client = principal.client.reference(universe, "client-1");
    const id = await new AccessFixture(database, environment).replicateGrant(alice, client, [
        { ...page.permission("read"), scope: "personal" },
        person.permission("read"),
    ]);
    const context: AccessContext = {
        subject: alice,
        groups: [],
        actors: [client],
        attributes: {},
        now: 1000,
        grants: [{ id, scope: "alice" }],
        contacts: [],
        linkSecrets: [],
    };

    // check in the governing scope and list in the containing one
    const snapshot = Snapshot.live(database);
    const decide = async (
        asked: PermissionReference,
        target: ObjectReference,
        table: typeof pageTable,
    ) => {
        const governing = await authorizer.resolve(
            snapshot,
            authorizer.governingScope(target),
            context,
        );
        const containing = await authorizer.resolve(snapshot, target.scope, context);
        const checked = await authorizer.check(snapshot, asked, target, governing);
        const listed = await database
            .select({ id: table.id })
            .from(table)
            .where(authorizer.where(asked, containing, table));

        return [checked.isAllowed, listed.map((row) => row.id)];
    };
    const outcomes = [
        await decide(page.permission("edit"), page.reference("personal", "one"), pageTable),
        await decide(
            profile.permission("read"),
            profile.reference(universe, "alice"),
            profileTable,
        ),
        await decide(person.permission("read"), person.reference(universe, "alice"), personTable),
        await decide(person.permission("read"), person.reference(universe, "bob"), personTable),
    ];

    // refuse what the grant omits, even in the universe, and allow the lent person alice owns alone
    expect(outcomes).toEqual([
        [false, []],
        [false, []],
        [true, ["alice"]],
        [false, ["alice"]],
    ]);
});

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

test("intersect two grants on the narrower object and method, taking either side alone when the other gives everything", () => {
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

test("allow every method of a type a grant binds to no method, else only its bound methods", () => {
    const read = { packageId: module1.package.id, type: "page", name: "read", scope: "space-1" };
    const pages = { packageId: module1.package.id, type: "page" };
    const sheets = { packageId: module1.package.id, type: "sheet" };
    const loan: GrantPermission[] = [{ ...read, method: "get" }];

    expect([
        GrantPermission.allowsMethod(pages, "space-1", "get", loan),
        GrantPermission.allowsMethod(pages, "space-1", "list", loan),
        GrantPermission.allowsMethod(pages, "space-2", "get", loan),
        GrantPermission.allowsMethod(sheets, "space-1", "get", loan),
        GrantPermission.allowsMethod(pages, "space-1", "list", [read]),
        GrantPermission.allowsMethod(pages, "space-1", "list", undefined),
    ]).toEqual([true, false, true, true, true, true]);
});

test("give a scope-less grant permission in every scope, and narrow it to the other side's scope", () => {
    const anywhere = { packageId: module1.package.id, type: "page", name: "read" };
    const here: GrantPermission = { ...anywhere, scope: "space-1" };
    const pages = { packageId: module1.package.id, type: "page" };

    expect([
        GrantPermission.gives(anywhere, anywhere, "space-1"),
        GrantPermission.gives(anywhere, anywhere, "space-2"),
        GrantPermission.gives(here, anywhere, "space-2"),
        GrantPermission.allowsMethod(pages, "space-2", "get", [anywhere]),
        GrantPermission.intersect([anywhere], [here]),
        GrantPermission.intersect([here], [{ ...anywhere, scope: "space-2" }]),
    ]).toEqual([true, true, false, true, [here], []]);
});
