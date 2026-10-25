import { testSimulation } from "@destack/test/world";
import { expect, onTestFinished, test } from "@destack/test";
import { TestDatabase } from "@destack/db/test";
import { defineTable, Snapshot, text } from "@destack/db";
import { Scope, type Subject } from "@destack/sync";
import {
    accessTables,
    type AccessContext,
    Authorizer,
    subjectSet,
    permission,
    Policy,
    principal,
    relation,
    type TableMapping,
    union,
} from "../index.ts";
import { module1 } from "../test/fixture.ts";

/** The package whose installations notes take pins from. */
const PINNING = "package-019f5530-8000-7000-8000-0000000000a1";

/** Packages as the sets of their installations in a scope. */
const installed = new Policy(module1.package, {
    name: "package",
    relations: { installation: { subjects: [principal.installation] } },
    permissions: {},
});

/** Notes their owner reads, which readers and the pinning package's installations pin. */
const note = new Policy(module1.package, {
    name: "note",
    relations: { owner: { subjects: [principal.user] } },
    permissions: {
        read: relation("owner"),
        pin: union(permission("read"), subjectSet(installed, PINNING, "installation")),
    },
});

/** Installations, each of one package in one scope. */
const installationTable = defineTable("example_installation", {
    id: text("id").primaryKey(),
    scope: text("scope").notNull(),
    packageId: text("package_id").notNull(),
});

/** Notes. */
const noteTable = defineTable("example_note", {
    id: text("id").primaryKey(),
    scope: text("scope").notNull(),
    owner: text("owner").notNull(),
});

/** Map packages onto their installations by package, and notes onto their table. */
const mappings: TableMapping[] = [
    {
        policy: installed,
        table: installationTable,
        id: "packageId",
        scope: "scope",
        attributes: {},
        relations: { installation: { column: "id" } },
    },
    {
        policy: note,
        table: noteTable,
        id: "id",
        scope: "scope",
        attributes: {},
        relations: { owner: { column: "owner", scope: Scope.universe.id } },
    },
];

/** Call as a principal running its own code. */
function caller(subject: Subject): AccessContext {
    return {
        subject,
        groups: [],
        actors: [],
        attributes: {},
        now: 1000,
        grants: [],
        contacts: [],
        linkSecrets: [],
    };
}

test("permit a package's installations in a note's scope as members of its set, beside the note's readers, in memory and in SQL", async () => {
    await using simulation = testSimulation();
    const environment = simulation.environment("access");
    const tables = [installationTable, noteTable, ...accessTables];
    const storage = await TestDatabase.create(tables, { environment, isMigrated: true });
    onTestFinished(() => storage.close());
    const database = storage.database;
    const authorizer = new Authorizer([installed, note], mappings);

    // install the pinning package in two spaces and another package in the first
    await database.insert(installationTable).values([
        { id: "installation-pinning", scope: "space-1", packageId: PINNING },
        { id: "installation-other", scope: "space-1", packageId: module1.package.id },
        { id: "installation-elsewhere", scope: "space-2", packageId: PINNING },
    ]);
    await database.insert(noteTable).values({ id: "note-1", scope: "space-1", owner: "alice" });

    // decide pinning the note as its owner and as each installation
    const snapshot = Snapshot.live(database);
    const target = note.reference("space-1", "note-1");
    const pin = note.permission("pin");
    const subjects = [
        principal.user.reference(Scope.universe.id, "alice"),
        principal.installation.reference("space-1", "installation-pinning"),
        principal.installation.reference("space-1", "installation-other"),
        principal.installation.reference("space-2", "installation-elsewhere"),
    ];
    const decided = await Promise.all(
        subjects.map(async (subject) => {
            const access = await authorizer.resolve(snapshot, "space-1", caller(subject));
            const checked = await authorizer.check(snapshot, pin, target, access);
            const listed = await database
                .select({ id: noteTable.id })
                .from(noteTable)
                .where(authorizer.where(pin, access, noteTable));

            return [checked.isAllowed, listed.map((row) => row.id)];
        }),
    );

    // permit the owner and the pinning package's installation in the note's space alone
    expect(decided).toEqual([
        [true, ["note-1"]],
        [true, ["note-1"]],
        [false, []],
        [false, []],
    ]);
});
