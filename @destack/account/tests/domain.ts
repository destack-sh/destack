import { directoryTables } from "@destack/directory";
import { principal } from "@destack/access";
import { copyOwner, copyScope } from "@destack/access/test";
import { account, domain, user } from "../src/object/index.ts";
import { accountTables } from "../src/stack/index.ts";
import type { DatabaseConnection, Dialect } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { ResourceContext } from "@destack/resource/context";
import { identifier } from "@destack/schema";
import { Authentication } from "@destack/service/authentication";
import { createClient } from "@destack/service/client";
import { ServiceError } from "@destack/service/error";
import { Health } from "@destack/service/health";
import { Server } from "@destack/service/server";
import { Scope } from "@destack/sync";
import { v7 } from "uuid";
import { defineService } from "@destack/service";
import { ObjectServer } from "@destack/object/server";
import { DomainChallenge } from "../src/object/index.ts";
import { DomainVerifier } from "../src/server/index.ts";
import { testCallKey } from "@destack/service/test";

/** The service serving accounts' domains to the fixture's callers, as the account service serves them. */
export const domainService = defineService("domains", { objects: { domain } });

/** The time scenarios run at: 2026-10-01 00:00 UTC. */
export const NOW = Date.UTC(2026, 9, 1);

/** The header the fixture reads its callers from. */
const USER_HEADER = "x-user";

/** Two accounts with an owner each and a stranger, calling domains over a stand-in DNS. */
export class DomainFixture implements AsyncDisposable {
    /** The owner of the first account. */
    readonly ownerId = identifier("user").parse(`user-${v7()}`);
    /** The owner of the second account. */
    readonly rivalId = identifier("user").parse(`user-${v7()}`);
    /** A user of neither account. */
    readonly strangerId = identifier("user").parse(`user-${v7()}`);
    /** The first account. */
    readonly accountId = identifier("account").parse(`account-${v7()}`);
    /** The second account. */
    readonly otherId = identifier("account").parse(`account-${v7()}`);
    /** A hostname of the scenario's own, apart from other scenarios in the database. */
    readonly hostname = `notes-${crypto.randomUUID().slice(0, 8)}.example.com`;
    /** The global database. */
    readonly database: DatabaseConnection;
    /** The TXT records the stand-in DNS answers, by name. */
    readonly records = new Map<string, string[]>();
    /** The names the stand-in DNS was asked for, in order. */
    readonly lookups: string[] = [];
    /** The hosted domain service. */
    readonly server: Server;

    /** Serve the domain service over a global database. */
    private constructor(database: DatabaseConnection) {
        this.database = database;
        const resolver = {
            txt: async (name: string) => {
                this.lookups.push(name);

                return this.records.get(name) ?? [];
            },
        };
        const objects = new ObjectServer({
            objects: { domain: new DomainVerifier(resolver).handle() },
            policies: [account],
            database,
            callKey: testCallKey,
            origin: { package: domain.package, service: domainService.name },
            history: { ingest: async (batch) => batch.calls.length },
        });
        this.server = Server.start({
            ...objects.implement(domainService),
            audience: domainService.package.id,
            resources: new ResourceContext(),
            health: new Health("domain"),
            authenticate: async (request) => this.#authenticate(request),
            authorizeHost: async () => {},
            drainTimeout: 1000,
        });
    }

    /** Connect to the domain service as a user. */
    client(userId: string) {
        return createClient(domainService, {
            url: "https://domain.test",
            headers: { [USER_HEADER]: userId },
            fetch: (request) => this.server.fetch(request),
        });
    }

    /** Publish the challenge record of a claim, beside the records the name has. */
    publish(domain: { readonly id: string; readonly hostname: string }): void {
        const challenge = DomainChallenge.of(domain);
        const published = this.records.get(challenge.name) ?? [];
        this.records.set(challenge.name, [...published, challenge.value]);
    }

    /** Open the migrated global database of each dialect for a file's scenarios. */
    static async databases(): Promise<Map<Dialect, TestDatabase>> {
        const opened = new Map<Dialect, TestDatabase>();
        for (const dialect of TEST_DIALECTS) {
            const tables = [...accountTables, ...directoryTables];
            opened.set(dialect, await TestDatabase.create(dialect, tables, { isMigrated: true }));
        }

        return opened;
    }

    /** Record the users and the two accounts, each owned by its user, and serve the domains. */
    static async open(database: DatabaseConnection): Promise<DomainFixture> {
        const fixture = new DomainFixture(database);
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
        // record the users as the access copy has them
        const record = { createdAt: 1, updatedAt: 1 };
        for (const userId of [this.ownerId, this.rivalId, this.strangerId]) {
            const person = principal.user.reference(Scope.universe.id, userId);
            await this.database
                .insert(user.table)
                .values({ ...record, id: userId, name: userId, email: `${userId}@example.test` });
            await copyScope(this.database, person);
        }

        // record each account below its owner, and let the owner own it
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
                defaultResidency: "eu",
                scope: ownerId,
            });
            const scope = account.reference(ownerId, accountId);
            await copyScope(this.database, scope);
            await copyOwner(
                this.database,
                scope,
                principal.user.reference(Scope.universe.id, ownerId),
            );
        }
    }

    /** Authenticate the user a request's header carries. */
    #authenticate(request: Request): Authentication {
        // require a user header
        const userId = request.headers.get(USER_HEADER);
        if (userId === null) {
            throw new ServiceError("UNAUTHORIZED", { message: "missing user header" });
        }

        // represent the user alone
        const subject = principal.user.reference(Scope.universe.id, userId);
        const now = Date.now();

        return new Authentication({
            credential: { kind: "fixture", id: "fixture-1" },
            audience: domainService.package.id,
            verifiedAt: now,
            expiresAt: now + 60_000,
            subject,
            subjects: [subject],
        });
    }

    /** Drain the domain service after a scenario. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.server.close();
    }
}
