import { anyone, type Creation, InstallationId, principal } from "@destack/access";
import type { MethodContextOf, ResultOf } from "@destack/object";
import { present } from "@destack/schema";
import { ConnectionError } from "../error/error.ts";
import { integration, type Registration } from "../object/integration.ts";
import { TokenGrant } from "../oauth/grant.ts";
import { AuthorizationServer } from "../oauth/server.ts";
import { Runner, TokenSource } from "./token.ts";

/** Implement integrations. */
export function implementIntegration(tokens: TokenSource): typeof integration {
    return integration.implement({
        methods: {
            create: {
                // relate anyone to a public integration, and the installation issuing its tokens
                creation: (context) => relate(context),
                authorize: async (context) => requireSecret(context),
                prepare: (context) => keepSecret(context, tokens),
                handle: (_call, next) => next(),
            },
            register: {
                prepare: (context) =>
                    register(
                        present(context.id, "a registered client's identifier"),
                        context.input,
                        tokens,
                        context,
                    ),
                handle: (context, next) => {
                    const input = {
                        ...context.input,
                        ...context.prepared,
                        registration: "dynamic",
                    };

                    return next(context.with({ input }));
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

/** Relate anyone to a public integration, and the installation issuing its tokens. */
async function relate(
    context: MethodContextOf<typeof integration, "create", unknown>,
): Promise<Creation> {
    // relate anyone to a public one
    const { input } = context;
    const everyone =
        input.isPublic === true
            ? [{ relation: "public", subject: anyone.reference("*", "*") }]
            : [];

    // relate the installation an installation issuer declares in its space
    const { issuer } = input;
    const issuing =
        issuer.kind === "installation"
            ? [
                  {
                      relation: "issuer",
                      subject: principal.installation.reference(
                          issuer.issuer.scope,
                          await InstallationId.of(
                              { id: issuer.issuer.scope },
                              issuer.name,
                              context.environment.crypto,
                          ),
                      ),
                  },
              ]
            : [];

    return { relationships: [...everyone, ...issuing] };
}

/** Require a secret exactly when the client authenticates with one, none for an installed app. */
function requireSecret(context: MethodContextOf<typeof integration, "create", unknown>): void {
    // refuse a secret for an installed app
    const { issuer, authentication, secret } = context.input;
    if (issuer.kind === "installation" && secret !== undefined) {
        throw ConnectionError.of(
            "SECRET_UNSUPPORTED",
            "an installed app's integration takes no client secret",
        );
    }
    // require a secret exactly when the client authenticates with one
    else if ((authentication === "none") !== (secret === undefined)) {
        const needed = authentication === "none" ? "no secret" : "a secret";
        throw ConnectionError.of(
            "SECRET_MISMATCH",
            `an integration authenticating with ${authentication} needs ${needed}`,
        );
    }
}

/** Store a created integration's secret. */
async function keepSecret(
    context: MethodContextOf<typeof integration, "create", unknown>,
    tokens: TokenSource,
): Promise<{}> {
    // write the secret of a confidential client
    const { secret } = context.input;
    if (secret !== undefined) {
        const id = present(context.id, "a prepared creation's identifier");
        await tokens.put(context, id, secret);
    }

    return {};
}

/** Register a client at a resource's authorization server, storing an issued secret. */
async function register(
    id: string,
    request: { readonly resource: string; readonly redirectUri: string },
    tokens: TokenSource,
    runner: Runner,
): Promise<Registration> {
    // find the resource's authorization server and its registration endpoint
    const { fetch } = tokens.options;
    const { issuer, endpoints } = await AuthorizationServer.discover(request.resource, fetch);
    if (endpoints.registration === undefined) {
        throw ConnectionError.of(
            "REGISTRATION_UNSUPPORTED",
            `${issuer} does not register clients dynamically`,
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

    // store the issued secret
    await tokens.put(runner, id, issued.secret);

    return registration;
}

/** Delete an integration, destroying a confidential client's secret with it. */
async function destroy(
    context: MethodContextOf<typeof integration, "delete">,
    next: () => Promise<ResultOf<typeof integration, "delete">>,
    tokens: TokenSource,
): Promise<ResultOf<typeof integration, "delete">> {
    // delete the integration, then its secret
    const target = context.target;
    const deleted = await next();
    if (target.authentication !== "none") {
        await tokens.remove(context, target.id);
    }

    return deleted;
}

/** Call the target integration's server as its client, with its secret. */
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
