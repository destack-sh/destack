import { testSimulation } from "@destack/test/world";
import { expect, onTestFinished, test } from "@destack/test";
import { asc, type DatabaseConnection, Snapshot } from "@destack/db";
import type { ObjectReference, Subject } from "@destack/sync";
import { TestDatabase } from "@destack/db/test";
import { Relationship } from "../relationship/relationship.ts";
import { accessRelationship } from "../relationship/table.ts";
import { AccessFixture } from "../test/access.ts";
import { fixtureDatabase, item, module1 } from "../test/fixture.ts";
import {
    AccessContext,
    Authorizer,
    Caller,
    intersection,
    Policy,
    principal,
    relation,
    type TableMapping,
    through,
    union,
} from "../index.ts";

/** Organizations, with editors and viewers whose permissions apply everywhere below them. */
const org = new Policy(module1.package, {
    name: "org",
    relations: { editor: { subjects: [principal.user] }, viewer: { subjects: [principal.user] } },
    permissions: {
        read: union(relation("viewer"), relation("editor")),
        edit: relation("editor"),
    },
    scope: true,
});

/** Folders in an organization, inheriting its permissions through their enclosing scope. */
const folder = new Policy(module1.package, {
    name: "folder",
    relations: {
        org: { subjects: [org], grantedBy: null, isScope: true },
        editor: { subjects: [principal.user] },
        viewer: { subjects: [principal.user] },
    },
    permissions: {
        read: union(relation("viewer"), relation("editor"), through("org", "read")),
        edit: union(relation("editor"), through("org", "edit")),
    },
    scope: true,
});

/** Documents in a folder, with an owner field and the folder's permissions. */
const document = new Policy(module1.package, {
    name: "doc",
    relations: {
        owner: { subjects: [principal.user], grantedBy: null },
        folder: { subjects: [folder], grantedBy: null, isScope: true },
    },
    permissions: {
        read: union(relation("owner"), through("folder", "read")),
        edit: union(relation("owner"), through("folder", "edit")),
    },
});

/** The documents, stored in the fixture's item table. */
const mapping: TableMapping = {
    policy: document,
    table: item,
    id: "id",
    scope: "scope",
    attributes: {},
    relations: { owner: { column: "owner", scope: "universe" } },
};

/** The people of the scenario. */
const people = {
    carol: principal.user.reference("universe", "carol"),
    dave: principal.user.reference("universe", "dave"),
    erin: principal.user.reference("universe", "erin"),
};

/** The relationships written so far, numbering their identifiers. */
let written = 0;

/** Relate a subject to an object, as declarations and replication do. */
async function relate(
    database: DatabaseConnection,
    object: ObjectReference,
    relationName: string,
    subject: Subject,
): Promise<void> {
    written += 1;
    const id = `relationship-01996ab0-0000-7000-8000-${String(written).padStart(12, "0")}`;
    await database.transaction(
        async (transaction) =>
            await transaction.insert(accessRelationship).values(
                Relationship.encode({
                    id,
                    object,
                    relation: relationName,
                    subject,
                    createdAt: 1,
                    expiresAt: null,
                }),
            ),
    );
}

/** Write a document of frank's in the plans folder as its row. */
function documentRow(id: string) {
    return {
        id,
        scope: "plans",
        owner: "frank",
        parent: null,
        row: 0,
        column: 0,
        locked: 0,
        team: 0,
        protected: 0,
    };
}

test("inherit a scope's relations onto the objects it encloses, through every scope above it", async () => {
    // store a folder in an organization, and three documents of someone else in it
    await using simulation = testSimulation();
    const environment = simulation.environment("access");
    const storage = await TestDatabase.create(fixtureDatabase, {
        environment: environment,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const { database } = storage;
    const acme = org.reference("universe", "acme");
    const plans = folder.reference("acme", "plans");
    const replicas = new AccessFixture(database, environment);
    await replicas.replicateScope(acme, org);
    await replicas.replicateScope(plans, folder);
    await database.transaction(
        async (transaction) =>
            await transaction.insert(item).values(["a", "b", "c"].map((id) => documentRow(id))),
    );

    // make carol an editor of the organization, dave a viewer of the folder, and erin nothing
    await relate(database, acme, "editor", people.carol);
    await relate(database, plans, "viewer", people.dave);
    const authorizer = new Authorizer([org, folder, document], [mapping]);

    // list the documents each reads and edits, in SQL and in memory alike
    const allowed = async (person: keyof typeof people, name: "read" | "edit") => {
        const context = AccessContext.of(Caller.of(people[person]), 1000);
        const access = await authorizer.resolve(Snapshot.live(database), "plans", context);
        const listed = (
            await database
                .select({ id: item.id })
                .from(item)
                .where(authorizer.where(document.permission(name), access))
                .orderBy(asc(item.id))
        ).map((row) => row.id);
        const checked = (
            await authorizer.check(
                Snapshot.live(database),
                document.permission(name),
                document.reference("plans", "a"),
                access,
            )
        ).isAllowed;

        return [listed, checked];
    };
    expect([
        await allowed("carol", "read"),
        await allowed("carol", "edit"),
        await allowed("dave", "read"),
        await allowed("dave", "edit"),
        await allowed("erin", "read"),
    ]).toEqual([
        [["a", "b", "c"], true],
        [["a", "b", "c"], true],
        [["a", "b", "c"], true],
        [[], false],
        [[], false],
    ]);
});

test("refuse a permission read through an enclosing scope that its chain's relationships cannot decide", () => {
    // read a folder permission of viewers who are also editors, beyond a union of relationships
    const narrowed = new Policy(module1.package, {
        name: "folder",
        relations: {
            org: { subjects: [org], grantedBy: null, isScope: true },
            editor: { subjects: [principal.user] },
            viewer: { subjects: [principal.user] },
        },
        permissions: {
            read: intersection(relation("viewer"), relation("editor")),
            edit: relation("editor"),
        },
        scope: true,
    });
    const inside = new Policy(module1.package, {
        name: "doc",
        relations: { folder: { subjects: [narrowed], grantedBy: null, isScope: true } },
        permissions: { read: through("folder", "read") },
    });
    expect(() => new Authorizer([org, narrowed, inside], [{ ...mapping, policy: inside }])).toThrow(
        "folder.read is a chain permission: it can use relations only",
    );
});

test("share an object with everyone with a permission on another, as its relations decide it", async () => {
    // store a team with its viewers and editors as readers, and a note shared with its readers
    await using simulation = testSimulation();
    const environment = simulation.environment("access");
    const storage = await TestDatabase.create(fixtureDatabase, {
        environment: environment,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const { database } = storage;
    const team = new Policy(module1.package, {
        name: "crew",
        relations: {
            editor: { subjects: [principal.user] },
            viewer: { subjects: [principal.user] },
        },
        permissions: { read: union(relation("viewer"), relation("editor")) },
    });
    const note = new Policy(module1.package, {
        name: "memo",
        relations: { reader: { subjects: [team.members("read")] } },
        permissions: { read: relation("reader") },
    });
    const authorizer = new Authorizer([team, note], [{ ...mapping, policy: note, relations: {} }]);
    await new AccessFixture(database, environment).replicateScope(
        org.reference("universe", "plans"),
        org,
    );
    await database.transaction(
        async (transaction) =>
            await transaction.insert(item).values({
                id: "a",
                scope: "plans",
                owner: "frank",
                parent: null,
                row: 0,
                column: 0,
                locked: 0,
                team: 0,
                protected: 0,
            }),
    );
    const crew = team.reference("plans", "core");
    await relate(database, crew, "editor", people.carol);
    await relate(database, crew, "viewer", people.dave);
    await relate(database, note.reference("plans", "a"), "reader", {
        ...crew,
        relation: "read",
    });

    // let the team's editor and viewer read the note, and nobody else, in SQL and in memory alike
    const reads = async (person: keyof typeof people) => {
        const context = AccessContext.of(Caller.of(people[person]), 1000);
        const access = await authorizer.resolve(Snapshot.live(database), "plans", context);
        const listed = (
            await database
                .select({ id: item.id })
                .from(item)
                .where(authorizer.where(note.permission("read"), access))
        ).map((row) => row.id);
        const checked = (
            await authorizer.check(
                Snapshot.live(database),
                note.permission("read"),
                note.reference("plans", "a"),
                access,
            )
        ).isAllowed;

        return [listed, checked];
    };
    expect([await reads("carol"), await reads("dave"), await reads("erin")]).toEqual([
        [["a"], true],
        [["a"], true],
        [[], false],
    ]);
});

test("share an object with a scope's readers, including those of the scopes enclosing it", async () => {
    // store a document in a folder of an organization, shared with the folder's readers
    await using simulation = testSimulation();
    const environment = simulation.environment("access");
    const storage = await TestDatabase.create(fixtureDatabase, {
        environment: environment,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const { database } = storage;
    const shared = new Policy(module1.package, {
        name: "memo",
        relations: { reader: { subjects: [folder.members("read")] } },
        permissions: { read: relation("reader") },
    });
    const authorizer = new Authorizer(
        [org, folder, shared],
        [{ ...mapping, policy: shared, relations: {} }],
    );
    const acme = org.reference("universe", "acme");
    const plans = folder.reference("acme", "plans");
    const replicas = new AccessFixture(database, environment);
    await replicas.replicateScope(acme, org);
    await replicas.replicateScope(plans, folder);
    await database.transaction(
        async (transaction) =>
            await transaction.insert(item).values({
                id: "a",
                scope: "plans",
                owner: "frank",
                parent: null,
                row: 0,
                column: 0,
                locked: 0,
                team: 0,
                protected: 0,
            }),
    );
    await relate(database, acme, "editor", people.carol);
    await relate(database, plans, "viewer", people.dave);
    await relate(database, shared.reference("plans", "a"), "reader", {
        ...plans,
        relation: "read",
    });

    // let the organization's editor and the folder's viewer read the document, and nobody else, in SQL and in memory alike
    const reads = async (person: keyof typeof people) => {
        const context = AccessContext.of(Caller.of(people[person]), 1000);
        const access = await authorizer.resolve(Snapshot.live(database), "plans", context);
        const listed = (
            await database
                .select({ id: item.id })
                .from(item)
                .where(authorizer.where(shared.permission("read"), access))
        ).map((row) => row.id);
        const checked = (
            await authorizer.check(
                Snapshot.live(database),
                shared.permission("read"),
                shared.reference("plans", "a"),
                access,
            )
        ).isAllowed;

        // decide the same row from the organization, descending into the folder
        const above = await authorizer.resolve(Snapshot.live(database), "acme", context);
        const below = await above.descend(Snapshot.live(database), ["plans"]);
        const descended = await authorizer.checkRows(
            Snapshot.live(database),
            shared.permission("read"),
            above,
            [{ id: "a", scope: "plans" }],
            undefined,
            below,
        );

        return [listed, checked, descended.permitted.has(0)];
    };
    expect([await reads("carol"), await reads("dave"), await reads("erin")]).toEqual([
        [["a"], true, true],
        [["a"], true, true],
        [[], false, false],
    ]);
});

test("read a permission through each object's scope, granting nothing where the scope type lacks the permission", async () => {
    // store notes in a shop serving its workers and in a club declaring no serving
    await using simulation = testSimulation();
    const environment = simulation.environment("access");
    const storage = await TestDatabase.create(fixtureDatabase, {
        environment: environment,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const { database } = storage;
    const shop = new Policy(module1.package, {
        name: "shop",
        relations: { worker: { subjects: [principal.user] } },
        permissions: { serve: relation("worker") },
        scope: true,
    });
    const club = new Policy(module1.package, {
        name: "club",
        relations: { worker: { subjects: [principal.user] } },
        permissions: { read: relation("worker") },
        scope: true,
    });
    const note = new Policy(module1.package, {
        name: "note",
        relations: {
            owner: { subjects: [principal.user], grantedBy: null },
            scope: { subjects: [], isScope: true },
        },
        permissions: { read: union(relation("owner"), through("scope", "serve")) },
    });
    const corner = shop.reference("universe", "corner");
    const chess = club.reference("universe", "chess");
    const replicas = new AccessFixture(database, environment);
    await replicas.replicateScope(corner, shop);
    await replicas.replicateScope(chess, club);
    await database.transaction(
        async (transaction) =>
            await transaction.insert(item).values([
                { ...documentRow("a"), scope: "corner" },
                { ...documentRow("b"), scope: "chess" },
            ]),
    );

    // make dave a worker of both
    await relate(database, corner, "worker", people.dave);
    await relate(database, chess, "worker", people.dave);
    const authorizer = new Authorizer([shop, club, note], [{ ...mapping, policy: note }]);

    // list and check the notes dave reads in each scope, in SQL and in memory alike
    const reads = async (scope: string, id: string) => {
        const context = AccessContext.of(Caller.of(people.dave), 1000);
        const access = await authorizer.resolve(Snapshot.live(database), scope, context);
        const listed = (
            await database
                .select({ id: item.id })
                .from(item)
                .where(authorizer.where(note.permission("read"), access))
                .orderBy(asc(item.id))
        ).map((row) => row.id);
        const checked = (
            await authorizer.check(
                Snapshot.live(database),
                note.permission("read"),
                note.reference(scope, id),
                access,
            )
        ).isAllowed;

        return [listed, checked];
    };
    expect([await reads("corner", "a"), await reads("chess", "b")]).toEqual([
        [["a"], true],
        [[], false],
    ]);
});
