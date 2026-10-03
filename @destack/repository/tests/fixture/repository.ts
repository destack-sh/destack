import { onTestFinished } from "@destack/test";

import { directoryTables } from "@destack/directory";
import { accessRelationship, accessRole, principal, Relationship, Role } from "@destack/access";
import { copyOwner, copyScope } from "@destack/access/test";
import { DirectoryStore } from "@destack/directory";
import { account, connection, user } from "@destack/account/object";
import { accountTables } from "@destack/account/stack";
import { Journal } from "@destack/audit";
import type { DatabaseConnection, Dialect } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { ResourceContext } from "@destack/resource/context";
import { schema } from "@destack/schema";
import { Authentication } from "@destack/service/authentication";
import { Health } from "@destack/service/health";
import { Server } from "@destack/service/server";
import { connect } from "../../src/client/index.ts";
import { GitHubApp } from "../../src/github/index.ts";
import { LocalGitStorage } from "../../src/local/index.ts";
import { repository } from "../../src/object/index.ts";
import { RepositoryServer } from "../../src/server/index.ts";
import { repositoryTables } from "../../src/stack/index.ts";
import { temporary } from "./git.ts";
import { APP_ID, GITHUB_API, GitHubStandIn } from "./github.ts";
import { githubPrivateKey } from "./key.ts";
import { testCallKey } from "@destack/service/test";

/** The identifiers the fixture's accounts, users, connections and host take. */
export const ids = {
    account: schema.identifier("account").parse("account-01996ab0-0000-7000-8000-000000000001"),
    other: schema.identifier("account").parse("account-01996ab0-0000-7000-8000-000000000002"),
    owner: schema.identifier("user").parse("user-01996ab0-0000-7000-8000-00000000000a"),
    reader: schema.identifier("user").parse("user-01996ab0-0000-7000-8000-00000000000b"),
    stranger: schema.identifier("user").parse("user-01996ab0-0000-7000-8000-00000000000c"),
    connection: schema
        .identifier("connected-account")
        .parse("connected-account-01996ab0-0000-7000-8000-000000000004"),
    otherConnection: schema
        .identifier("connected-account")
        .parse("connected-account-01996ab0-0000-7000-8000-000000000005"),
    host: schema.identifier("host").parse("host-01996ab0-0000-7000-8000-000000000008"),
    otherHost: schema.identifier("host").parse("host-01996ab0-0000-7000-8000-000000000009"),
};

/** The installation of the fixture GitHub App in the owner's GitHub organisation. */
export const INSTALLATION = 4242;

/** The installation of the fixture GitHub App in another account's GitHub organisation. */
export const OTHER_INSTALLATION = 5151;

/** A region serving an account's repositories: its databases, storage, GitHub stand-in and service. */
export class RepositoryFixture {
    /** The global database of accounts, their connections and the directory. */
    readonly global: DatabaseConnection;
    /** The regional database of repositories and references. */
    readonly database: DatabaseConnection;
    /** The region's platform storage. */
    readonly storage: LocalGitStorage;
    /** The GitHub REST API stand-in. */
    readonly github: GitHubStandIn;
    /** The repository server. */
    readonly server: RepositoryServer;
    /** The HTTP server serving the repository service. */
    readonly http: Server;
    /** The journal of the regional database's calls. */
    readonly journal: Journal;
    /** The failures of committed work to settle, in order. */
    readonly reported: unknown[];

    /** Keep a started region. */
    private constructor(fields: Omit<RepositoryFixture, "connect" | "connectHost">) {
        this.global = fields.global;
        this.database = fields.database;
        this.storage = fields.storage;
        this.github = fields.github;
        this.server = fields.server;
        this.http = fields.http;
        this.journal = fields.journal;
        this.reported = fields.reported;
    }

    /** Start a region of a dialect with the owner's account and a connected GitHub App installation. */
    static async open(dialect: Dialect): Promise<RepositoryFixture> {
        // keep the global and regional databases
        const globalStorage = await TestDatabase.create(
            dialect,
            [...accountTables, ...directoryTables],
            {
                isMigrated: true,
            },
        );
        const regional = await TestDatabase.create(dialect, [...repositoryTables], {
            isMigrated: true,
        });
        onTestFinished(async () => {
            await globalStorage.close();
            await regional.close();
        });

        // keep the accounts globally, and their GitHub App installations' connections copied into the region
        const global = globalStorage.database;
        await global.insert(user.table).values({
            id: ids.owner,
            name: "Owner",
            email: "owner@example.test",
            createdAt: 1,
            updatedAt: 1,
        });
        for (const [id, handle, connectedAccountId, installation] of [
            [ids.account, "acme", ids.connection, INSTALLATION],
            [ids.other, "rival", ids.otherConnection, OTHER_INSTALLATION],
        ] as const) {
            await global.insert(account.table).values({
                id,
                handle,
                name: handle,
                defaultResidency: "eu",
                scope: ids.owner,
                createdAt: 1,
                updatedAt: 1,
            });
            await regional.database.insert(connection.table).values({
                id: connectedAccountId,
                scope: id,
                userId: ids.owner,
                provider: "github",
                issuer: "https://github.com",
                subject: handle,
                applicationId: APP_ID,
                kind: "installation",
                installationId: String(installation),
                scopes: [],
                permissions: { contents: "read", metadata: "read" },
                authorizedAt: 1,
                createdAt: 1,
                updatedAt: 1,
            });
        }

        // copy each account's access into the region: its owner, and a reader of repositories
        const database = regional.database;
        for (const id of [ids.account, ids.other]) {
            const scope = account.reference("universe", id);
            await copyScope(database, scope);
            await copyOwner(database, scope, principal.user.reference("universe", ids.owner));
        }
        await RepositoryFixture.#grantReading(database);

        // serve the repositories with local storage and the GitHub App against the stand-in, recording calls
        const github = new GitHubStandIn();
        const storage = new LocalGitStorage(await temporary("destack-storage-"));
        const reported: unknown[] = [];
        const server = new RepositoryServer({
            callKey: testCallKey,
            database,
            directory: new DirectoryStore(global),
            storage,
            github: new GitHubApp({
                id: APP_ID,
                key: await githubPrivateKey(),
                api: GITHUB_API,
                fetch: github.fetch,
            }),
            fetch: github.fetch,
            report: (error) => reported.push(error),
        });
        const http = RepositoryFixture.#serve(server);

        return new RepositoryFixture({
            global,
            database,
            storage,
            github,
            server,
            http,
            journal: server.objects.journal,
            reported,
        });
    }

    /** Connect to the repository service as a user. */
    connect(userId: string) {
        return connect({
            url: "https://repository.test",
            headers: { "x-user": userId },
            fetch: (request) => this.http.fetch(request),
        });
    }

    /** Connect to the repository service as a host of the account. */
    connectHost(host: string) {
        return connect({
            url: "https://repository.test",
            headers: { "x-host": host },
            fetch: (request) => this.http.fetch(request),
        });
    }

    /** Define a role granting repository reads and pulls in the account and bind it to the reader. */
    static async #grantReading(database: DatabaseConnection): Promise<void> {
        const now = Date.now();
        const role = schema.identifier("role").parse("role-01996ab0-0000-7000-8000-000000000006");
        await database.insert(accessRole).values({
            id: role,
            createdAt: now,
            updatedAt: now,
            scope: ids.account,
            name: "reader",
            description: "Reads and pulls repositories",
        });
        await Role.permit(
            database,
            role,
            ids.account,
            (["read", "pull"] as const).map((name) => {
                const permission = repository.permission(name);

                return { packageId: permission.packageId, type: permission.type, name };
            }),
        );
        await database.insert(accessRelationship).values(
            Relationship.encode(
                {
                    id: schema
                        .identifier("relationship")
                        .parse("relationship-01996ab0-0000-7000-8000-000000000007"),
                    object: account.reference("universe", ids.account),
                    role,
                    subject: principal.user.reference("universe", ids.reader),
                    createdAt: now,
                    expiresAt: null,
                },
                ids.account,
            ),
        );
    }

    /** Serve the repository service to the user in an x-user header or the host in an x-host header. */
    static #serve(server: RepositoryServer): Server {
        // settle through the controllers by hand when each test chooses
        const implementation = server.service();
        const audience = implementation.service.package.id;
        const http = Server.start({
            ...implementation,
            controllers: [],
            audience,
            resources: new ResourceContext(),
            health: new Health("repository"),
            drainTimeout: 1000,
            authorizeHost: async () => {},
            authenticate: async (request) => {
                const host = request.headers.get("x-host");
                const userId = request.headers.get("x-user");
                const subject =
                    host !== null
                        ? principal.host.reference(ids.account, host)
                        : userId !== null
                          ? principal.user.reference("universe", userId)
                          : null;
                if (subject === null) {
                    throw new TypeError("a fixture request names no host or user");
                }
                const now = Date.now();

                return new Authentication({
                    subject,
                    subjects: [subject],
                    credential: { kind: host === null ? "user" : "host-key", id: subject.id },
                    audience,
                    verifiedAt: now,
                    expiresAt: now + 60_000,
                });
            },
        });
        onTestFinished(() => http.close());

        return http;
    }
}
