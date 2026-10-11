import { principal, Caller } from "@destack/access";
import { and, eq } from "@destack/db";
import {
    type CallOf,
    type ObjectReconciliation,
    observeCondition,
    type PreparedCallOf,
    type ResultOf,
} from "@destack/object";
import { Duration, present } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import type { Token } from "../credential/credential.ts";
import { ConnectionError } from "../error/error.ts";
import { type Integration, integration } from "../object/integration.ts";
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
import { Pkce } from "@destack/service/authentication";
import { DEVICE_CODE } from "../oauth/server.ts";
import { PERSON_KINDS, TokenSource } from "./token.ts";
import { RequestId } from "@destack/service/request";

/** The random bytes of a state, 256 bits. */
const STATE_BYTES = 32;

/** The milliseconds a device poll slows down by when asked to (RFC 8628 3.5). */
const SLOW_DOWN_MILLISECONDS = 5000;

/** Start authorizing a connection: register its client where its resource's server allows, then draw the state and verifier, or start a device authorization. */
export async function start(
    call: CallOf<typeof connection, "authorize">,
    tokens: TokenSource,
): Promise<AuthorizationStart> {
    // refuse a kind no person authorizes
    const target = call.requireTarget();
    const kind = target.protocol.kind;
    if (!PERSON_KINDS.has(kind)) {
        throw new ServiceError("BAD_REQUEST", {
            message: `a ${kind} connection needs no person to authorize it`,
        });
    }

    // start a device authorization, or send the person to a page in a browser
    return kind === "device-code" ? startDevice(call, tokens) : startPage(call, target, tokens);
}

/** Send a person to an installation page or an authorization page in a browser, registering the client of a connection without an integration first. */
async function startPage(
    call: CallOf<typeof connection, "authorize">,
    target: Connection,
    tokens: TokenSource,
): Promise<AuthorizationStart> {
    // require a redirect URI the client registered or register a client for it
    const redirectUri = call.input.redirectUri;
    if (redirectUri === undefined) {
        throw new ServiceError("BAD_REQUEST", {
            message: "authorizing in a browser needs a redirect URI",
        });
    }
    const registered =
        target.integration === null ? await register(call, target, redirectUri, tokens) : undefined;
    const client =
        registered === undefined
            ? await tokens.integration(call.requireStore(), target.integration)
            : {
                  integration: registered,
                  server: tokens.server(registered, undefined, call.requireStore().clock),
              };
    if (!client.integration.redirectUris.includes(redirectUri)) {
        throw new ServiceError("BAD_REQUEST", {
            message: `integration ${client.integration.clientId} registered no redirect URI ${redirectUri}`,
        });
    }

    // send the person to the installation page with the state
    const state = crypto.getRandomValues(new Uint8Array(STATE_BYTES)).toHex();
    if (target.protocol.kind === "installation") {
        const issuer = tokens.issuer(client.integration);

        const url = issuer.installationUrl(client.integration, state).href;

        return { kind: "installation", url, state, redirectUri };
    }

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
    call: CallOf<typeof connection, "authorize">,
    tokens: TokenSource,
): Promise<AuthorizationStart> {
    // ask the integration's server for a device code
    const target = call.requireTarget();
    const client = await tokens.integration(call.requireStore(), target.integration);
    const device = await client.server.authorizeDevice(target.scopes, target.resource ?? undefined);

    return { kind: "device", ...device };
}

/** Register the client of a connection without an integration in the account enclosing its space, or in the connection's scope with its vault where integrations are kept. */
async function register(
    call: CallOf<typeof connection, "authorize">,
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
    const placement =
        tokens.options.home === undefined
            ? { scope: call.scope, vault: target.vault }
            : { scope: present(call.chain[1], `the scope enclosing ${call.scope}`), vault: null };
    const request = { resource, redirectUri, vault: placement.vault };

    return tokens.register(placement.scope, request, call.requireStore());
}

/** Record a started authorization bound to the person and session starting it, and the client a registration created. */
export async function started(
    call: PreparedCallOf<typeof connection, "authorize">,
): Promise<ResultOf<typeof connection, "authorize">> {
    // reference the client its registration created
    const target = call.requireTarget();
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

/** Exchange an authorization's redirect for the external account's credential, kept in the vault before the transaction. */
export async function exchange(
    call: CallOf<typeof connection, "complete">,
    tokens: TokenSource,
): Promise<AuthorizationGrant> {
    // find the request by its state for the person and session that started it (RFC 9700 4.7)
    const target = call.requireTarget();
    const { parameters } = call.input;
    const request = await findRequest(call, target, parameters["state"]);
    const caller = requirePerson(call);
    if (request.userId !== caller.userId || request.session !== caller.session) {
        throw new ConnectionError(
            "INVALID_AUTHORIZATION",
            "only the session that started an authorization completes it",
        );
    }

    // require the issuer the server returns to be the integration's (RFC 9207 2.4)
    const client = await tokens.integration(call.requireStore(), target.integration);
    requireIssuer(client.integration, parameters["iss"]);

    // refuse a server's error redirect
    if (parameters["error"] !== undefined) {
        throw new ConnectionError(
            "REFUSED",
            parameters["error_description"] ?? parameters["error"],
            { error: parameters["error"] },
        );
    }

    // record the installation whose tokens its issuer issues when lent
    if (target.protocol.kind === "installation") {
        const subject = tokens.issuer(client.integration).installation(parameters);

        return { request: request.id, subject, scopes: target.scopes };
    }

    // exchange the code with the verifier and keep the token in the vault
    const token = await client.server.token({
        grant_type: "authorization_code",
        code: present(parameters["code"], "the authorization code"),
        redirect_uri: present(request.redirectUri, "the request's redirect URI"),
        code_verifier: present(request.verifier, "the request's code verifier"),
        ...(target.resource === null ? {} : { resource: target.resource }),
    });
    const system = call.requireStore();
    await tokens.write(system, target, { ...(await tokens.read(system, target)), token });

    return {
        request: request.id,
        scopes: token.scopes ?? target.scopes,
        ...(token.expiresAt === undefined ? {} : { expiresAt: token.expiresAt }),
    };
}

/** Mark a connection authorized by its completed authorization and use the request up. */
export async function complete(
    call: PreparedCallOf<typeof connection, "complete">,
): Promise<ResultOf<typeof connection, "complete">> {
    // use the request up
    const target = call.requireTarget();
    const grant = call.prepared;
    await call.mutate(call.caller, [authorizationRequest.call("delete", { id: grant.request })]);

    // record the granted account, scopes and expiry with the Authorized condition
    return call.updateStatus({
        ...(grant.subject === undefined ? {} : { subject: grant.subject }),
        scopes: grant.scopes,
        expiresAt: grant.expiresAt ?? null,
        conditions: {
            ...target.conditions,
            Authorized: observeCondition(
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

/** Poll a connection's device authorization once (RFC 8628 3.4), returning when to poll again, absent once it settles. */
export async function poll(
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
    const client = await tokens.integration(reconciliation.store, row.integration);
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

/** Use a device authorization's request up, then keep its token and mark the connection authorized, or record it denied or expired. */
async function settle(
    reconciliation: ObjectReconciliation<typeof connection>,
    row: Connection,
    request: AuthorizationRequest,
    answer: Token | ConnectionError,
    tokens: TokenSource,
): Promise<void> {
    // use the request up
    const { store, now } = reconciliation;
    await store.mutate(Caller.of(store.principal, now), {
        id: RequestId.create(now),
        calls: [authorizationRequest.call("delete", authorizationRequest.target(request))],
    });

    // record a denied or expired authorization
    if (answer instanceof ConnectionError) {
        const denied = { status: "false" as const, reason: "Denied", message: answer.message };
        await tokens.observe(store, row, denied, {}, now);
    }
    // keep the token and mark the connection authorized
    else {
        await tokens.write(store, row, { ...(await tokens.read(store, row)), token: answer });
        await tokens.observe(
            store,
            row,
            { status: "true", reason: "Authorized", message: "" },
            { expiresAt: answer.expiresAt ?? null, scopes: answer.scopes ?? row.scopes },
            now,
        );
    }
}

/** Require the issuer an authorization response returns to be the integration's, and present where the server returns it (RFC 9207 2.4). */
function requireIssuer(named: Integration, issuer: string | undefined): void {
    if (
        (issuer !== undefined && issuer !== named.issuer) ||
        (issuer === undefined && named.endpoints.isIssuerReturned)
    ) {
        throw new ConnectionError(
            "INVALID_AUTHORIZATION",
            `the authorization response returns issuer ${issuer ?? "none"} instead of ${named.issuer}`,
        );
    }
}

/** Read the person and session calling, refusing any other caller. */
function requirePerson(call: CallOf<typeof connection, "authorize" | "complete">): {
    readonly userId: string;
    readonly session: string;
} {
    // require a person calling in a signed-in session
    const { subject, credential } = call.caller;
    if (subject === undefined || !principal.user.is(subject) || credential?.kind !== "session") {
        throw new ServiceError("FORBIDDEN", {
            message: "a person authorizes a connection in a signed-in session",
        });
    }

    return { userId: subject.id, session: credential.id };
}

/** Find a connection's pending browser authorization by its state. */
async function findRequest(
    call: CallOf<typeof connection, "complete">,
    target: Connection,
    state: string | undefined,
): Promise<AuthorizationRequest> {
    const [request] =
        state === undefined
            ? []
            : await call.database
                  .select()
                  .from(authorizationRequest.table)
                  .where(
                      and(
                          eq(authorizationRequest.table.parentId, target.id),
                          eq(authorizationRequest.table.state, state),
                      ),
                  );
    if (request === undefined) {
        throw new ConnectionError(
            "INVALID_AUTHORIZATION",
            "no pending authorization has this state",
        );
    }

    return request;
}
