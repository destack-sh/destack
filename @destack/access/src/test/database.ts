import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import type { Subject } from "@destack/sync";
import { aligned } from "@destack/schema";
import { fixtureDatabase, item, mappings, node, policies, rows } from "./fixture.ts";
import {
    ACCESS_PACKAGE_ID,
    Authorizer,
    principal,
    type AccessContext,
    Authorization,
    type CallerCredential,
} from "../index.ts";

/** How long a test session credential lasts: a minute, longer than any test. */
const CREDENTIAL_MILLISECONDS = 60_000;

/** Open a migrated application database, PostgreSQL if configured, with users and one grant. */
export async function openFixture(dialect = aligned(TEST_DIALECTS, TEST_DIALECTS.length - 1)) {
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
    await Authorization.run(
        authorizer,
        database,
        () => alice,
        (authorization) =>
            authorization.grant({
                object: node.reference("personal", "b"),
                relation: "editor",
                subject: principal.user.reference("universe", "bob"),
            }),
    );

    return { test, database, authorizer, alice, bob, close: () => test.close() };
}

/** Reference a globally identified user. */
export function userSubject(id: string): Subject {
    return principal.user.reference("universe", id);
}

/** Build a session credential verified for access at a time, lapsing a minute later. */
export function sessionCredential(id: string, now: number): CallerCredential {
    return {
        kind: "session",
        id,
        audience: ACCESS_PACKAGE_ID,
        verifiedAt: now,
        expiresAt: now + CREDENTIAL_MILLISECONDS,
    };
}
