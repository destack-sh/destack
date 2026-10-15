import { anyone } from "@destack/access";
import type { CallOf, ResultOf } from "@destack/object";
import { present } from "@destack/schema";
import { CallError } from "@destack/runtime/error";
import { Credential } from "../credential/credential.ts";
import { ConnectionError } from "../error/error.ts";
import { integration, type Registration } from "../object/integration.ts";
import { TokenGrant } from "../oauth/grant.ts";
import { AuthorizationServer } from "../oauth/server.ts";
import { Runner, TokenSource } from "./token.ts";
import type { ObjectReference } from "@destack/sync";

/** Implement integrations. */
export function implementIntegration(tokens: TokenSource): typeof integration {
    return integration.implement({
        methods: {
            create: {
                // relate anyone to a public integration as it is created
                creation: (call) =>
                    call.input.isPublic === true
                        ? {
                              relationships: [
                                  { relation: "public", subject: anyone.reference("*", "*") },
                              ],
                          }
                        : {},
                authorize: async (call) => requireSecretVault(call),
                prepare: (call) => keepSecret(call, tokens),
                handle: (_call, next) => next(),
            },
            register: {
                authorize: async (call) => {
                    if (call.input.vault !== null) {
                        Credential.requireVault(call.input.vault, call.scope);
                    }
                },
                prepare: (call) =>
                    register(
                        call.scope,
                        present(call.id, "a registered client's identifier"),
                        call.input,
                        tokens,
                        call,
                    ),
                handle: (call, next) => {
                    const { vault, ...registration } = call.prepared;
                    const input = { ...call.input, ...registration, vault: vault ?? null };

                    return next(call.with({ input: { ...input, registration: "dynamic" } }));
                },
            },
            authorizeDevice: {
                prepare: async (call) => {
                    const { scopes, resource } = call.input.request;

                    return (await client(call, tokens)).authorizeDevice(scopes, resource);
                },
                handle: async (call) => call.prepared,
            },
            requestToken: {
                prepare: async (call) =>
                    (await client(call, tokens)).token(TokenGrant.form(call.input.grant)),
                handle: async (call) => call.prepared,
            },
            revokeToken: {
                prepare: async (call) => {
                    const { token, hint } = call.input.revocation;
                    await (await client(call, tokens)).revoke(token, hint);

                    return {};
                },
                handle: async (call) => call.prepared,
            },
            delete: (call, next) => destroy(call, next, tokens),
        },
    });
}

/** Require a confidential client's vault in the integration's scope, and a secret with a vault. */
function requireSecretVault(call: CallOf<typeof integration, "create", unknown>): void {
    // refuse a secret of an installed app, whose issuer keeps its key
    const vault = call.input.vault ?? undefined;
    if (call.input.issuer.kind === "installation" && vault !== undefined) {
        throw new CallError("BAD_REQUEST", {
            message: "an installed app's issuer keeps its key, so its integration keeps no secret",
        });
    }
    // require a vault exactly with a secret
    else if ((vault === undefined) !== (call.input.secret === undefined)) {
        throw new CallError("BAD_REQUEST", {
            message: "an integration keeps its secret in a vault, and a vault only with a secret",
        });
    }
    // require the vault in the integration's scope
    else if (vault !== undefined) {
        Credential.requireVault(vault, call.scope);
    }
}

/** Keep a created integration's secret in its vault. */
async function keepSecret(
    call: CallOf<typeof integration, "create", unknown>,
    tokens: TokenSource,
): Promise<{}> {
    // write the secret of a confidential client
    const vault = call.input.vault ?? undefined;
    const secret = call.input.secret;
    if (vault !== undefined && secret !== undefined) {
        const id = present(call.id, "a prepared creation's identifier");
        await tokens.put(call, vault, id, secret);
    }

    return {};
}

/** Register a client at a resource's authorization server, keeping an issued secret. */
async function register(
    scope: string,
    id: string,
    request: {
        readonly resource: string;
        readonly redirectUri: string;
        readonly vault: ObjectReference | null;
    },
    tokens: TokenSource,
    runner: Runner,
): Promise<Registration> {
    // find the resource's authorization server and its registration endpoint
    const { fetch } = tokens.options;
    const { issuer, endpoints } = await AuthorizationServer.discover(request.resource, fetch);
    if (endpoints.registration === undefined) {
        throw new ConnectionError(
            "INVALID_AUTHORIZATION",
            `${issuer} registers no clients dynamically`,
        );
    }

    // register a client
    const issued = await AuthorizationServer.register(
        endpoints.registration,
        [request.redirectUri],
        fetch,
    );
    const registration = {
        issuer: { kind: "server" as const, url: issuer },
        endpoints,
        clientId: issued.clientId,
        authentication: issued.authentication,
        redirectUris: [request.redirectUri],
    };
    if (issued.secret === undefined) {
        return registration;
    }

    // keep the issued secret in the registration's vault
    const vault = request.vault;
    if (vault === null) {
        throw new ConnectionError(
            "INVALID_AUTHORIZATION",
            `${issuer} issued a client secret, which ${scope} keeps in no vault`,
        );
    }
    await tokens.put(runner, vault, id, issued.secret);

    return { ...registration, vault };
}

/** Delete an integration, destroying its secret with it. */
async function destroy(
    call: CallOf<typeof integration, "delete">,
    next: () => Promise<ResultOf<typeof integration, "delete">>,
    tokens: TokenSource,
): Promise<ResultOf<typeof integration, "delete">> {
    // delete the integration, then its secret
    const target = call.target;
    const deleted = await next();
    if (target.vault !== null) {
        await tokens.remove(call, target.vault, target.id);
    }

    return deleted;
}

/** Call the target integration's server as its client, with its home vault's secret. */
async function client(
    call: CallOf<typeof integration, "requestToken" | "revokeToken" | "authorizeDevice", unknown>,
    tokens: TokenSource,
): Promise<AuthorizationServer> {
    const target = call.target;

    return tokens.local(call, target);
}
