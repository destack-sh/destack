import { testSimulation } from "@destack/test/world";
import type { Simulation } from "@destack/environment/simulation";
import { TestDatabase } from "@destack/db/test";
import { Replica, Scope, type Subject } from "@destack/sync";
import type { DatabaseConnection, LogPosition } from "@destack/db";
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
export async function openDatabase(
    partition: string = Scope.universe.id,
    simulation: Simulation = testSimulation(),
) {
    // open the database in its world, a world of its own by default, and authorize its objects as its partition's
    const environment = simulation.environment("access");
    const test = await TestDatabase.create(fixtureDatabase, {
        environment: environment,
        isMigrated: true,
    });
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

    return {
        test,
        simulation,
        database,
        environment,
        authorizer,
        alice,
        bob,
        close: async () => {
            await test.close();
            await simulation[Symbol.asyncDispose]();
        },
    };
}

/** Open a migrated application database home to a partition, with its notes and one grant. */
export async function openFixture(
    partition: string = Scope.universe.id,
    simulation: Simulation = testSimulation(),
) {
    // populate the database in its world and grant bob one note
    const opened = await openDatabase(partition, simulation);
    const { database, environment, authorizer, alice } = opened;
    await database.transaction(async (transaction) => await transaction.insert(item).values(rows));
    await Authorization.run(
        authorizer,
        database,
        () => alice,
        environment,
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

/** Read the home position of the replica including each scope, absent for a scope none includes. */
export async function homePositions(
    database: DatabaseConnection,
    scopes: readonly string[],
): Promise<Record<string, LogPosition | undefined>> {
    const positions = await Promise.all(
        scopes.map(async (scope) => {
            const [replica] = await Replica.including(database, scope);
            const origin =
                replica === undefined
                    ? undefined
                    : await Replica.origin(database, replica.name, replica.scope);

            return [scope, origin] as const;
        }),
    );

    return Object.fromEntries(positions);
}
