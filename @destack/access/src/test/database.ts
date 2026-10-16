import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import type { Subject } from "@destack/sync";
import { nth } from "@destack/schema";
import { fixtureDatabase, item, mappings, node, policies, rows } from "./fixture.ts";
import {
    ACCESS_PACKAGE_ID,
    Authorizer,
    principal,
    type AccessContext,
    Authorization,
    type Credential,
} from "../index.ts";

/** How long a test session credential lasts: a minute, longer than any test. */
const CREDENTIAL_MILLISECONDS = 60_000;

/** Open a migrated application database, PostgreSQL if configured, with users and one grant. */
export async function openFixture(dialect = nth(TEST_DIALECTS, -1)) {
    // open the application database and authorize its objects
    const test = await TestDatabase.create(dialect, fixtureDatabase, { isMigrated: true });
    const { database } = test;
    const authorizer = new Authorizer(policies, mappings);

    // authenticate two independent users
    const alice: AccessContext = {
        subject: principal.user.reference("universe", "alice"),
        groups: [],
        now: 1000,
        attributes: {},
        actors: [],
        contacts: [],
        linkSecrets: [],
        credential: sessionCredential("alice", 1000),
    };
    const bob: AccessContext = {
        ...alice,
        subject: principal.user.reference("universe", "bob"),
        groups: [],
        credential: sessionCredential("bob", 1000),
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
export function sessionCredential(id: string, now: number): Credential {
    return {
        kind: "session",
        id,
        audience: ACCESS_PACKAGE_ID,
        verifiedAt: now,
        expiresAt: now + CREDENTIAL_MILLISECONDS,
    };
}
