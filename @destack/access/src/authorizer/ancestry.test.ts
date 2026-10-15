import { present } from "@destack/schema";
import { expect, onTestFinished, test } from "@destack/test";
import { defineTable, integer, type DatabaseConnection, Snapshot, sql, text } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
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

/** Clubs kept as scopes elsewhere, joined by their members and visited while open. */
const club = new Policy(module4.package, {
    name: "club",
    attributes: { open: "number" },
    relations: { member: { subjects: [principal.user] } },
    permissions: { join: relation("member"), visit: resource({ open: 1 }) },
    scope: true,
});

/** The clubs' table, which a database may have without keeping a given club's row. */
const clubTable = defineTable("example_club", {
    id: text("id").primaryKey(),
    scope: text("scope").notNull(),
    open: integer("open").notNull(),
});

/** The club kept elsewhere. */
const chess = club.reference("universe", "chess");

/** Carol, a signed-in user. */
const carol = principal.user.reference("universe", "carol");

test.for(TEST_DIALECTS)(
    "decide a scope with no table here on its ancestry row, refusing terms that need its attributes, on %s",
    async (dialect) => {
        // keep the access tables alone with the club's ancestry row, so clubs map onto the ancestry
        const storage = await TestDatabase.create(dialect, accessTables, { isMigrated: true });
        onTestFinished(() => storage.close());
        const authorizer = new Authorizer([club], []);
        await new AccessFixture(storage.database).replicateScope(chess);

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
    },
);

test.for(TEST_DIALECTS)(
    "decide a scope whose object row lives elsewhere on its ancestry row, and refuse a scope with neither, on %s",
    async (dialect) => {
        // keep the clubs' table, empty, beside the access tables
        const storage = await TestDatabase.create(dialect, [...accessTables, clubTable], {
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

        // refuse even a member while this database keeps no row of the club
        const member = [carol, { ...chess, relation: "member" }];
        expect(await decide(authorizer, storage.database, member)).toEqual({
            join: [false, false],
            visit: [false, false],
        });
        await new AccessFixture(storage.database).replicateScope(chess);

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

        // decide on the club's own row once this database keeps it, open and visited by anyone
        await storage.database
            .insert(clubTable)
            .values({ id: "chess", scope: "universe", open: 1 });
        expect(await decide(authorizer, storage.database, [carol])).toEqual({
            join: [false, false],
            visit: [true, true],
        });
    },
);

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
