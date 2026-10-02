import { principal } from "@destack/access";
import type { Subject } from "@destack/sync";
import { and, eq, isNotNull, isNull, type DatabaseConnection } from "@destack/db";
import type { Call } from "@destack/object";
import { DirectoryStore } from "@destack/directory";
import { identifier, type Identifier } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import { connection, type Connection } from "../../object/connection.ts";
import { Digest } from "../../object/digest.ts";
import type { ConnectionGrant, ConnectionProvider, Vault } from "./provider.ts";

/** How long an authorization waits, GitHub's and Google's ten minute codes. */
const AUTHORIZATION_MILLISECONDS = 10 * 60 * 1000;

/** The random bytes of a state and of a PKCE code verifier, 256 bits. */
const RANDOM_BYTES = 32;

/** One call of a connection method. */
type ConnectionCall = Call<typeof connection.table>;

/** The outcome of a provider exchange. */
interface Exchange {
    /** The external account the provider authorized, with its credential in the vault. */
    readonly grant: Omit<ConnectionGrant, "credential">;
    /** The vault secret keeping an OAuth credential. */
    readonly secret?: VaultSecret;
}

/** A live installation of a provider's application that an account connected. */
export interface ProviderInstallation {
    /** The account holding the connection. */
    readonly scope: Connection["scope"];
    /** The connection. */
    readonly id: Connection["id"];
    /** The provider's installation identifier. */
    readonly installationId: string;
}

/** A vault secret the account service reached on behalf of a principal. */
interface VaultSecret {
    /** The space holding the secret. */
    readonly spaceId: Identifier<"space">;
    /** The secret. */
    readonly secretId: Identifier<"secret">;
    /** The principal the account service acts for. */
    readonly subject: Subject;
}

/** The connection authorization flow. */
export class Connections {
    /** The configured providers, by name. */
    readonly providers: ReadonlyMap<string, ConnectionProvider>;
    /** The vaults holding OAuth credentials. */
    readonly vault: Vault;

    /** Serve connections to the configured providers. */
    constructor(options: {
        readonly providers: readonly ConnectionProvider[];
        readonly vault: Vault;
    }) {
        this.providers = new Map(options.providers.map((provider) => [provider.name, provider]));
        this.vault = options.vault;
    }

    /** Read the live installations of a provider application. */
    static async installations(
        database: DatabaseConnection,
        selection: {
            readonly provider: string;
            readonly applicationId: string;
            readonly scope?: Connection["scope"];
            readonly id?: Connection["id"];
            readonly installationId?: string;
        },
    ): Promise<ProviderInstallation[]> {
        // select the live installations of the application
        const table = connection.table;
        const rows = await database
            .select({ scope: table.scope, id: table.id, installationId: table.installationId })
            .from(table)
            .where(
                and(
                    eq(table.provider, selection.provider),
                    eq(table.applicationId, selection.applicationId),
                    eq(table.kind, "installation"),
                    isNotNull(table.authorizedAt),
                    isNull(table.revokedAt),
                    selection.scope === undefined ? undefined : eq(table.scope, selection.scope),
                    selection.id === undefined ? undefined : eq(table.id, selection.id),
                    selection.installationId === undefined
                        ? undefined
                        : eq(table.installationId, selection.installationId),
                ),
            );

        // read each installation
        return rows.map((row) => ({ ...row, installationId: row.installationId! }));
    }

    /** Give the connection object its authorization flow. */
    handle(): typeof connection {
        return connection.handle({
            authorize: (call, next) => this.#authorize(call, next),
            complete: {
                authorize: async (call) => {
                    requirePending(call);
                },
                prepare: (call) => this.#exchange(call),
                effect: (call) => this.#complete(call),
                settle: async (call, _prepared, isCommitted) => {
                    // destroy the credential of a failed completion
                    const spaceId = call.target?.secretSpaceId;
                    if (!isCommitted && spaceId !== undefined && spaceId !== null) {
                        await this.vault.destroy({ spaceId, secretId: credentialSecretId(call) });
                    }
                },
            },
            cancel: async (call, next) => {
                // drop a pending authorization only
                requirePending(call);

                return next();
            },
            revoke: {
                authorize: async (call) => {
                    requireActive(call);
                },
                prepare: async (call) => this.#grant(call),
                effect: (call) => this.#revoke(call),
                settle: async (_call, prepared, isCommitted) => {
                    // withdraw the grant of a committed revocation
                    if (isCommitted) {
                        await this.#withdraw(prepared as Grant);
                    }
                },
            },
        });
    }

    /** Start an authorization. */
    async #authorize(
        call: ConnectionCall,
        next: (call?: ConnectionCall) => Promise<unknown>,
    ): Promise<unknown> {
        // require a configured provider and a vault for OAuth
        const input = call.input as {
            readonly provider: string;
            readonly scopes: readonly string[];
            readonly secretSpaceId?: string;
            readonly vaultId?: string;
        };
        const provider = this.#provider(input.provider);
        const isVaulted = input.secretSpaceId !== undefined && input.vaultId !== undefined;
        if (isVaulted !== (provider.kind === "oauth")) {
            throw new ServiceError("BAD_REQUEST", {
                message:
                    provider.kind === "oauth"
                        ? "an oauth connection requires the vault keeping its credential"
                        : "an installation keeps no credential in a vault",
            });
        }

        // require the vault's space to lie in the connection's account
        const zone = isVaulted
            ? await new DirectoryStore(call.database).locate(input.secretSpaceId!)
            : undefined;
        if (isVaulted && zone?.scope !== call.scope) {
            throw new ServiceError("BAD_REQUEST", {
                message: `space ${input.secretSpaceId} is no space of account ${call.scope}`,
            });
        }

        // draw the state and verifier, and build the provider page
        const state = crypto.getRandomValues(new Uint8Array(RANDOM_BYTES)).toHex();
        const verifier = crypto
            .getRandomValues(new Uint8Array(RANDOM_BYTES))
            .toBase64({ alphabet: "base64url", omitPadding: true });
        const challenge = await Digest.base64url(verifier);
        const page = provider.authorize({ state, challenge, scopes: input.scopes });

        return next(
            call.with({
                input: {
                    ...call.input,
                    issuer: provider.issuer,
                    applicationId: provider.applicationId,
                    kind: provider.kind,
                    authorizationUrl: page.href,
                    state,
                    verifier,
                },
            }),
        );
    }

    /** Exchange the provider's callback for the authorized account. */
    async #exchange(call: ConnectionCall): Promise<Exchange> {
        // require the pending authorization in time, with the state it holds
        const target = call.target!;
        const input = call.input as {
            readonly state: string;
            readonly parameters: Readonly<Record<string, string>>;
        };
        if (call.now >= target.createdAt + AUTHORIZATION_MILLISECONDS) {
            throw new ServiceError("GONE", {
                status: 410,
                message: "connection authorization expired",
            });
        } else if (input.state !== target.state) {
            throw new ServiceError("BAD_REQUEST", { message: "connection state does not match" });
        }

        // exchange the callback, requiring what the provider's kind of grant holds
        const provider = this.#provider(target.provider);
        const { credential, ...grant } = await provider.complete({
            parameters: input.parameters,
            verifier: target.verifier!,
            scopes: target.scopes,
        });
        if (provider.kind === "installation") {
            if (grant.installationId === undefined) {
                throw new ServiceError("BAD_GATEWAY", {
                    message: `${provider.name} granted no installation`,
                });
            }

            return { grant };
        } else if (credential === undefined) {
            throw new ServiceError("BAD_GATEWAY", {
                message: `${provider.name} granted no credential`,
            });
        }

        // keep the credential in the named vault
        const subject = call.caller!;
        const secretId = credentialSecretId(call);
        await this.vault.write({
            id: secretId,
            spaceId: target.secretSpaceId!,
            vaultId: target.vaultId!,
            name: target.id,
            value: credential,
            subject,
        });

        return { grant, secret: { spaceId: target.secretSpaceId!, secretId, subject } };
    }

    /** Record the authorized account. */
    async #complete(call: ConnectionCall): Promise<unknown> {
        // require the authorization to be pending still, then record its grant
        requirePending(call);
        const { grant, secret } = call.prepared as Exchange;

        return call.update({
            subject: grant.subject,
            installationId: grant.installationId ?? null,
            scopes: [...grant.scopes],
            permissions: grant.permissions,
            secretId: secret?.secretId ?? null,
            expiresAt: grant.expiresAt ?? null,
            authorizedAt: call.now,
            authorizationUrl: null,
            state: null,
            verifier: null,
        });
    }

    /** Read the grant a revocation withdraws after it commits, without its credential. */
    #grant(call: ConnectionCall): Grant {
        const target = call.target!;

        return {
            provider: target.provider,
            subject: target.subject!,
            ...(target.installationId === null ? {} : { installationId: target.installationId }),
            ...(target.secretId === null
                ? {}
                : {
                      secret: {
                          spaceId: target.secretSpaceId!,
                          secretId: target.secretId,
                          subject: call.caller!,
                      },
                  }),
        };
    }

    /** Revoke a grant at its provider, then destroy its credential. */
    async #withdraw(grant: Grant): Promise<void> {
        // read the credential and revoke the grant with it
        const credential =
            grant.secret === undefined ? undefined : await this.vault.read(grant.secret);
        await this.#provider(grant.provider).revoke({
            subject: grant.subject,
            ...(grant.installationId === undefined ? {} : { installationId: grant.installationId }),
            ...(credential === undefined ? {} : { credential }),
        });

        // destroy the credential
        if (grant.secret !== undefined) {
            await this.vault.destroy(grant.secret);
        }
    }

    /** Mark an active connection revoked. */
    async #revoke(call: ConnectionCall): Promise<unknown> {
        requireActive(call);

        return call.update({ revokedAt: call.now });
    }

    /** Read a configured provider by name. */
    #provider(name: string): ConnectionProvider {
        const provider = this.providers.get(name);
        if (provider === undefined) {
            throw new ServiceError("BAD_REQUEST", { message: `unknown provider ${name}` });
        }

        return provider;
    }
}

/** Require a pending connection and its authorizing user as the caller. */
function requirePending(call: ConnectionCall): Connection {
    // refuse an authorized connection, and any caller but its authorizing user
    const target = call.target!;
    const caller = call.caller;
    if (target.authorizedAt !== null) {
        throw new ServiceError("CONFLICT", { message: "connection is authorized" });
    } else if (caller === undefined || !principal.user.is(caller) || caller.id !== target.userId) {
        throw new ServiceError("FORBIDDEN", {
            message: "only the user authorizing a connection completes or cancels it",
        });
    }

    return target;
}

/** Require an authorized connection not revoked yet. */
function requireActive(call: ConnectionCall): Connection {
    const target = call.target!;
    if (target.authorizedAt === null || target.revokedAt !== null) {
        throw new ServiceError("CONFLICT", { message: "connection is not active" });
    }

    return target;
}

/** Build the identifier of the secret keeping a completion's credential. */
function credentialSecretId(call: ConnectionCall): Identifier<"secret"> {
    return identifier("secret").parse(`secret-${call.key!}`);
}

/** A connection's grant at its provider, withdrawn once its revocation commits. */
interface Grant {
    /** The provider holding the grant. */
    readonly provider: string;
    /** The provider's subject of the grant. */
    readonly subject: string;
    /** The provider installation, for installation grants. */
    readonly installationId?: string;
    /** The vault secret holding the grant's credential, for OAuth grants. */
    readonly secret?: VaultSecret;
}
