import { TestDatabase } from "@destack/db/test";
import { Scope, type Subject } from "@destack/sync";
import { fixtureDatabase, item, mappings, node, policies, rows } from "./fixture.ts";
import {
    ACCESS_PACKAGE_ID,
    Authorizer,
    principal,
    type AccessContext,
    Authorization,
    type Credential,
    Relationship,
} from "../index.ts";

/** How long a test session credential lasts: a minute, longer than any test. */
const CREDENTIAL_MILLISECONDS = 60_000;

/** Open a migrated database home to a partition, the universe by default, with two users. */
export async function openDatabase(partition: string = Scope.universe.id) {
    // open the database and authorize its objects as its partition's
    const test = await TestDatabase.create(fixtureDatabase, { isMigrated: true });
    const { database } = test;
    const authorizer = new Authorizer(policies, mappings, { partition });

    // authenticate two independent users
    const alice: AccessContext = {
        grants: [],
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

    return { test, database, authorizer, alice, bob, close: () => test.close() };
}

/** Open a migrated application database home to a partition, with its notes and one grant. */
export async function openFixture(partition: string = Scope.universe.id) {
    // populate the database and grant bob one note
    const opened = await openDatabase(partition);
    const { database, authorizer, alice } = opened;
    await database.insert(item).values(rows);
    await Authorization.run(
        authorizer,
        database,
        () => alice,
        (authorization) =>
            Relationship.grant(
                {
                    object: node.reference("personal", "b"),
                    relation: "editor",
                    subject: principal.user.reference("universe", "bob"),
                },
                authorization,
            ),
    );

    return opened;
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
