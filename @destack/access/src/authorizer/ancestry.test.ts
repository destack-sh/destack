import { testSimulation } from "@destack/test/world";
import { present } from "@destack/schema";
import { expect, onTestFinished, test } from "@destack/test";
import { defineTable, integer, type DatabaseConnection, Snapshot, sql, text } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { schema } from "@destack/schema";
import type { ObjectReference, Subject } from "@destack/sync";
import {
    accessTables,
    Authorizer,
    Policy,
    principal,
    relation,
    resource,
    type TableMapping,
} from "../index.ts";
import { AccessFixture } from "../test/access.ts";
import { module4 } from "../test/fixture.ts";

/** Clubs stored as scopes elsewhere, joined by their members and visited while open. */
const club = new Policy(module4.package, {
    name: "club",
    attributes: { open: "number" },
    relations: { member: { subjects: [principal.user] } },
    permissions: { join: relation("member"), visit: resource({ open: 1 }) },
    scope: true,
});

/** The clubs' table, which a database may have without storing a given club's row. */
const clubTable = defineTable("example_club", {
    id: text("id").primaryKey(),
    scope: text("scope").notNull(),
    open: integer("open").notNull(),
});

/** The club stored elsewhere. */
const chess = club.reference("universe", "chess");

/** Carol, a signed-in user. */
const carol = principal.user.reference("universe", "carol");

test("decide a scope with no table here on its ancestry row, refusing terms that need its attributes", async () => {
    // store the access tables alone with the club's ancestry row, so clubs map onto the ancestry
    await using simulation = testSimulation();
    const environment = simulation.environment("access");
    const storage = await TestDatabase.create(accessTables, {
        environment: environment,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const authorizer = new Authorizer([club], []);
    await new AccessFixture(storage.database, environment).replicateScope(chess, club);

    // decide for a caller with the signed set of the club's members, and for one without it
    expect({
        member: await decide(authorizer, storage.database, [
            carol,
            { ...chess, relation: "member" },
        ]),
        stranger: await decide(authorizer, storage.database, [carol]),
    }).toEqual({
        member: { join: [true, true], visit: [false, false] },
        stranger: { join: [false, false], visit: [false, false] },
    });
});

test("decide a scope whose object row lives elsewhere on its ancestry row, and decide a scope with neither by its signed sets alone", async () => {
    // add the clubs' table, empty, beside the access tables
    await using simulation = testSimulation();
    const environment = simulation.environment("access");
    const storage = await TestDatabase.create([...accessTables, clubTable], {
        environment: environment,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const mapping: TableMapping = {
        policy: club,
        table: clubTable,
        id: "id",
        scope: "scope",
        attributes: { open: "open" },
        relations: {},
    };
    const authorizer = new Authorizer([club], [mapping]);

    // let a member's signed set decide joining while this database stores no row of the club, refusing terms on its attributes
    const member = [carol, { ...chess, relation: "member" }];
    expect(await decide(authorizer, storage.database, member)).toEqual({
        join: [true, true],
        visit: [false, false],
    });
    await new AccessFixture(storage.database, environment).replicateScope(chess, club);

    // decide for a caller with the signed set of the club's members, and for one without it
    expect({
        member: await decide(authorizer, storage.database, [
            carol,
            { ...chess, relation: "member" },
        ]),
        stranger: await decide(authorizer, storage.database, [carol]),
    }).toEqual({
        member: { join: [true, true], visit: [false, false] },
        stranger: { join: [false, false], visit: [false, false] },
    });

    // decide on the club's row once this database stores it, open and visited by anyone
    await storage.database.insert(clubTable).values({ id: "chess", scope: "universe", open: 1 });
    expect(await decide(authorizer, storage.database, [carol])).toEqual({
        join: [false, false],
        visit: [true, true],
    });
});

/** Decide a caller's join and visit on the chess club, in memory and in SQL. */
async function decide(
    authorizer: Authorizer,
    database: DatabaseConnection,
    subjects: readonly (Subject | ObjectReference)[],
) {
    // resolve the caller at the universe
    const snapshot = Snapshot.live(database);
    const [subject, ...groups] = subjects;
    const access = await authorizer.resolve(snapshot, "universe", {
        grants: [],
        subject: present(subject, "the caller"),
        actors: [],
        groups,
        contacts: [],
        linkSecrets: [],
        now: 1000,
        attributes: {},
    });

    // decide each permission in memory and as the permission query
    const both = async (name: "join" | "visit") => {
        const permission = club.permission(name);
        const memory = await authorizer.check(snapshot, permission, chess, access);
        const [row] = await database.execute(
            sql`SELECT CASE WHEN ${authorizer.permits(permission, chess, access)} THEN 1 ELSE 0 END AS permitted`,
            schema.record(
                schema.string(),
                schema.union([schema.number(), schema.string(), schema.bigint()]),
            ),
        );

        return [memory.isAllowed, Number(row?.["permitted"]) === 1];
    };

    return { join: await both("join"), visit: await both("visit") };
}
