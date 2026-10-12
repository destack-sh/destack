import type { Token } from "../credential/credential.ts";
import { and, type DatabaseConnection, eq, isNotNull } from "@destack/db";
import { schema } from "@destack/schema";
import { type Integration, integration } from "../object/integration.ts";
import { connection } from "../object/connection.ts";

/** An authorization server whose integrations install on external accounts and issue tokens. */
export interface InstallationIssuer {
    /** The issuer identifier of the integrations it serves, such as the server's origin. */
    readonly issuer: string;
    /** Write the installation page of the integration, returning to the redirect with the state. */
    installationUrl(integration: Pick<Integration, "clientId" | "endpoints">, state: string): URL;
    /** Read the installation from a completed installation's redirect parameters. */
    installation(parameters: Readonly<Record<string, string>>): string;
    /** Issue an installation access token with the client identifier and private key. */
    token(
        clientId: string,
        installation: string,
        privateKey: string,
        grant?: InstallationGrant,
    ): Promise<Token>;
    /** Remove the integration from the account with its client identifier and private key. */
    revoke(clientId: string, installation: string, privateKey: string): Promise<void>;
}

/** What an installation token opens: some of the installation's resources and permissions. */
export interface InstallationGrant {
    /** The resources the token opens, such as repositories, by identifier or name. */
    readonly resources?:
        | { readonly ids: readonly string[] }
        | { readonly names: readonly string[] };
    /** The permissions the token has, by name, such as contents read. */
    readonly permissions?: Readonly<Record<string, string>>;
}

/** An authorized installation of an integration on an external account, kept by a connection. */
export interface Installation {
    /** The scope holding the connection. */
    readonly scope: string;
    /** The connection. */
    readonly id: string;
    /** The issuer's identifier of the installation. */
    readonly installation: string;
}

/** The installations of integrations that connections authorized. */
export const Installation = {
    /** List the authorized installations of the integrations with a client identifier. */
    async list(
        database: DatabaseConnection,
        selection: {
            readonly clientId: string;
            readonly scope?: string;
            readonly id?: string;
            readonly installation?: string;
        },
    ): Promise<Installation[]> {
        // find the integrations with the client identifier
        const integrations = await database
            .select({ id: integration.table.id })
            .from(integration.table)
            .where(eq(integration.table.clientId, selection.clientId));
        const ids = new Set<string>(integrations.map((row) => row.id));

        // keep the authorized installation connections of those integrations
        const rows = await database
            .select()
            .from(connection.table)
            .where(
                and(
                    isNotNull(connection.table.subject),
                    selection.scope === undefined
                        ? undefined
                        : eq(connection.table.scope, selection.scope),
                    selection.id === undefined
                        ? undefined
                        : eq(
                              connection.table.id,
                              schema.identifier("connection").parse(selection.id),
                          ),
                    selection.installation === undefined
                        ? undefined
                        : eq(connection.table.subject, selection.installation),
                ),
            );

        return rows.flatMap((row) =>
            row.protocol.kind === "installation" &&
            row.conditions["Authorized"]?.status === "true" &&
            row.integration !== null &&
            ids.has(row.integration.id) &&
            row.subject !== null
                ? [{ scope: row.scope, id: row.id, installation: row.subject }]
                : [],
        );
    },
};
