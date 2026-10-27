import { Secret } from "@destack/vault";
import { Connection, connection, integration } from "@destack/connection/object";
import type { Runner } from "@destack/connection/server";
import type { Lease, LeaseMode } from "@destack/resource";
import { ForgeError } from "../error/index.ts";
import { GitHubSignature } from "./signature.ts";
import { type Repository, repository } from "../object/index.ts";
import { GitHubApp } from "./app.ts";
import { GitHubEvent } from "./event.ts";

/** The repositories the forge hosts at GitHub through the installations of its GitHub App. */
export class GitHubHosting {
    /** The GitHub App the installations belong to. */
    readonly app: GitHubApp;
    /** The signature of the App's webhook deliveries. */
    readonly signature: GitHubSignature;

    /** Host the repositories of an App's installations. */
    constructor(app: GitHubApp) {
        this.app = app;
        this.signature = new GitHubSignature();
    }

    /** Verify a delivery of the App and list the live repositories it changed. */
    async receive(request: Request, runner: Runner): Promise<Repository[]> {
        // verify the delivery and read the changed repository, none for a ping
        const secret = await Secret.text(this.app.secret);
        const event = GitHubEvent.read(
            await this.signature.verify(
                request,
                secret,
                {},
                runner.now,
                this.app.environment.crypto,
            ),
        );
        if (event === null) {
            return [];
        }

        // find the live repositories of the GitHub repository
        const candidates = await runner.read(repository.table).findMany({
            where: {
                hosting: "github",
                providerRepositoryId: event.repositoryId,
                deletedAt: { isNull: true },
            },
        });

        // select those whose account's connection is the delivering installation
        const delivered: Repository[] = [];
        for (const candidate of candidates) {
            const { connectionId } = candidate;
            const installation =
                connectionId === null
                    ? undefined
                    : await this.installation(runner, candidate.accountId, connectionId);
            if (installation === event.installationId) {
                delivered.push(candidate);
            }
        }

        return delivered;
    }

    /** Identify a GitHub repository through the installation of its connection's replica. */
    async identify(
        target: Repository,
        runner: Pick<Runner, "query" | "self" | "read" | "now">,
    ): Promise<string> {
        // require a GitHub origin with the replica of its connection
        const { connectionId, remote } = target;
        if (target.hosting !== "github" || connectionId === null || remote === null) {
            throw new TypeError(`repository ${target.id} has no github origin`);
        }
        const installation = await this.#requireInstallation(
            runner,
            target.accountId,
            connectionId,
        );

        // read the repository by its full name
        const found = await this.app.repository(
            installation,
            GitHubApp.fullName(remote),
            runner.now,
        );

        return found.id;
    }

    /** Lease an identified GitHub repository through an installation token. */
    async open(
        target: Repository,
        mode: LeaseMode,
        runner: Pick<Runner, "query" | "self" | "read" | "now">,
    ): Promise<Lease> {
        // refuse an incomplete origin
        const { connectionId, providerRepositoryId, remote } = target;
        if (connectionId === null || remote === null) {
            throw new TypeError(`github repository ${target.id} has an incomplete origin`);
        }
        // refuse a repository its controller has not identified yet
        else if (providerRepositoryId === null) {
            throw ForgeError.of(
                "GITHUB_IDENTIFIER_MISSING",
                `github repository ${target.id} has no github identifier yet`,
            );
        }

        // issue the token through the account's installation
        const installation = await this.#requireInstallation(
            runner,
            target.accountId,
            connectionId,
        );

        return this.app.open(installation, providerRepositoryId, remote, mode, runner.now);
    }

    /** Read the App installation of an account's connection, as the connection package lists it. */
    async installation(
        runner: Pick<Runner, "read">,
        accountId: string,
        connectionId: string,
    ): Promise<string | undefined> {
        // read the connection lent to the forge from its replica, and its integration
        const lent = await runner
            .read(connection.table)
            .findFirst({ where: { scope: accountId, id: connection.identifier(connectionId) } });
        const integrated =
            lent?.integration === null || lent === undefined
                ? undefined
                : await runner.read(integration.table).findFirst({
                      where: {
                          scope: lent.integration.scope,
                          id: integration.identifier(lent.integration.id),
                      },
                  });

        return lent === undefined
            ? undefined
            : (Connection.installation(lent, integrated, this.app.id) ?? undefined);
    }

    /** Require the App installation of an account's connection. */
    async #requireInstallation(
        runner: Pick<Runner, "query" | "self" | "read">,
        accountId: string,
        connectionId: string,
    ): Promise<string> {
        const installation = await this.installation(runner, accountId, connectionId);
        if (installation === undefined) {
            throw ForgeError.of(
                "CONNECTION_REPLICA_MISSING",
                `the forge has no replica of connection ${connectionId}`,
            );
        }

        return installation;
    }
}
