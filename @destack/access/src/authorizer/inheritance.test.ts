import { expect, onTestFinished, test } from "@destack/test";
import { asc, type DatabaseConnection, Snapshot } from "@destack/db";
import type { ObjectReference, Subject } from "@destack/sync";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { Relationship } from "../relationship/relationship.ts";
import { accessRelationship } from "../relationship/table.ts";
import { AccessFixture } from "../test/access.ts";
import { fixtureDatabase, item, module1 } from "../test/fixture.ts";
import {
    type AccessContext,
    Authorizer,
    intersection,
    Policy,
    principal,
    relation,
    type TableMapping,
    through,
    union,
} from "../index.ts";

/** Organisations, whose editors and viewers keep their permissions everywhere below them. */
const org = new Policy(module1.package, {
    name: "org",
    relations: { editor: { subjects: [principal.user] }, viewer: { subjects: [principal.user] } },
    permissions: {
        read: union(relation("viewer"), relation("editor")),
        edit: relation("editor"),
    },
    scope: true,
});

/** Folders in an organisation, inheriting its permissions through their enclosing scope. */
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

/** Documents in a folder, owned by their owner field and inheriting the folder's permissions. */
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

/** The documents, kept in the fixture's item table. */
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
    await database
        .insert(accessRelationship)
        .values(
            Relationship.encode(
                { id, object, relation: relationName, subject, createdAt: 1, expiresAt: null },
                object.id,
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

test.for(TEST_DIALECTS)(
    "inherit a scope's relations onto the objects it encloses, through every scope above it, on %s",
    async (dialect) => {
        // keep a folder in an organisation, and three documents owned by someone else in it
        const storage = await TestDatabase.create(dialect, fixtureDatabase, {
            isMigrated: true,
        });
        onTestFinished(() => storage.close());
        const { database } = storage;
        const acme = org.reference("universe", "acme");
        const plans = folder.reference("acme", "plans");
        const copies = new AccessFixture(database);
        await copies.copyScope(acme);
        await copies.copyScope(plans);
        await database.insert(item).values(["a", "b", "c"].map((id) => documentRow(id)));

        // make carol an editor of the organisation, dave a viewer of the folder, and erin nothing
        await relate(database, acme, "editor", people.carol);
        await relate(database, plans, "viewer", people.dave);
        const authorizer = new Authorizer([org, folder, document], [mapping]);

        // list the documents each reads and edits, in SQL and in memory alike
        const allowed = async (person: keyof typeof people, name: "read" | "edit") => {
            const context: AccessContext = {
                subjects: [people[person]],
                now: 1000,
                attributes: {},
            };
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
    },
);

test("refuse a permission read through an enclosing scope that its chain's relationships cannot decide", () => {
    // read a folder permission of viewers who are also editors, which a union of relationships cannot decide
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
        "folder.read is decided along a scope chain, so it follows relations alone",
    );
});

test.for(TEST_DIALECTS)(
    "share an object with everyone with a permission on another, as its relations decide it, on %s",
    async (dialect) => {
        // keep a team whose readers are its viewers and editors, and a note shared with its readers
        const storage = await TestDatabase.create(dialect, fixtureDatabase, {
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
        const authorizer = new Authorizer(
            [team, note],
            [{ ...mapping, policy: note, relations: {} }],
        );
        await new AccessFixture(database).copyScope(org.reference("universe", "plans"));
        await database.insert(item).values({
            id: "a",
            scope: "plans",
            owner: "frank",
            parent: null,
            row: 0,
            column: 0,
            locked: 0,
            team: 0,
            protected: 0,
        });
        const crew = team.reference("plans", "core");
        await relate(database, crew, "editor", people.carol);
        await relate(database, crew, "viewer", people.dave);
        await relate(database, note.reference("plans", "a"), "reader", {
            ...crew,
            relation: "read",
        });

        // let the team's editor and viewer read the note, and nobody else, in SQL and in memory alike
        const reads = async (person: keyof typeof people) => {
            const context: AccessContext = {
                subjects: [people[person]],
                now: 1000,
                attributes: {},
            };
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
    },
);

test.for(TEST_DIALECTS)(
    "share an object with a scope's readers, including those of the scopes enclosing it, on %s",
    async (dialect) => {
        // keep a document in a folder of an organisation, shared with the folder's readers
        const storage = await TestDatabase.create(dialect, fixtureDatabase, {
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
        const accessCopies = new AccessFixture(database);
        await accessCopies.copyScope(acme);
        await accessCopies.copyScope(plans);
        await database.insert(item).values({
            id: "a",
            scope: "plans",
            owner: "frank",
            parent: null,
            row: 0,
            column: 0,
            locked: 0,
            team: 0,
            protected: 0,
        });
        await relate(database, acme, "editor", people.carol);
        await relate(database, plans, "viewer", people.dave);
        await relate(database, shared.reference("plans", "a"), "reader", {
            ...plans,
            relation: "read",
        });

        // let the organisation's editor and the folder's viewer read the document, and nobody else, in SQL and in memory alike
        const reads = async (person: keyof typeof people) => {
            const context: AccessContext = {
                subjects: [people[person]],
                now: 1000,
                attributes: {},
            };
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

            // decide the same row from the organisation, descending into the folder
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
    },
);

test("refuse scope types that enclose each other, and scope relations accepting more than one scope type", () => {
    // enclose a ring in a band that encloses the ring
    const ring: Policy = new Policy(module1.package, {
        name: "ring",
        relations: {
            band: { subjects: ["band"], grantedBy: null, isScope: true },
            viewer: { subjects: [principal.user] },
        },
        permissions: { read: union(relation("viewer"), through("band", "read")) },
        scope: true,
    });
    const band = new Policy(module1.package, {
        name: "band",
        relations: {
            ring: { subjects: [ring], grantedBy: null, isScope: true },
            viewer: { subjects: [principal.user] },
        },
        permissions: { read: union(relation("viewer"), through("ring", "read")) },
        scope: true,
    });
    expect(() => new Authorizer([ring, band], [])).toThrow("band.read encloses itself");

    // accept two scope types through one scope relation
    const loose = new Policy(module1.package, {
        name: "loose",
        relations: { holder: { subjects: [org, folder], grantedBy: null, isScope: true } },
        permissions: { read: through("holder", "read") },
    });
    expect(() => new Authorizer([org, folder, loose], [])).toThrow(
        "loose.holder is the scope holding each object, so it accepts one scope type alone",
    );
});
