import { onTestFinished } from "@destack/test";

import { principal } from "@destack/access";
import type { DirectoryClient } from "@destack/account/client";
import * as hostTest from "@destack/host/test";
import { Journal } from "@destack/audit";
import { LocalBucket } from "@destack/bucket/local";
import { PackageStore } from "@destack/build/store";
import type { DatabaseConnection, Dialect } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { ResourceContext } from "@destack/resource/context";
import { aligned, type Identifier, present, schema } from "@destack/schema";
import { RequestId } from "@destack/service/request";
import { Scope } from "@destack/sync";
import { Authentication, Represented } from "@destack/service/authentication";
import { Health } from "@destack/service/health";
import { Server } from "@destack/service/server";
import { connect } from "../../src/client/index.ts";
import { GitHubApp } from "../../src/github/index.ts";
import { LocalGitStorage } from "../../src/local/index.ts";
import { reference, repository } from "../../src/object/index.ts";
import { type ForgeImplementation, type Forge, implementForge } from "../../src/server/index.ts";
import { forgeService } from "../../src/service/index.ts";
import { forgeDatabase } from "../../src/stack/index.ts";
import { temporary } from "./git.ts";
import { APP_ID, GITHUB_API, GitHubInstallations, GitHubStandIn } from "./github.ts";
import { githubPrivateKey } from "./key.ts";
import { testCallKey } from "@destack/service/test";

/** The identifiers the fixture's accounts, users and hosts take. */
export const ids = {
    account: hostTest.ids.account,
    other: hostTest.ids.other,
    owner: hostTest.ids.owner,
    reader: schema.identifier("user").parse("user-01996ab0-0000-7000-8000-00000000000d"),
    stranger: schema.identifier("user").parse("user-01996ab0-0000-7000-8000-00000000000e"),
    host: schema.identifier("host").parse("host-01996ab0-0000-7000-8000-000000000008"),
    otherHost: schema.identifier("host").parse("host-01996ab0-0000-7000-8000-000000000009"),
};

/** The origin clients call the fixture forge at. */
const ORIGIN = "https://forge.test";

/** The installation of the fixture GitHub App in the owner's GitHub organisation. */
export const INSTALLATION = 4242;

/** The installation of the fixture GitHub App in another account's GitHub organisation. */
export const OTHER_INSTALLATION = 5151;

/** A region serving an account's repositories: its databases, storage, GitHub stand-in and service. */
export class RepositoryFixture {
    /** The account service with the accounts and the directory. */
    readonly accounts: hostTest.AccountFixture;
    /** The forge's database of repositories and references. */
    readonly database: DatabaseConnection;
    /** The directory as the region's host reaches it. */
    readonly directory: DirectoryClient;
    /** The connection of the owner's account to its GitHub App installation. */
    readonly connection: Identifier<"connected-account">;
    /** The region's platform storage. */
    readonly storage: LocalGitStorage;
    /** The GitHub REST API stand-in. */
    readonly github: GitHubStandIn;
    /** The forge serving the repositories. */
    readonly forge: Forge;
    /** The HTTP server serving the forge. */
    readonly http: Server;
    /** The journal of the forge database's calls. */
    readonly journal: Journal;
    /** The failures of committed work to settle, in order. */
    readonly reported: unknown[];
    /** The service's placement, as which it follows the account service. */
    readonly placement: Identifier<"placement">;

    /** Keep a started region. */
    private constructor(
        fields: Omit<RepositoryFixture, "connect" | "connectHost" | "connectSpace" | "settle">,
    ) {
        this.accounts = fields.accounts;
        this.database = fields.database;
        this.directory = fields.directory;
        this.connection = fields.connection;
        this.storage = fields.storage;
        this.github = fields.github;
        this.forge = fields.forge;
        this.http = fields.http;
        this.journal = fields.journal;
        this.reported = fields.reported;
        this.placement = fields.placement;
    }

    /** Start a region of a dialect following the account service, with the owner's accounts, their GitHub App installations and a reader. */
    static async open(dialect: Dialect): Promise<RepositoryFixture> {
        // serve the account service with the GitHub App's installations, and keep the forge's database
        const installations = new GitHubInstallations(
            new Map([
                [String(INSTALLATION), "acme"],
                [String(OTHER_INSTALLATION), "rival"],
            ]),
        );
        const accounts = await hostTest.AccountFixture.open({
            policies: [repository.policy, reference.policy],
            providers: [installations],
        });
        const regional = await TestDatabase.create(dialect, forgeDatabase, {
            isMigrated: true,
        });
        onTestFinished(async () => {
            await accounts[Symbol.asyncDispose]();
            await regional.close();
        });

        // connect each account's installation as the owner, through the account service
        const owner = accounts.user(ids.owner);
        const connections: Identifier<"connected-account">[] = [];
        for (const [accountId, installation] of [
            [ids.account, INSTALLATION],
            [ids.other, OTHER_INSTALLATION],
        ] as const) {
            const pending = await owner.connection.authorize({
                accountId,
                requestId: RequestId.create(),
                provider: installations.name,
                scopes: [],
            });
            const page = new URL(present(pending.authorizationUrl, "the installation page"));
            const state = present(page.searchParams.get("state"), "the installation's state");
            const installed = await owner.connection.complete({
                accountId,
                id: pending.id,
                requestId: RequestId.create(),
                state,
                parameters: { installation_id: String(installation), state },
            });
            connections.push(installed.id);
        }

        // let the reader read and pull the owner's account's repositories
        const role = await owner.role.create({
            accountId: ids.account,
            requestId: RequestId.create(),
            name: "reader",
            description: "Reads and pulls repositories",
            permissions: [repository.permission("read"), repository.permission("pull")],
        });
        await owner.account.grant({
            scope: ids.owner,
            id: ids.account,
            requestId: RequestId.create(),
            role: role.id,
            subject: principal.user.reference(Scope.universe.id, ids.reader),
        });

        // place the service in the platform's region, following and reaching the account service as its workload through the region's host
        const placement = await accounts.place(forgeService.package.id);
        const region = await accounts.enroll(hostTest.ids.platform);
        const identity = accounts.identity(region, placement);
        const directory = identity.directory();

        // serve the repositories with local storage and the GitHub App against the stand-in, recording calls
        const github = new GitHubStandIn();
        const storage = new LocalGitStorage(await temporary("destack-storage-"));
        const bucket = await LocalBucket.open(await temporary("destack-bucket-"), "forge-test");
        onTestFinished(() => bucket[Symbol.asyncDispose]());
        const reported: unknown[] = [];
        const database = regional.database;
        const implementation = implementForge({
            callKey: testCallKey,
            database,
            identity,
            store: new PackageStore(bucket),
            endpoint: new URL(ORIGIN),
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
        const http = RepositoryFixture.#serve(implementation);
        await accounts.settle(implementation.objects, placement);

        return new RepositoryFixture({
            accounts,
            database,
            directory,
            connection: aligned(connections, 0),
            storage,
            github,
            forge: implementation.forge,
            http,
            journal: implementation.objects.journal,
            reported,
            placement,
        });
    }

    /** Wait until the service's copies reach the account service's writes so far. */
    settle(): Promise<void> {
        return this.accounts.settle(this.forge.objects, this.placement);
    }

    /** Connect to the forge as a user. */
    connect(userId: string) {
        return connect({
            url: ORIGIN,
            headers: { "x-user": userId },
            fetch: (request) => this.http.fetch(request),
        });
    }

    /** Connect to the forge as a host of the account. */
    connectHost(host: string) {
        return connect({
            url: ORIGIN,
            headers: { "x-host": host },
            fetch: (request) => this.http.fetch(request),
        });
    }

    /** Connect to the forge service as a host's cell representing a space. */
    connectSpace(host: string, space: string) {
        return connect({
            url: ORIGIN,
            headers: { "x-host": host },
            fetch: Represented.fetch(
                (request) => this.http.fetch(request),
                principal.space.reference(Scope.universe.id, space),
            ),
        });
    }

    /** Serve the forge to the user in an x-user header or the host in an x-host header. */
    static #serve(implementation: ForgeImplementation): Server {
        // settle through the controllers by hand when each test chooses
        const audience = implementation.service.package.id;
        const http = Server.start({
            ...implementation,
            controllers: (implementation.controllers ?? []).filter(
                (controller) => controller.name === "replica",
            ),
            audience,
            resources: new ResourceContext(),
            health: new Health("forge"),
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
