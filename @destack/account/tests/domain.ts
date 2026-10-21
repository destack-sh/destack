import { type Authentication, Caller, principal } from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import { Account, account, claim, user, DomainChallenge } from "../src/object/index.ts";
import type { DatabaseConnection, Dialect } from "@destack/db";
import type { EventStore } from "@destack/event";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { ResourceContext } from "@destack/resource/context";
import { Identifier } from "@destack/schema";
import { createClient } from "@destack/object/client";
import { CallError } from "@destack/object";
import { Health, Server, ObjectStore } from "@destack/object/server";
import { Scope } from "@destack/sync";
import { testEvents, serversOf, testCallKey } from "@destack/object/test";
import { Package } from "@destack/package";
import * as server from "../src/server/index.ts";
import accountPackage from "../src/package.ts";
import { AccountFixture, SIGN_UP } from "../src/test/index.ts";
import { Clock } from "@destack/environment";

/** The time scenarios run at: 2026-10-01 00:00 UTC. */
export const NOW = Date.UTC(2026, 9, 1);

/** The header the fixture reads its callers from. */
const USER_HEADER = "x-user";

/** Two accounts with an owner each and a stranger, calling domains over a stand-in DNS. */
export class DomainFixture implements AsyncDisposable {
    /** The owner of the first account. */
    readonly ownerId = Identifier.create("user");
    /** The owner of the second account. */
    readonly rivalId = Identifier.create("user");
    /** A user of neither account. */
    readonly strangerId = Identifier.create("user");
    /** The first account. */
    readonly accountId = Identifier.create("account");
    /** The second account. */
    readonly otherId = Identifier.create("account");
    /** A domain of the scenario, apart from other scenarios in the database. */
    readonly name = `acme-${crypto.randomUUID().slice(0, 8)}.example.com`;
    /** The account database. */
    readonly database: DatabaseConnection;
    /** The TXT records the stand-in DNS answers, by name. */
    readonly records = new Map<string, string[]>();
    /** The names the stand-in DNS was asked for, in order. */
    readonly lookups: string[] = [];
    /** The hosted domains and hostnames. */
    readonly server: Server;

    /** Serve the domains and hostnames over an account database with its event log. */
    private constructor(database: DatabaseConnection, events: EventStore) {
        this.database = database;
        const resolver = {
            txt: async (name: string) => {
                this.lookups.push(name);

                return this.records.get(name) ?? [];
            },
        };
        const objects = new ObjectStore({
            chain: async () => undefined,
            clock: Clock.system,
            principal: Scope.universe,
            servers: serversOf([server.implementDomain({ resolver }), server.implementHostname()]),
            policies: [account],
            database,
            claims: { object: claim },
            callKey: testCallKey,
            events,
            origin: { package: Package.of(accountPackage) },
        });
        this.server = Server.start({
            clock: Clock.system,
            ...objects.package(accountPackage),
            resources: new ResourceContext(),
            health: new Health("domain"),
            authenticate: async (request) => this.#authenticate(request),
            authorizeMachine: async () => {},
            drainTimeout: 1000,
        });
    }

    /** Connect to the domains and hostnames as a user. */
    client(userId: string) {
        return createClient(accountPackage, {
            url: "https://domain.test",
            headers: { [USER_HEADER]: userId },
            fetch: (request) => this.server.fetch(request),
        });
    }

    /** Publish the challenge record of a claim, beside the records the name has. */
    publish(claimed: { readonly scope: string; readonly name: string }): void {
        const challenge = DomainChallenge.of(claimed);
        const published = this.records.get(challenge.name) ?? [];
        this.records.set(challenge.name, [...published, challenge.value]);
    }

    /** Open the migrated account database of each dialect for a file's scenarios. */
    static async databases(): Promise<Map<Dialect, TestDatabase>> {
        const opened = new Map<Dialect, TestDatabase>();
        for (const dialect of TEST_DIALECTS) {
            opened.set(dialect, await AccountFixture.storage(undefined, dialect));
        }

        return opened;
    }

    /** Record the users and their two accounts, and serve domains under a test's entitlements. */
    static async open(database: DatabaseConnection): Promise<DomainFixture> {
        const fixture = new DomainFixture(database, await testEvents(database));
        try {
            await fixture.#provision();

            return fixture;
        } catch (error) {
            await fixture.server.close();
            throw error;
        }
    }

    /** Record the users, and each account below its owner with an owner role. */
    async #provision(): Promise<void> {
        // record the users as the access replica has them
        const record = { createdAt: 1, updatedAt: 1 };
        for (const userId of [this.ownerId, this.rivalId, this.strangerId]) {
            const person = principal.user.reference(Scope.universe.id, userId);
            await this.database.insert(user.table).values({
                ...record,
                ...SIGN_UP,
                id: userId,
                homeId: Account.home(Account.personal(userId)),
                name: userId,
                email: `${userId}@example.test`,
            });
            await new AccessFixture(this.database).replicateScope(person);
        }

        // record each account below its owner
        for (const [accountId, ownerId] of [
            [this.accountId, this.ownerId],
            [this.otherId, this.rivalId],
        ] as const) {
            const handle = `handle-${accountId.slice(-12)}`;
            await this.database.insert(account.table).values({
                ...record,
                id: accountId,
                handle,
                name: handle,
                residencyId: "eu",
                scope: ownerId,
            });
            const scope = account.reference(ownerId, accountId);
            const access = new AccessFixture(this.database);
            await access.replicateScope(scope);
            await access.bindOwner(scope, principal.user.reference(Scope.universe.id, ownerId));
        }
    }

    /** Authenticate the user in a request's header. */
    #authenticate(request: Request): Authentication {
        // require a user header
        const userId = request.headers.get(USER_HEADER);
        if (userId === null) {
            throw new CallError("UNAUTHORIZED", { message: "missing user header" });
        }

        // represent the user alone
        const subject = principal.user.reference(Scope.universe.id, userId);
        const now = Date.now();

        return {
            caller: Caller.of(subject),
            credential: {
                kind: "fixture",
                id: "fixture-1",
                audience: accountPackage.id,
                verifiedAt: now,
                expiresAt: now + 60_000,
            },
        };
    }

    /** Drain the server after a scenario. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.server.close();
    }
}
