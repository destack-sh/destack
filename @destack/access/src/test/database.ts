import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import type { Subject } from "@destack/sync";
import { fixtureDatabase, item, mappings, node, policies, rows } from "./fixture.ts";
import { Authorizer, principal, type AccessContext, Authorization } from "../index.ts";

/** Open a migrated application database, PostgreSQL when configured, with independent users and one explicit grant. */
export async function openFixture(dialect = TEST_DIALECTS.at(-1)!) {
    // open the application database and authorize its objects
    const test = await TestDatabase.create(dialect, fixtureDatabase, { isMigrated: true });
    const { database } = test;
    const authorizer = new Authorizer(policies, mappings);

    // authenticate two independent users
    const alice: AccessContext = {
        subjects: [principal.user.reference("universe", "alice")],
        now: 1000,
        attributes: {},
    };
    const bob: AccessContext = {
        ...alice,
        subjects: [principal.user.reference("universe", "bob")],
    };

    // populate the migrated database and grant bob one note
    await database.insert(item).values(rows);
    await new Authorization(authorizer, database, () => alice).grant({
        object: node.reference("personal", "b"),
        relation: "editor",
        subject: principal.user.reference("universe", "bob"),
    });

    return { test, database, authorizer, alice, bob, close: () => test.close() };
}

/** Reference a globally identified user. */
export function userSubject(id: string): Subject {
    return principal.user.reference("universe", id);
}
