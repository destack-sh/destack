import { ProviderInstallation } from "@destack/account/server";
import { and, eq, isNull, type DatabaseConnection, type Select } from "@destack/db";
import type { Lease, LeaseMode } from "@destack/resource";
import { schema } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import { GitHubSignature } from "@destack/service/github";
import { repository, type RepositoryOrigin } from "../object/index.ts";
import { GitHubApp } from "./app.ts";
import { GitHubEvent } from "./event.ts";

/** The live repositories one webhook delivery changed, and when it arrived. */
export interface GitHubChange {
    /** The repositories connected through the delivering installation. */
    readonly repositories: Select<typeof repository.table>[];
    /** The receiving time, in UTC epoch milliseconds. */
    readonly receivedAt: number;
}

/** The repositories the forge keeps at GitHub through the installations of its GitHub App. */
export class GitHubHosting {
    /** The GitHub App the installations belong to. */
    readonly app: GitHubApp;
    /** The signature of the App's webhook deliveries. */
    readonly signature: GitHubSignature;
    /** The forge's database with its repositories and copies of its residency's connections. */
    readonly #database: DatabaseConnection;

    /** Keep the repositories of an App's installations in the forge's database. */
    constructor(app: GitHubApp, database: DatabaseConnection) {
        this.app = app;
        this.signature = new GitHubSignature();
        this.#database = database;
    }

    /** Verify a delivery signed with the App's webhook secret, and read the repositories it changes, null for a ping. */
    async receive(request: Request, secret: string): Promise<GitHubChange | null> {
        // verify the delivery and read the changed repository, ignoring the ping
        const delivery = await this.signature.verify(request, secret, {}, Date.now());
        const event = GitHubEvent.read(delivery);
        if (event === null) {
            return null;
        }

        // find the live repositories of the GitHub repository and the connections to the delivering installation
        const table = repository.table;
        const [candidates, connections] = await Promise.all([
            this.#database
                .select()
                .from(table)
                .where(
                    and(
                        eq(table.hosting, "github"),
                        eq(table.providerRepositoryId, event.repositoryId),
                        isNull(table.deletionRequestedAt),
                    ),
                ),
            this.#installations({ installationId: event.installationId }),
        ]);

        // keep the repositories connected through that installation only
        const installations = new Set(connections.map((row) => `${row.scope} ${row.id}`));
        const repositories = candidates.filter((candidate) =>
            installations.has(`${candidate.scope} ${candidate.connectedAccountId}`),
        );

        return { repositories, receivedAt: delivery.receivedAt };
    }

    /** Identify a GitHub repository through the installation an account's connection names. */
    async identify(
        scope: string,
        origin: Extract<RepositoryOrigin, { hosting: "github" }>,
    ): Promise<string> {
        const installation = await this.#installation(scope, origin.connectedAccountId);
        const found = await this.app.repository(installation, GitHubApp.fullName(origin.remote));

        return found.id;
    }

    /** Lease a GitHub repository through an installation token limited to it and the mode. */
    async open(target: Select<typeof repository.table>, mode: LeaseMode): Promise<Lease> {
        // require a complete origin
        const { connectedAccountId, providerRepositoryId, remote } = target;
        if (connectedAccountId === null || providerRepositoryId === null || remote === null) {
            throw new TypeError(`github repository ${target.id} has no complete origin`);
        }

        // mint the token through the account's installation
        const installation = await this.#installation(target.scope, connectedAccountId);

        return this.app.open(installation, providerRepositoryId, remote, mode);
    }

    /** Read the installation of the App an account's connection names. */
    async #installation(scope: string, connectedAccountId: string): Promise<string> {
        const [row] = await this.#installations({
            scope: schema.identifier("account").parse(scope),
            id: schema.identifier("connected-account").parse(connectedAccountId),
        });
        if (row === undefined) {
            throw new ServiceError("BAD_REQUEST", {
                message: `connection ${connectedAccountId} is no installation of the github app`,
            });
        }

        return row.installationId;
    }

    /** Read the live installations of the App a selection names. */
    #installations(
        selection: Omit<
            Parameters<typeof ProviderInstallation.list>[1],
            "provider" | "applicationId"
        >,
    ) {
        return ProviderInstallation.list(this.#database, {
            ...selection,
            provider: "github",
            applicationId: this.app.id,
        });
    }
}
