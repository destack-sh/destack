import type { CallOf, PreparedCallOf, ResultOf } from "@destack/object";
import { present } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import type { ObjectReference } from "@destack/sync";
import { ConnectionError } from "../error/error.ts";
import { integration, type Registration } from "../object/integration.ts";
import { TokenGrant } from "../oauth/grant.ts";
import { AuthorizationServer } from "../oauth/server.ts";
import { TokenSource } from "./token.ts";
import type { ObjectStore } from "@destack/object/server";

/** Serve integrations, keeping each client secret or private key in its vault as the service. */
export function serveIntegrations(tokens: TokenSource): typeof integration {
    return integration.implement({
        methods: {
            create: {
                authorize: async (call) => requireSecretVault(call),
                prepare: (call) => keepSecret(call, tokens),
                handle: (_call, next) => next(),
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
            authorizeDevice: {
                prepare: async (call) => {
                    const { scopes, resource } = call.input.request;

                    return (await client(call, tokens)).authorizeDevice(scopes, resource);
                },
                handle: async (call) => call.prepared,
            },
            register: {
                authorize: async (call) => {
                    if (call.input.vault !== null) {
                        requireVault(call.input.vault, call.scope);
                    }
                },
                prepare: (call) =>
                    register(
                        call.scope,
                        present(call.id, "a registered client's identifier"),
                        call.input,
                        tokens,
                        call.requireStore(),
                    ),
                handle: (call, next) => {
                    const { vault, ...registration } = call.prepared;
                    const input = { ...call.input, ...registration, vault: vault ?? null };

                    return next(call.with({ input: { ...input, registration: "dynamic" } }));
                },
            },
            delete: (call, next) => destroy(call, next, tokens),
        },
    });
}

/** Require a confidential client's vault in the integration's scope, and a secret with a vault. */
function requireSecretVault(call: CallOf<typeof integration, "create">): void {
    // require a vault exactly with a secret, in the integration's scope
    const vault = call.input.vault ?? undefined;
    if ((vault === undefined) !== (call.input.secret === undefined)) {
        throw new ServiceError("BAD_REQUEST", {
            message: "an integration keeps its secret in a vault, and a vault only with a secret",
        });
    } else if (vault !== undefined) {
        requireVault(vault, call.scope);
    }
}

/** Keep a created integration's secret in its vault. */
async function keepSecret(
    call: CallOf<typeof integration, "create">,
    tokens: TokenSource,
): Promise<{}> {
    // write the secret of a confidential client
    const vault = call.input.vault ?? undefined;
    const secret = call.input.secret;
    if (vault !== undefined && secret !== undefined) {
        const id = present(call.id, "a prepared creation's identifier");
        await tokens.put(call.requireStore(), vault, id, secret);
    }

    return {};
}

/** Delete an integration, destroying its secret with it. */
async function destroy(
    call: PreparedCallOf<typeof integration, "delete">,
    next: () => Promise<ResultOf<typeof integration, "delete">>,
    tokens: TokenSource,
): Promise<ResultOf<typeof integration, "delete">> {
    // delete the integration, then its secret
    const target = call.requireTarget();
    const deleted = await next();
    if (target.vault !== null) {
        await tokens.remove(call.requireStore(), target.vault, target.id);
    }

    return deleted;
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
    system: ObjectStore,
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
        issuer,
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
    await tokens.put(system, vault, id, issued.secret);

    return { ...registration, vault };
}

/** Require a vault of the scope itself, which the scope's service keeps. */
export function requireVault(vault: ObjectReference, scope: string): void {
    if (vault.scope !== scope) {
        throw new ServiceError("BAD_REQUEST", {
            message: `a credential of ${scope} lives in a vault of ${scope}, not of ${vault.scope}`,
        });
    }
}

/** Call the target integration's server as its client, with its home vault's secret. */
async function client(
    call: CallOf<typeof integration, "requestToken" | "revokeToken" | "authorizeDevice">,
    tokens: TokenSource,
): Promise<AuthorizationServer> {
    const target = call.requireTarget();

    return tokens.local(call.requireStore(), target);
}
