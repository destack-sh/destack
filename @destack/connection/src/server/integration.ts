import { anyone } from "@destack/access";
import type { MethodContextOf, ResultOf } from "@destack/object";
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
                creation: (context) =>
                    context.input.isPublic === true
                        ? {
                              relationships: [
                                  { relation: "public", subject: anyone.reference("*", "*") },
                              ],
                          }
                        : {},
                authorize: async (context) => requireSecretVault(context),
                prepare: (context) => keepSecret(context, tokens),
                handle: (_call, next) => next(),
            },
            register: {
                authorize: async (context) => {
                    if (context.input.vault !== null) {
                        Credential.requireVault(context.input.vault, context.scope);
                    }
                },
                prepare: (context) =>
                    register(
                        context.scope,
                        present(context.id, "a registered client's identifier"),
                        context.input,
                        tokens,
                        context,
                    ),
                handle: (context, next) => {
                    const { vault, ...registration } = context.prepared;
                    const input = { ...context.input, ...registration, vault: vault ?? null };

                    return next(context.with({ input: { ...input, registration: "dynamic" } }));
                },
            },
            authorizeDevice: {
                prepare: async (context) => {
                    const { scopes, resource } = context.input.request;

                    return (await client(context, tokens)).authorizeDevice(scopes, resource);
                },
                handle: async (context) => context.prepared,
            },
            requestToken: {
                prepare: async (context) =>
                    (await client(context, tokens)).token(TokenGrant.form(context.input.grant)),
                handle: async (context) => context.prepared,
            },
            revokeToken: {
                prepare: async (context) => {
                    const { token, hint } = context.input.revocation;
                    await (await client(context, tokens)).revoke(token, hint);

                    return {};
                },
                handle: async (context) => context.prepared,
            },
            delete: (call, next) => destroy(call, next, tokens),
        },
    });
}

/** Require a confidential client's vault in the integration's scope, and a secret with a vault. */
function requireSecretVault(context: MethodContextOf<typeof integration, "create", unknown>): void {
    // refuse a secret of an installed app, whose issuer holds its key
    const vault = context.input.vault ?? undefined;
    if (context.input.issuer.kind === "installation" && vault !== undefined) {
        throw new CallError("BAD_REQUEST", {
            message: "an installed app's issuer holds its key, so its integration has no secret",
        });
    }
    // require a vault exactly with a secret
    else if ((vault === undefined) !== (context.input.secret === undefined)) {
        throw new CallError("BAD_REQUEST", {
            message: "an integration stores its secret in a vault, and a vault only with a secret",
        });
    }
    // require the vault in the integration's scope
    else if (vault !== undefined) {
        Credential.requireVault(vault, context.scope);
    }
}

/** Store a created integration's secret in its vault. */
async function keepSecret(
    context: MethodContextOf<typeof integration, "create", unknown>,
    tokens: TokenSource,
): Promise<{}> {
    // write the secret of a confidential client
    const vault = context.input.vault ?? undefined;
    const secret = context.input.secret;
    if (vault !== undefined && secret !== undefined) {
        const id = present(context.id, "a prepared creation's identifier");
        await tokens.put(context, vault, id, secret);
    }

    return {};
}

/** Register a client at a resource's authorization server, storing an issued secret. */
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

    // store the issued secret in the registration's vault
    const vault = request.vault;
    if (vault === null) {
        throw new ConnectionError(
            "INVALID_AUTHORIZATION",
            `${issuer} issued a client secret, which ${scope} has no vault for`,
        );
    }
    await tokens.put(runner, vault, id, issued.secret);

    return { ...registration, vault };
}

/** Delete an integration, destroying its secret with it. */
async function destroy(
    context: MethodContextOf<typeof integration, "delete">,
    next: () => Promise<ResultOf<typeof integration, "delete">>,
    tokens: TokenSource,
): Promise<ResultOf<typeof integration, "delete">> {
    // delete the integration, then its secret
    const target = context.target;
    const deleted = await next();
    if (target.vault !== null) {
        await tokens.remove(context, target.vault, target.id);
    }

    return deleted;
}

/** Call the target integration's server as its client, with its home vault's secret. */
async function client(
    context: MethodContextOf<
        typeof integration,
        "requestToken" | "revokeToken" | "authorizeDevice",
        unknown
    >,
    tokens: TokenSource,
): Promise<AuthorizationServer> {
    const target = context.target;

    return tokens.local(context, target);
}
