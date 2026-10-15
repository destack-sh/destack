import { principal } from "@destack/access";
import {
    type CallOf,
    type ObjectReconciliation,
    type ResultOf,
    StatusCondition,
} from "@destack/object";
import { Duration, present } from "@destack/schema";
import { CallError } from "@destack/runtime/error";
import type { Token } from "../credential/credential.ts";
import { ConnectionError } from "../error/error.ts";
import { type Integration, integration, IntegrationIssuer } from "../object/integration.ts";
import {
    type AuthorizationRequest,
    authorizationRequest,
} from "../object/authorization-request.ts";
import {
    type AuthorizationGrant,
    type AuthorizationStart,
    type Connection,
    connection,
} from "../object/connection.ts";
import { Pkce } from "@destack/runtime/authentication";
import { DEVICE_CODE } from "../oauth/server.ts";
import { PERSON_KINDS, Runner, TokenSource } from "./token.ts";

/** The random bytes of a state, 256 bits. */
const STATE_BYTES = 32;

/** The milliseconds a device poll slows down by when asked to (RFC 8628 3.5). */
const SLOW_DOWN_MILLISECONDS = 5000;

/** The authorization of connections: browser pages, installations and device flows. */
export const ConnectionAuthorization = { start, started, exchange, complete, poll };

/** Start authorizing a connection, or start a device authorization. */
async function start(
    call: CallOf<typeof connection, "authorize", unknown>,
    tokens: TokenSource,
): Promise<AuthorizationStart> {
    // refuse a kind no person authorizes
    const target = call.target;
    const kind = target.protocol.kind;
    if (!PERSON_KINDS.has(kind)) {
        throw new CallError("BAD_REQUEST", {
            message: `a ${kind} connection needs no person to authorize it`,
        });
    }

    // start a device authorization, or send the person to a page in a browser
    return kind === "device-code" ? startDevice(call, tokens) : startPage(call, target, tokens);
}

/** Send a person to an installation or authorization page in a browser. */
async function startPage(
    call: CallOf<typeof connection, "authorize", unknown>,
    target: Connection,
    tokens: TokenSource,
): Promise<AuthorizationStart> {
    // require a redirect URI
    const redirectUri = call.input.redirectUri;
    if (redirectUri === undefined) {
        throw new CallError("BAD_REQUEST", {
            message: "authorizing in a browser needs a redirect URI",
        });
    }

    // send the person to an installation page with the state
    const state = crypto.getRandomValues(new Uint8Array(STATE_BYTES)).toHex();
    if (target.protocol.kind === "installation") {
        const installed = await TokenSource.find(call, target.integration);
        requireRedirect(installed, redirectUri);
        const url = installationPage(installed, state).href;

        return { kind: "installation", url, state, redirectUri };
    }

    // require a redirect URI the client registered, or register a client for it
    const registered =
        target.integration === null ? await register(call, target, redirectUri, tokens) : undefined;
    const client =
        registered === undefined
            ? await tokens.integration(call, target.integration)
            : {
                  integration: registered,
                  server: tokens.server(registered, undefined, Runner.clock(call)),
              };
    requireRedirect(client.integration, redirectUri);

    // send the person to the authorization page with the state and the PKCE challenge
    const verifier = Pkce.verifier();
    const url = client.server.authorizationUrl({
        state,
        challenge: await Pkce.challenge(verifier),
        redirectUri,
        scopes: target.scopes,
        ...(target.resource === null ? {} : { resource: target.resource }),
    });
    const created =
        registered === undefined ? null : integration.reference(registered.scope, registered.id);

    return { kind: "page", url: url.href, state, verifier, redirectUri, registered: created };
}

/** Start a device authorization of a connection (RFC 8628 3.1) through its integration's server. */
async function startDevice(
    call: CallOf<typeof connection, "authorize", unknown>,
    tokens: TokenSource,
): Promise<AuthorizationStart> {
    // ask the integration's server for a device code
    const target = call.target;
    const client = await tokens.integration(call, target.integration);
    const device = await client.server.authorizeDevice(target.scopes, target.resource ?? undefined);

    return { kind: "device", ...device };
}

/** Register the client of a connection without an integration. */
async function register(
    call: CallOf<typeof connection, "authorize", unknown>,
    target: Connection,
    redirectUri: string,
    tokens: TokenSource,
): Promise<Integration> {
    // read the resource whose server registers the client, which creation requires without an integration
    const resource = present(
        target.resource,
        "the resource of a connection without an integration",
    );

    // register with the connection's vault where the scope keeps its integrations, else at their home in the enclosing scope
    const placement = tokens.isHome(call.chain)
        ? { scope: call.scope, vault: target.vault }
        : { scope: present(call.chain[1], `the scope enclosing ${call.scope}`), vault: null };
    const request = { resource, redirectUri, vault: placement.vault };

    return tokens.register(placement.scope, request, call);
}

/** Record a started authorization bound to its person and session, and a registered client. */
async function started(
    call: CallOf<typeof connection, "authorize">,
): Promise<ResultOf<typeof connection, "authorize">> {
    // reference the client its registration created
    const target = call.target;
    const prepared = call.prepared;
    if (prepared.kind === "page" && prepared.registered !== null) {
        await call.updateStatus({ integration: prepared.registered });
    }

    // record the request as the person and session starting it
    const caller = requirePerson(call);
    const person = { parentId: target.id, userId: caller.userId, session: caller.session };

    // keep a page's state, redirect and verifier, an installation's state and redirect, or a device's code and interval
    const request =
        prepared.kind === "page"
            ? {
                  kind: prepared.kind,
                  state: prepared.state,
                  redirectUri: prepared.redirectUri,
                  verifier: prepared.verifier,
              }
            : prepared.kind === "installation"
              ? { kind: prepared.kind, state: prepared.state, redirectUri: prepared.redirectUri }
              : {
                    kind: prepared.kind,
                    deviceCode: prepared.deviceCode,
                    interval: prepared.interval,
                };
    await call.mutate(call.caller, [authorizationRequest.call("start", { ...person, ...request })]);

    return prepared.kind === "device"
        ? { url: prepared.url, userCode: prepared.userCode }
        : { url: prepared.url };
}

/** Exchange an authorization's redirect for the credential, kept in the vault before commit. */
async function exchange(
    call: CallOf<typeof connection, "complete", unknown>,
    tokens: TokenSource,
): Promise<AuthorizationGrant> {
    // find the request by its state for the person and session that started it (RFC 9700 4.7)
    const target = call.target;
    const { parameters } = call.input;
    const request = await findRequest(call, target, parameters["state"]);
    const caller = requirePerson(call);
    if (request.userId !== caller.userId || request.session !== caller.session) {
        throw new ConnectionError(
            "INVALID_AUTHORIZATION",
            "only the session that started an authorization completes it",
        );
    }

    // record the installation its issuer reads from the redirect, whose tokens it issues when lent
    if (target.protocol.kind === "installation") {
        refuseError(parameters);
        const issued = await tokens.issuer(call, target.integration);
        const subject = await issued.installation(parameters);

        return { request: request.id, subject, scopes: target.scopes };
    }

    // require the issuer the server returns to be the integration's (RFC 9207 2.4), then no error
    const client = await tokens.integration(call, target.integration);
    requireIssuer(client.integration, parameters["iss"]);
    refuseError(parameters);

    // exchange the code with the verifier and keep the token in the vault
    const token = await client.server.token({
        grant_type: "authorization_code",
        code: present(parameters["code"], "the authorization code"),
        redirect_uri: present(request.redirectUri, "the request's redirect URI"),
        code_verifier: present(request.verifier, "the request's code verifier"),
        ...(target.resource === null ? {} : { resource: target.resource }),
    });
    await tokens.write(call, target, { token });

    return {
        request: request.id,
        scopes: token.scopes ?? target.scopes,
        ...(token.expiresAt === undefined ? {} : { expiresAt: token.expiresAt }),
    };
}

/** Mark a connection authorized by its completed authorization and use the request up. */
async function complete(
    call: CallOf<typeof connection, "complete">,
): Promise<ResultOf<typeof connection, "complete">> {
    // use the request up
    const target = call.target;
    const grant = call.prepared;
    await call.mutate(call.caller, [authorizationRequest.call("delete", { id: grant.request })]);

    // record the granted account, scopes and expiry with the Authorized condition
    return call.updateStatus({
        // keep the time of a token grant's credential, an installation's tokens coming from its issuer
        ...(grant.subject === undefined ? { keptAt: call.now } : { subject: grant.subject }),
        scopes: grant.scopes,
        expiresAt: grant.expiresAt ?? null,
        conditions: {
            ...target.conditions,
            Authorized: StatusCondition.observe(
                target.conditions["Authorized"],
                {
                    status: "true",
                    reason: "Authorized",
                    message: "",
                    observedGeneration: target.generation,
                },
                call.now,
            ),
        },
    });
}

/** Poll a device authorization once (RFC 8628 3.4), returning when to poll again if unsettled. */
async function poll(
    reconciliation: ObjectReconciliation<typeof connection>,
    row: Connection,
    request: AuthorizationRequest,
    tokens: TokenSource,
): Promise<number | undefined> {
    // poll at the interval the server asked for
    const interval = Duration.milliseconds({
        seconds: present(request.interval, "a device authorization's interval"),
    });

    // ask for the token, waiting while the person has yet to approve
    const client = await tokens.integration(Runner.of(reconciliation.store), row.integration);
    let answer: Token | ConnectionError;
    try {
        answer = await client.server.token({
            grant_type: DEVICE_CODE,
            device_code: present(request.deviceCode, "the device code"),
        });
    } catch (error) {
        if (!(error instanceof ConnectionError) || error.code !== "REFUSED") {
            throw error;
        } else if (error.error === "authorization_pending") {
            return interval;
        } else if (error.error === "slow_down") {
            return interval + SLOW_DOWN_MILLISECONDS;
        }
        answer = error;
    }

    // settle the authorization with the answer
    await settle(reconciliation, row, request, answer, tokens);

    return undefined;
}

/** Use up a device authorization's request, then settle the connection. */
async function settle(
    reconciliation: ObjectReconciliation<typeof connection>,
    row: Connection,
    request: AuthorizationRequest,
    answer: Token | ConnectionError,
    tokens: TokenSource,
): Promise<void> {
    // use the request up
    const runner = Runner.of(reconciliation.store);
    await runner.mutate(runner.self, [
        authorizationRequest.call("delete", authorizationRequest.target(request)),
    ]);

    // record a denied or expired authorization
    if (answer instanceof ConnectionError) {
        const denied = { status: "false" as const, reason: "Denied", message: answer.message };
        await tokens.observe(runner, row, denied, {});
    }
    // keep the token and mark the connection authorized
    else {
        await tokens.write(runner, row, { token: answer });
        await tokens.observe(
            runner,
            row,
            { status: "true", reason: "Authorized", message: "" },
            {
                expiresAt: answer.expiresAt ?? null,
                scopes: answer.scopes ?? row.scopes,
                keptAt: runner.now,
            },
        );
    }
}

/** Require an authorization response's issuer to be the integration's (RFC 9207 2.4). */
function requireIssuer(named: Integration, issuer: string | undefined): void {
    const url = IntegrationIssuer.url(named.issuer);
    if (
        (issuer !== undefined && issuer !== url) ||
        (issuer === undefined && named.endpoints.isIssuerReturned)
    ) {
        throw new ConnectionError(
            "INVALID_AUTHORIZATION",
            `the authorization response returns issuer ${issuer ?? "none"} instead of ${url}`,
        );
    }
}

/** Refuse an authorization server's error redirect (RFC 6749 4.1.2.1). */
function refuseError(parameters: Readonly<Record<string, string>>): void {
    if (parameters["error"] !== undefined) {
        throw new ConnectionError(
            "REFUSED",
            parameters["error_description"] ?? parameters["error"],
            { error: parameters["error"] },
        );
    }
}

/** Require a redirect URI an integration registered. */
function requireRedirect(registered: Integration, redirectUri: string): void {
    if (!registered.redirectUris.includes(redirectUri)) {
        throw new CallError("BAD_REQUEST", {
            message: `integration ${registered.clientId} registered no redirect URI ${redirectUri}`,
        });
    }
}

/** Write an integration's installation page, returning to the redirect with the state. */
function installationPage(installed: Integration, state: string): URL {
    // require the installation page and return to it with the state
    const page = installed.endpoints.installation;
    if (page === undefined) {
        throw new ConnectionError(
            "INVALID_AUTHORIZATION",
            `integration ${installed.clientId} has no installation page`,
        );
    }
    const url = new URL(page);
    url.searchParams.set("state", state);

    return url;
}

/** Read the person and session calling, refusing any other caller. */
function requirePerson(call: CallOf<typeof connection, "authorize" | "complete", unknown>): {
    readonly userId: string;
    readonly session: string;
} {
    // require a person calling in a signed-in session
    const { subject, credential } = call.caller;
    if (!principal.user.is(subject) || credential?.kind !== "session") {
        throw new CallError("FORBIDDEN", {
            message: "a person authorizes a connection in a signed-in session",
        });
    }

    return { userId: subject.id, session: credential.id };
}

/** Find a connection's pending browser authorization by its state. */
async function findRequest(
    call: CallOf<typeof connection, "complete", unknown>,
    target: Connection,
    state: string | undefined,
): Promise<AuthorizationRequest> {
    const [request] =
        state === undefined
            ? []
            : await call
                  .reveal(authorizationRequest)
                  .findMany({ where: { parentId: target.id, state } });
    if (request === undefined) {
        throw new ConnectionError(
            "INVALID_AUTHORIZATION",
            "no pending authorization has this state",
        );
    }

    return request;
}
