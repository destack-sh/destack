import { principal } from "@destack/access";
import {
    type MethodContextOf,
    type ObjectReconciliation,
    type ResultOf,
    StatusCondition,
} from "@destack/object";
import { Duration, present } from "@destack/schema";
import type { Token } from "../credential/credential.ts";
import type { FamilyFailure } from "@destack/error";
import { ConnectionError, OAuthRefusal } from "../error/error.ts";
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
import { Pkce } from "@destack/access/token";
import { DEVICE_CODE } from "../oauth/server.ts";
import { PERSON_KINDS, Runner, TokenSource } from "./token.ts";

/** The random bytes of a state, 256 bits. */
const STATE_BYTES = 32;

/** The seconds a device poll's interval grows by on each slow_down (RFC 8628 3.5). */
const SLOW_DOWN_SECONDS = 5;

/** The authorization of connections: browser pages, installations and device flows. */
export class ConnectionAuthorization {
    /** The tokens the authorizations obtain and store. */
    readonly #tokens: TokenSource;

    /** Authorize connections through a token source. */
    constructor(tokens: TokenSource) {
        this.#tokens = tokens;
    }

    /** Start authorizing a connection, or start a device authorization. */
    async start(
        context: MethodContextOf<typeof connection, "authorize", unknown>,
    ): Promise<AuthorizationStart> {
        // refuse a kind no person authorizes
        const target = context.target;
        const kind = target.protocol.kind;
        if (!PERSON_KINDS.has(kind)) {
            throw ConnectionError.of(
                "AUTHORIZATION_UNSUPPORTED",
                `a ${kind} connection authorizes without a person`,
            );
        }

        // start a device authorization or a browser page
        return kind === "device-code"
            ? this.#startDevice(context)
            : this.#startPage(context, target);
    }

    /** Record a started authorization bound to its person and session, and a registered client. */
    async started(
        context: MethodContextOf<typeof connection, "authorize">,
    ): Promise<ResultOf<typeof connection, "authorize">> {
        // reference the client its registration created
        const target = context.target;
        const prepared = context.prepared;
        if (prepared.kind === "page" && prepared.registered !== null) {
            await context.updateStatus({ integration: prepared.registered });
        }

        // record the request as the person and session starting it
        const caller = requirePerson(context);
        const person = { parentId: target.id, userId: caller.userId, session: caller.session };

        // store the request's state
        const request = ConnectionAuthorization.#request(prepared);
        await context.mutate(context.caller, [
            authorizationRequest.call("start", { ...person, ...request }),
        ]);

        return prepared.kind === "device"
            ? { url: prepared.url, userCode: prepared.userCode }
            : { url: prepared.url };
    }

    /** Exchange an authorization's redirect for the credential, stored before commit. */
    async exchange(
        context: MethodContextOf<typeof connection, "complete", unknown>,
    ): Promise<AuthorizationGrant> {
        // find the request by its state for the person and session that started it (RFC 9700 4.7)
        const target = context.target;
        const { parameters } = context.input;
        const request = await findRequest(context, target, parameters["state"]);
        const caller = requirePerson(context);
        if (request.userId !== caller.userId || request.session !== caller.session) {
            throw ConnectionError.of(
                "SESSION_MISMATCH",
                "only the session that started an authorization can complete it",
            );
        }

        // record the installation the redirect names
        if (target.protocol.kind === "installation") {
            refuseError(parameters);
            const issued = await this.#tokens.issuer(context, target.integration);
            const subject = await issued.installation(parameters);

            return { request: request.id, subject, scopes: target.scopes };
        }

        // require the issuer, no error and a code (RFC 9207 2.4, RFC 6749 4.1.2)
        const client = await this.#tokens.integration(context, target.integration);
        requireIssuer(client.integration, parameters["iss"]);
        refuseError(parameters);
        const code = parameters["code"];
        if (code === undefined) {
            throw ConnectionError.of(
                "AUTHORIZATION_INVALID",
                "the authorization response has no code",
            );
        }

        // exchange the code with the verifier and store the token in the vault
        const token = await client.server.token({
            grant_type: "authorization_code",
            code,
            redirect_uri: present(request.redirectUri, "the request's redirect URI"),
            code_verifier: present(request.verifier, "the request's code verifier"),
            ...(target.resource === null ? {} : { resource: target.resource }),
        });
        await this.#tokens.write(context, target, { token });

        return {
            request: request.id,
            scopes: token.scopes ?? target.scopes,
            ...(token.expiresAt === undefined ? {} : { expiresAt: token.expiresAt }),
        };
    }

    /** Mark a connection authorized by its completed authorization and use the request up. */
    async complete(
        context: MethodContextOf<typeof connection, "complete">,
    ): Promise<ResultOf<typeof connection, "complete">> {
        // use the request up
        const target = context.target;
        const grant = context.prepared;
        await context.mutate(context.caller, [
            authorizationRequest.call("delete", { id: grant.request }),
        ]);

        // record the grant
        return context.updateStatus({
            // store when the credential was granted
            ...(grant.subject === undefined
                ? { storedAt: context.now }
                : { subject: grant.subject }),
            failures: 0,
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
                    context.now,
                ),
            },
        });
    }

    /** Poll a device authorization once (RFC 8628 3.4), returning when to poll again. */
    async poll(
        reconciliation: ObjectReconciliation<typeof connection>,
        row: Connection,
        request: AuthorizationRequest,
    ): Promise<number | undefined> {
        // poll at the interval the server asked for
        const seconds = present(request.interval, "a device authorization's interval");
        const interval = Duration.milliseconds({ seconds });

        // ask for the token
        const client = await this.#tokens.integration(
            Runner.of(reconciliation.store),
            row.integration,
        );
        let answer: Token | FamilyFailure<"AUTHORIZATION_REFUSED", OAuthRefusal>;
        try {
            answer = await client.server.token({
                grant_type: DEVICE_CODE,
                device_code: present(request.deviceCode, "the device code"),
            });
        } catch (error) {
            // wait while pending or slowed down
            if (!ConnectionError.is(error, "AUTHORIZATION_REFUSED")) {
                throw error;
            }
            const refused = OAuthRefusal.code(error);
            if (refused === "authorization_pending") {
                return interval;
            } else if (refused === "slow_down") {
                return ConnectionAuthorization.#slowDown(
                    reconciliation,
                    request,
                    seconds + SLOW_DOWN_SECONDS,
                );
            }
            answer = error;
        }

        // settle the authorization with the answer
        await this.#settle(reconciliation, row, request, answer);

        return undefined;
    }

    /** Send a person to an installation or authorization page in a browser. */
    async #startPage(
        context: MethodContextOf<typeof connection, "authorize", unknown>,
        target: Connection,
    ): Promise<AuthorizationStart> {
        // require a redirect URI
        const redirectUri = context.input.redirectUri;
        if (redirectUri === undefined) {
            throw ConnectionError.of(
                "REDIRECT_URI_REQUIRED",
                "authorizing in a browser needs a redirect URI",
            );
        }

        // send the person to an installation page with the state
        const { crypto, random } = context.environment;
        const state = random.bytes(STATE_BYTES).toHex();
        if (target.protocol.kind === "installation") {
            const installed = await TokenSource.find(context, target.integration);
            requireRedirect(installed, redirectUri);
            const url = installationPage(installed, state).href;

            return { kind: "installation", url, state, redirectUri };
        }

        // require a registered redirect URI
        const registered =
            target.integration === null
                ? await this.#register(context, target, redirectUri)
                : undefined;
        const client =
            registered === undefined
                ? await this.#tokens.integration(context, target.integration)
                : {
                      integration: registered,
                      server: this.#tokens.server(registered, undefined, context.now),
                  };
        requireRedirect(client.integration, redirectUri);

        // send the person to the authorization page with the state and the PKCE challenge
        const verifier = Pkce.verifier(random);
        const url = client.server.authorizationUrl({
            state,
            challenge: await Pkce.challenge(verifier, crypto),
            redirectUri,
            scopes: target.scopes,
            ...(target.resource === null ? {} : { resource: target.resource }),
        });
        const created =
            registered === undefined
                ? null
                : integration.reference(registered.scope, registered.id);

        return { kind: "page", url: url.href, state, verifier, redirectUri, registered: created };
    }

    /** Start a device authorization (RFC 8628 3.1) through the integration's server. */
    async #startDevice(
        context: MethodContextOf<typeof connection, "authorize", unknown>,
    ): Promise<AuthorizationStart> {
        // ask the integration's server for a device code
        const target = context.target;
        const client = await this.#tokens.integration(context, target.integration);
        const device = await client.server.authorizeDevice(
            target.scopes,
            target.resource ?? undefined,
        );

        return { kind: "device", ...device };
    }

    /** Register the client of a connection without an integration. */
    async #register(
        context: MethodContextOf<typeof connection, "authorize", unknown>,
        target: Connection,
        redirectUri: string,
    ): Promise<Integration> {
        // read the resource whose server registers the client
        const resource = present(
            target.resource,
            "the resource of a connection without an integration",
        );

        // register at the scope that stores the integrations
        const placement = this.#tokens.isPrimary(context.chain)
            ? { scope: context.scope, vault: target.vault }
            : {
                  scope: present(context.chain[1], `the scope enclosing ${context.scope}`),
                  vault: null,
              };
        const request = { resource, redirectUri, vault: placement.vault };

        return this.#tokens.register(placement.scope, request, context);
    }

    /** Keep a longer device poll interval for this and every later poll (RFC 8628 3.5). */
    static async #slowDown(
        reconciliation: ObjectReconciliation<typeof connection>,
        request: AuthorizationRequest,
        seconds: number,
    ): Promise<number> {
        const runner = Runner.of(reconciliation.store);
        await runner.mutate(runner.self, [
            authorizationRequest.call("update", {
                ...authorizationRequest.target(request),
                interval: seconds,
            }),
        ]);

        return Duration.milliseconds({ seconds });
    }

    /** Use up a device authorization's request, then settle the connection. */
    async #settle(
        reconciliation: ObjectReconciliation<typeof connection>,
        row: Connection,
        request: AuthorizationRequest,
        answer: Token | FamilyFailure<"AUTHORIZATION_REFUSED", OAuthRefusal>,
    ): Promise<void> {
        // use the request up
        const runner = Runner.of(reconciliation.store);
        await runner.mutate(runner.self, [
            authorizationRequest.call("delete", authorizationRequest.target(request)),
        ]);

        // record the refusal (RFC 8628 3.5)
        if (ConnectionError.is(answer, "AUTHORIZATION_REFUSED")) {
            const reason = ConnectionAuthorization.#refusal(OAuthRefusal.code(answer));
            await this.#tokens.observe(
                runner,
                row,
                { status: "false", reason, message: answer.message },
                {},
            );
        }
        // store the token and mark the connection authorized
        else {
            await this.#tokens.write(runner, row, { token: answer });
            await this.#tokens.observe(
                runner,
                row,
                { status: "true", reason: "Authorized", message: "" },
                {
                    expiresAt: answer.expiresAt ?? null,
                    scopes: answer.scopes ?? row.scopes,
                    storedAt: runner.now,
                    failures: 0,
                },
            );
        }
    }

    /** Read the request fields a page, installation or device authorization start stores. */
    static #request(prepared: AuthorizationStart) {
        // store the page's state
        if (prepared.kind === "page") {
            const { kind, state, redirectUri, verifier } = prepared;

            return { kind, state, redirectUri, verifier };
        }
        // store an installation's state and redirect
        else if (prepared.kind === "installation") {
            const { kind, state, redirectUri } = prepared;

            return { kind, state, redirectUri };
        }
        // store a device's code and interval
        else {
            const { kind, deviceCode, interval } = prepared;

            return { kind, deviceCode, interval };
        }
    }

    /** Read the Authorized reason of a refused device poll (RFC 8628 3.5). */
    static #refusal(error: string | undefined): "Denied" | "Expired" | "Failing" {
        // the person denied it
        if (error === "access_denied") {
            return "Denied";
        }
        // restart after an expired device code
        else if (error === "expired_token") {
            return "Expired";
        }
        // any other refusal is a failure
        else {
            return "Failing";
        }
    }
}

/** Require an authorization response's issuer to be the integration's (RFC 9207 2.4). */
function requireIssuer(named: Integration, issuer: string | undefined): void {
    const url = IntegrationIssuer.url(named.issuer);
    if (
        (issuer !== undefined && issuer !== url) ||
        (issuer === undefined && named.endpoints.isIssuerReturned)
    ) {
        throw ConnectionError.of(
            "AUTHORIZATION_INVALID",
            `expected issuer ${url} in the authorization response, got ${issuer ?? "none"}`,
        );
    }
}

/** Refuse an authorization server's error redirect (RFC 6749 4.1.2.1). */
function refuseError(parameters: Readonly<Record<string, string>>): void {
    if (parameters["error"] !== undefined) {
        throw ConnectionError.of(
            "AUTHORIZATION_REFUSED",
            parameters["error_description"] ?? parameters["error"],
            { data: { error: parameters["error"] } },
        );
    }
}

/** Require a redirect URI an integration registered. */
function requireRedirect(registered: Integration, redirectUri: string): void {
    if (!registered.redirectUris.includes(redirectUri)) {
        throw ConnectionError.of(
            "REDIRECT_URI_UNREGISTERED",
            `integration ${registered.clientId} has no registered redirect URI ${redirectUri}`,
        );
    }
}

/** Write an integration's installation page, returning to the redirect with the state. */
function installationPage(installed: Integration, state: string): URL {
    // require the installation page and return to it with the state
    const page = installed.endpoints.installation;
    if (page === undefined) {
        throw ConnectionError.of(
            "INSTALLATION_PAGE_MISSING",
            `integration ${installed.clientId} has no installation page`,
        );
    }
    const url = new URL(page);
    url.searchParams.set("state", state);

    return url;
}

/** Read the person and session calling, refusing any other caller. */
function requirePerson(
    context: MethodContextOf<typeof connection, "authorize" | "complete", unknown>,
): {
    readonly userId: string;
    readonly session: string;
} {
    // require a person calling in a signed-in session
    const { subject, credential } = context.caller;
    if (!principal.user.is(subject) || credential?.kind !== "session") {
        throw ConnectionError.of(
            "SESSION_REQUIRED",
            "a person must authorize a connection in a signed-in session",
        );
    }

    return { userId: subject.id, session: credential.id };
}

/** Find a connection's pending browser authorization by its state. */
async function findRequest(
    context: MethodContextOf<typeof connection, "complete", unknown>,
    target: Connection,
    state: string | undefined,
): Promise<AuthorizationRequest> {
    const [request] =
        state === undefined
            ? []
            : await context
                  .reveal(authorizationRequest)
                  .findMany({ where: { parentId: target.id, state } });
    if (request === undefined) {
        throw ConnectionError.of(
            "AUTHORIZATION_NOT_FOUND",
            "no pending authorization has this state",
        );
    }

    return request;
}
