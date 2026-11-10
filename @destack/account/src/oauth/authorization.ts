import type { AuthContext } from "better-auth";
import { type AssertionVerifier, Pkce } from "@destack/access/token";
import { AccessContext, Caller, Grant, GrantPermission, principal } from "@destack/access";
import { grant } from "@destack/object";
import { Directory } from "../address/directory.ts";
import { space } from "../object/space.ts";
import { type DatabaseConnection, Snapshot, Transaction } from "@destack/db";
import type { Environment } from "@destack/environment";
import { Failure } from "@destack/error";
import type { ObjectStore } from "@destack/object/server";
import { schema } from "@destack/schema";
import { RequestId, Scope, Subject } from "@destack/sync";
import { OAuthError, SECOND_MILLISECONDS } from "../authentication/token.ts";
import {
    AuthorizationRequestDecision,
    AuthorizationRequestOutcome,
    AuthorizationRequestReview,
    type Decision,
    RequestedGrant,
    RequestedPermission,
} from "../authentication/request.ts";
import { AccountAuthentication } from "../better-auth/authentication.ts";
import { ActiveSession } from "../better-auth/active-session.ts";
import { AccountError } from "../error/error.ts";
import { AuthorizationCode, authorizationCode } from "../object/authorization-code.ts";
import {
    type AuthorizationRequest,
    authorizationRequest,
    Prompt,
} from "../object/authorization-request.ts";
import { Client } from "../object/client.ts";
import { OAuthConsent, oauthConsent } from "../object/oauth-consent.ts";
import { ClientAuthentication, OAuthClient } from "./party.ts";
import { PackageResource } from "./resource.ts";
import { spaceSeparated, type GrantType } from "./token.ts";
import { Credential } from "../credential/credential.ts";

/** The prefix of a stored request's reference (RFC 9126 2.2). */
const REQUEST_URI = "urn:ietf:params:oauth:request_uri:";

/** How long a stored request waits for the person: ten minutes to sign in and consent. */
const REQUEST_MILLISECONDS = 10 * 60_000;

/** How long a code waits for its redemption: a minute, as the client redeems it at once. */
const CODE_MILLISECONDS = 60_000;

/** The assurance level of each acr value a request may ask, NIST SP 800-63's AALs. */
const ACR_LEVELS: ReadonlyMap<string, number> = new Map([
    ["aal1", 1],
    ["aal2", 2],
    ["aal3", 3],
]);

/** The longest max_age a request may ask: a year in seconds, longer than any session lasts. */
const MAX_AGE_SECONDS = 365 * 24 * 60 * 60;

/** The status a browser follows to the next page of an authorization (RFC 9110 15.4.3). */
const FOUND = 302;

/** An authorization request's parameters beside the client and redirect (RFC 6749 4.1.1). */
const AuthorizationQuery = schema.looseObject({
    /** The flow, the authorization code's alone. */
    response_type: schema.string(),
    /** The asked scopes, space separated. */
    scope: schema.string().exactOptional(),
    /** The client's opaque value. */
    state: schema.string().min(1).exactOptional(),
    /** The client's value the ID token repeats. */
    nonce: schema.string().min(1).exactOptional(),
    /** The S256 challenge of the client's verifier. */
    code_challenge: schema.string().min(43).max(128),
    /** The challenge's method, S256 alone. */
    code_challenge_method: schema.string(),
    /** The package the tokens are for, as its resource. */
    resource: schema.string().min(1).exactOptional(),
    /** The prompts, space separated. */
    prompt: schema.string().exactOptional(),
    /** The assurance levels the sign-in meets, NIST's aal1 to aal3 (OIDC Core 3.1.2.1). */
    acr_values: schema.string().exactOptional(),
    /** The longest age of the person's sign-in, in seconds, at most a year. */
    max_age: schema
        .string()
        .regex(/^\d+$/u)
        .transform((seconds) => Number(seconds))
        .pipe(schema.number().int().max(MAX_AGE_SECONDS))
        .exactOptional(),
});

/** The authorization detail type of a grant an installation asks (RFC 9396 2). */
const GRANT_DETAIL = "destack_grant";

/** A request object's authorization details: the one grant an installation asks. */
const AuthorizationDetails = schema
    .array(RequestedGrant.extend({ type: schema.literal(GRANT_DETAIL) }))
    .length(1);

/** A code redemption: the code, its redirect and the verifier (RFC 6749 4.1.3, RFC 7636 4.5). */
const CodeRequest = schema.looseObject({
    /** The code. */
    code: schema.string().min(1),
    /** The redirect the code returned to. */
    redirect_uri: schema.string().min(1),
    /** The verifier the code's challenge commits to. */
    code_verifier: schema.string().min(43).max(128),
    /** The package asked for, the authorized one. */
    resource: schema.string().min(1).exactOptional(),
});

/** What the authorization endpoint reads and stores requests, consents and codes with. */
export interface AuthorizationOptions {
    /** The issuer's URL, which every answer to a client includes (RFC 9207). */
    readonly issuer: string;
    /** The account database. */
    readonly database: DatabaseConnection;
    /** The account objects, which store requests, consents, codes and devices' clients. */
    readonly objects: Pick<ObjectStore, "mutate" | "as" | "database" | "environment">;
    /** The clock, the randomness of codes and the WebCrypto digesting them. */
    readonly environment: Pick<Environment, "clock" | "crypto" | "random">;
    /** The sign-in page, which continues a request once the person signs in. */
    readonly signIn: URL;
    /** The consent page, which decides a request. */
    readonly consent: URL;
    /** Read the session a browser's request signs in, none for a signed-out browser. */
    readonly session: (headers: Headers) => Promise<string | undefined>;
    /** Verify the request objects installations' spaces sign (RFC 9101). */
    readonly requests: Pick<AssertionVerifier, "verify">;
}

/** A stored request a page shows: its client, and whether its person must sign in again. */
export interface AuthorizationReview {
    /** The stored request. */
    readonly request: AuthorizationRequest;
    /** The client asking. */
    readonly party: OAuthClient;
    /** The person's live session, absent while signed out. */
    readonly session: ActiveSession | undefined;
    /** Whether the person must sign in again before the request continues. */
    readonly isLoginDue: boolean;
    /** What the person granted the request's delegate before, none without a grant asked. */
    readonly granted: readonly GrantPermission[];
}

/** The authorization endpoint: send a person to sign in and consent, then back (RFC 6749 3.1). */
export class AuthorizationEndpoint {
    /** What the endpoint reads and stores with. */
    readonly #options: AuthorizationOptions;

    /** Answer authorization requests. */
    constructor(options: AuthorizationOptions) {
        this.#options = options;
    }

    /** Answer an authorization request: a sign-in, a consent, or the code at the client. */
    async fetch(request: Request): Promise<Response> {
        // find the client and the request, answering a refusal before the redirect as a page
        const query = new URL(request.url).searchParams;
        const asked = await this.#read(query).catch((error: unknown) => this.#refusal(error));
        if (asked instanceof Response) {
            return asked;
        }

        // continue the request, returning a refusal of it to the client
        const { party, stored } = asked;

        return this.#continue(party, stored, request.headers).catch((error: unknown) =>
            this.#refuse(stored, error),
        );
    }

    /** Read a stored request a page continues, with its client and its person's session. */
    async review(requestUri: string, headers: Headers): Promise<AuthorizationReview> {
        // read the request, its client and the person's session
        const request = await this.#stored(requestUri);
        const party = await OAuthClient.find(request.clientId, this.#options.database);
        const session = await this.#session(headers);
        const standing =
            session === undefined || request.grant === null
                ? undefined
                : await this.#standing(session, request.grant.delegate);

        return {
            request,
            party,
            session,
            isLoginDue: session === undefined || this.#isLoginDue(request, session),
            granted: standing?.permissions ?? [],
        };
    }

    /** Decide a stored request as its person, answering the redirect with a code or a denial. */
    async decide(requestUri: string, headers: Headers, decision: Decision): Promise<string> {
        // refuse a decision without a session that meets the request
        const { request, party, session, isLoginDue } = await this.review(requestUri, headers);
        if (session === undefined || isLoginDue) {
            throw AccountError.of("SIGN_IN_REQUIRED", "the person is not signed in");
        }

        // refuse approving for this sign-in alone a request with no grant, or one growing a standing grant
        const standing =
            request.grant === null
                ? undefined
                : await this.#standing(session, request.grant.delegate);
        if (decision === "approve-once" && (request.grant === null || standing !== undefined)) {
            throw OAuthError.of("INVALID_REQUEST", "only a first grant can last for this sign-in");
        }
        // return a denial to the client
        else if (decision === "deny") {
            await this.#spend(request);

            return this.#location(request, {
                error: "access_denied",
                error_description: "the person denied access",
            });
        }

        // register a device's client for the person, grant the client the scopes and return the code, in one commit
        const { objects, environment, database } = this.#options;
        const now = environment.clock.now();
        const code = await Credential.create("authorization-code", environment);

        return Transaction.run(database, async (transaction) => {
            // register a device's client, record the consent, grant what was asked, then issue the code
            if (party.registration.kind === "device") {
                await Client.register(objects, party.clientId, session.userId, now, transaction);
            }
            const consent = await OAuthConsent.grant(
                objects,
                {
                    clientId: party.clientId,
                    userId: session.userId,
                    scopes: request.scopes,
                    grant: request.grant === null ? undefined : request.grant,
                },
                now,
                transaction,
            );
            if (request.grant !== null) {
                await this.#grant(
                    request.grant,
                    session,
                    decision === "approve-once",
                    standing,
                    transaction,
                );
            }

            return this.#issue(request, consent, session, code, transaction);
        });
    }

    /** Report whether the person's standing grant covers what a request asks, true for none. */
    async #isGranted(stored: AuthorizationRequest, session: ActiveSession): Promise<boolean> {
        if (stored.grant === null) {
            return true;
        }
        const standing = await this.#standing(session, stored.grant.delegate);
        const asked = stored.grant.permissions.map((permission) =>
            RequestedPermission.permission(permission),
        );

        return GrantPermission.includes(standing?.permissions ?? [], asked);
    }

    /** Read the person's standing grant to a delegate: unexpired, and lasting or this sign-in's. */
    async #standing(session: ActiveSession, delegate: Subject): Promise<Grant | undefined> {
        // find the person's unexpired grant to the delegate
        const standing = await Grant.find(
            Snapshot.live(this.#options.database),
            { delegator: principal.user.reference(Scope.universe.id, session.userId), delegate },
            this.#options.environment.clock.now(),
        );

        // leave out a grant for another sign-in alone, which ended with it
        return standing?.sessionId === null || standing?.sessionId === session.id
            ? standing
            : undefined;
    }

    /** Grant a delegate what the person approved beside what they granted it before. */
    async #grant(
        asked: RequestedGrant,
        session: ActiveSession,
        isOnce: boolean,
        standing: Grant | undefined,
        transaction: Transaction,
    ): Promise<void> {
        // create the grant as the person, replacing the standing one with both's permissions
        const { objects, environment } = this.#options;
        const person = principal.user.reference(Scope.universe.id, session.userId);
        const permissions = asked.permissions.map((permission) =>
            RequestedPermission.permission(permission),
        );
        await objects.mutate(
            AccessContext.of(Caller.of(person), environment.clock.now()),
            {
                id: RequestId.create(environment),
                calls: [
                    grant.call("create", {
                        ...grant.in(session.userId),
                        delegate: asked.delegate,
                        permissions: GrantPermission.merge(
                            standing?.permissions ?? [],
                            permissions,
                        ),
                        ...(isOnce ? { sessionId: session.id, expiresAt: session.expiresAt } : {}),
                    }),
                ],
            },
            { transaction },
        );
    }

    /** Read a request's enabled client, and its stored or newly stored request. */
    async #read(
        query: URLSearchParams,
    ): Promise<{ readonly party: OAuthClient; readonly stored: AuthorizationRequest } | Response> {
        // find the enabled client
        const clientId = query.get("client_id");
        if (clientId === null) {
            throw OAuthError.of("INVALID_REQUEST", "the request names no client");
        }
        const party = await OAuthClient.find(clientId, this.#options.database);
        party.requireEnabled();

        // continue a stored request of the client
        const requestUri = query.get("request_uri");
        if (requestUri !== null) {
            const stored = await this.#stored(requestUri);
            if (stored.clientId !== clientId) {
                throw OAuthError.of("INVALID_REQUEST_URI", "the request is another client's");
            }

            return { party, stored };
        }

        // read an installation origin's request from the object its space signs (RFC 9101)
        const signed = query.get("request");
        if (party.registration.kind === "installation" && signed === null) {
            throw OAuthError.of("INVALID_REQUEST", "an installation's origin signs its requests");
        }
        const { parameters, grant: asked } =
            signed === null
                ? { parameters: query, grant: undefined }
                : await this.#unsign(signed, party);

        // check a new request's redirect, then store the request, returning its refusal to the redirect
        const redirectUri = parameters.get("redirect_uri");
        if (redirectUri === null) {
            throw OAuthError.of("INVALID_REQUEST", "the request names no redirect_uri");
        }
        party.requireRedirect(redirectUri);
        const stored = await this.#push(parameters, party, redirectUri, asked).catch(
            (error: unknown) =>
                this.#refuse({ redirectUri, state: parameters.get("state") }, error),
        );

        return stored instanceof Response ? stored : { party, stored };
    }

    /** Read a request object its client's space signed: parameters and grant (RFC 9101). */
    async #unsign(
        signed: string,
        party: OAuthClient,
    ): Promise<{
        readonly parameters: URLSearchParams;
        readonly grant: RequestedGrant | undefined;
    }> {
        // verify it as signed by a space for this issuer
        const { registration } = party;
        const { environment, database, requests } = this.#options;
        const { authentication, claims } = await requests
            .verify(signed, environment.clock.now())
            .catch((error: unknown) => {
                throw OAuthError.of("INVALID_REQUEST_OBJECT", "invalid request object", {
                    cause: error,
                });
            });

        // require the space of the client's own origin to sign it, for that client
        const signer = authentication.caller.subject;
        const signerSpace = principal.space.is(signer) ? signer.id : signer.scope;
        const named =
            registration.kind === "installation"
                ? await new Directory(Snapshot.live(database)).find(space, registration.space)
                : undefined;
        if (named?.id !== signerSpace || claims["client_id"] !== party.clientId) {
            throw OAuthError.of("INVALID_REQUEST_OBJECT", "the request object is another client's");
        }

        // read its parameters, and the one grant it asks for the signing installation
        const parameters = new URLSearchParams(
            Object.entries(claims).filter(
                (entry): entry is [string, string] => typeof entry[1] === "string",
            ),
        );
        const details = claims["authorization_details"];
        const asked = details === undefined ? undefined : AuthorizationDetails.safeParse(details);
        if (asked !== undefined && !asked.success) {
            throw OAuthError.of("INVALID_REQUEST_OBJECT", "invalid authorization_details");
        }
        const [detail] = asked?.data ?? [];
        if (detail !== undefined && !Subject.same(detail.delegate, signer)) {
            throw OAuthError.of(
                "INVALID_REQUEST_OBJECT",
                "a space signs grants for its own installation",
            );
        }
        // refuse a grant naming another installation or space than the origin, which the person reads
        else if (
            detail !== undefined &&
            registration.kind === "installation" &&
            (detail.space !== registration.space ||
                (registration.installation !== null &&
                    detail.installation !== registration.installation))
        ) {
            throw OAuthError.of(
                "INVALID_REQUEST_OBJECT",
                "the grant names another installation than the origin asking",
            );
        }
        const { type: _type, ...requested } = detail ?? {};

        return {
            parameters,
            grant: detail === undefined ? undefined : RequestedGrant.parse(requested),
        };
    }

    /** Continue a stored request: send the person to sign in or consent, else return the code. */
    async #continue(
        party: OAuthClient,
        stored: AuthorizationRequest,
        headers: Headers,
    ): Promise<Response> {
        // read the person's session and their consent to the client
        const session = await this.#session(headers);
        const consent =
            session === undefined
                ? undefined
                : await Snapshot.live(this.#options.database)
                      .read(oauthConsent.table)
                      .findFirst({ where: { clientId: party.clientId, scope: session.userId } });

        // decide what the request still needs
        const isLoginDue = session === undefined || this.#isLoginDue(stored, session);
        // ask an origin consent for a grant beyond the standing one, any other client for its scopes
        const isConsentDue =
            party.registration.kind === "installation"
                ? session !== undefined && !(await this.#isGranted(stored, session))
                : consent === undefined ||
                  stored.prompts.includes("consent") ||
                  stored.scopes.some((scope) => !consent.scopes.includes(scope));

        // refuse a level the person's enrolled methods never reach, which signing in again cannot meet
        const isUnreachable =
            session !== undefined &&
            stored.assurance !== null &&
            (await session.reachable(this.#options.database)) < stored.assurance;
        if (isUnreachable) {
            throw OAuthError.of(
                "UNMET_AUTHENTICATION_REQUIREMENTS",
                "add a second factor or a passkey to sign in this strongly",
            );
        }
        // refuse to interact when the client asked for no prompt (OIDC Core 3.1.2.6)
        else if (stored.prompts.includes("none") && isLoginDue) {
            throw OAuthError.of("LOGIN_REQUIRED", "the person must sign in");
        }
        // refuse to ask for consent when the client asked for no prompt
        else if (stored.prompts.includes("none") && isConsentDue) {
            throw OAuthError.of("CONSENT_REQUIRED", "the person must consent");
        }
        // send the person to sign in, then back to the request
        else if (isLoginDue || session === undefined) {
            return found(this.#page(this.#options.signIn, stored));
        }
        // send the person to consent to the request
        else if (isConsentDue) {
            return found(this.#page(this.#options.consent, stored));
        }
        // return the code of the standing consent, widening an installation origin's in the same commit
        else {
            const { database, environment, objects } = this.#options;
            const code = await Credential.create("authorization-code", environment);
            const location = await Transaction.run(database, async (transaction) => {
                const standing =
                    party.registration.kind === "installation" || consent === undefined
                        ? await OAuthConsent.grant(
                              objects,
                              {
                                  clientId: party.clientId,
                                  userId: session.userId,
                                  scopes: stored.scopes,
                                  grant: undefined,
                              },
                              environment.clock.now(),
                              transaction,
                          )
                        : consent;

                return this.#issue(stored, standing, session, code, transaction);
            });

            return found(location);
        }
    }

    /** Store a new request from its parameters, refusing a request the client may not make. */
    async #push(
        query: URLSearchParams,
        party: OAuthClient,
        redirectUri: string,
        requested: RequestedGrant | undefined,
    ): Promise<AuthorizationRequest> {
        // refuse another flow than the code's with S256 (RFC 7636 4.4.1)
        const parsed = AuthorizationQuery.safeParse(Object.fromEntries(query));
        if (!parsed.success) {
            throw OAuthError.of("INVALID_REQUEST", "invalid authorization request");
        }
        const asked = parsed.data;
        if (asked.response_type !== "code") {
            throw OAuthError.of("UNSUPPORTED_RESPONSE_TYPE", "the response_type is code alone");
        }
        // refuse another challenge method than S256
        else if (asked.code_challenge_method !== "S256") {
            throw OAuthError.of("INVALID_REQUEST", "the code_challenge_method is S256 alone");
        }

        // refuse prompts beside none, scopes the client may not ask for, and a resource of no package
        const prompts = Prompt.array().safeParse(spaceSeparated(asked.prompt));
        const scopes = spaceSeparated(asked.scope);
        party.requireScopes(scopes);
        if (!prompts.success || (prompts.data.includes("none") && prompts.data.length > 1)) {
            throw OAuthError.of("INVALID_REQUEST", "invalid prompt");
        }
        // refuse a resource of no package
        else if (
            asked.resource !== undefined &&
            PackageResource.package(asked.resource) === undefined
        ) {
            throw OAuthError.of("INVALID_TARGET", `${asked.resource} names no package`);
        }

        // read the lowest assurance level the acr values accept, refusing an unknown value
        const levels = spaceSeparated(asked.acr_values).map((value) => ACR_LEVELS.get(value));
        if (levels.some((level) => level === undefined)) {
            throw OAuthError.of("INVALID_REQUEST", "acr_values are aal1, aal2 or aal3");
        }
        const assurance =
            levels.length === 0 ? null : Math.min(...levels.flatMap((level) => level ?? []));

        // store the request until the person had time to sign in and consent
        const { objects, environment } = this.#options;
        const [created] = await objects.mutate(objects.as(authorizationRequest), {
            id: RequestId.create(environment),
            calls: [
                authorizationRequest.call("create", {
                    ...authorizationRequest.in(Scope.universe.id),
                    clientId: party.clientId,
                    redirectUri,
                    scopes,
                    state: asked.state ?? null,
                    nonce: asked.nonce ?? null,
                    codeChallenge: asked.code_challenge,
                    resource: asked.resource ?? null,
                    prompts: prompts.data,
                    maxAge:
                        asked.max_age === undefined ? null : asked.max_age * SECOND_MILLISECONDS,
                    assurance,
                    grant: requested ?? null,
                    expiresAt: environment.clock.now() + REQUEST_MILLISECONDS,
                }),
            ],
        });

        return this.#stored(`${REQUEST_URI}${created.id}`);
    }

    /** Send a request refused before its redirect exists to the sign-in page (RFC 6749 4.1.2.1). */
    #refusal(error: unknown): Response {
        // pass on a failure that is no OAuth refusal
        if (!OAuthError.is(error)) {
            throw error;
        }

        // show the refusal on the sign-in page, as Ory Hydra's error URL does
        const { reason, message } = Failure.of(error);
        const url = new URL(this.#options.signIn);
        url.searchParams.set("error", reason.toLowerCase());
        url.searchParams.set("error_description", message);

        return found(url.href);
    }

    /** Address a page continuing a stored request by its reference. */
    #page(page: URL, stored: AuthorizationRequest): string {
        // name the client and the stored request
        const url = new URL(page);
        url.searchParams.set("client_id", stored.clientId);
        url.searchParams.set("request_uri", `${REQUEST_URI}${stored.id}`);

        return url.href;
    }

    /** Read a stored request by its reference, refusing an unknown or lapsed one. */
    async #stored(requestUri: string): Promise<AuthorizationRequest> {
        // read the request the reference names
        const id = schema
            .identifier("authorization-request")
            .safeParse(requestUri.slice(REQUEST_URI.length));
        const row =
            requestUri.startsWith(REQUEST_URI) && id.success
                ? await Snapshot.live(this.#options.database)
                      .read(authorizationRequest.table)
                      .findFirst({ where: { id: id.data } })
                : undefined;
        if (row === undefined || row.expiresAt <= this.#options.environment.clock.now()) {
            throw OAuthError.of("INVALID_REQUEST_URI", "the authorization request lapsed");
        }

        return row;
    }

    /** Remove a decided request. */
    async #spend(stored: AuthorizationRequest, transaction?: Transaction): Promise<void> {
        const { objects, environment } = this.#options;
        await objects.mutate(
            objects.as(authorizationRequest),
            {
                id: RequestId.create(environment),
                calls: [authorizationRequest.call("delete", authorizationRequest.target(stored))],
            },
            transaction === undefined ? {} : { transaction },
        );
    }

    /** Read the live session a browser's request signs in. */
    async #session(headers: Headers): Promise<ActiveSession | undefined> {
        const id = await this.#options.session(headers);

        return id === undefined
            ? undefined
            : ActiveSession.read(id, this.#options.database, this.#options.environment.clock.now());
    }

    /** Report whether a person signed in before the request must sign in again. */
    #isLoginDue(stored: AuthorizationRequest, session: ActiveSession): boolean {
        const { authenticatedAt } = session;
        const now = this.#options.environment.clock.now();
        // require a sign-in of a session that never completed one
        if (authenticatedAt === null) {
            return true;
        }
        // require a sign-in below the level the request asks
        else if (stored.assurance !== null && (session.assurance?.level ?? 0) < stored.assurance) {
            return true;
        }
        // accept any sign-in after the request arrived, however short its longest age
        else if (authenticatedAt >= stored.createdAt) {
            return false;
        }
        // require a new sign-in for a login prompt, or past the longest age
        else {
            const isTooOld = stored.maxAge !== null && authenticatedAt < now - stored.maxAge;

            return stored.prompts.includes("login") || isTooOld;
        }
    }

    /** Spend a request on a drawn code of its consent, returning the client's redirect. */
    async #issue(
        stored: AuthorizationRequest,
        consent: OAuthConsent,
        session: ActiveSession,
        code: Awaited<ReturnType<typeof Credential.create>>,
        transaction: Transaction,
    ): Promise<string> {
        // store the code's digest in the person's scope, then spend the request
        const { objects, environment } = this.#options;
        await objects.mutate(
            objects.as(authorizationCode),
            {
                id: RequestId.create(environment),
                calls: [
                    authorizationCode.call("create", {
                        ...authorizationCode.in(consent.scope),
                        digest: code.digest,
                        consentId: consent.id,
                        sessionId: session.id,
                        redirectUri: stored.redirectUri,
                        scopes: stored.scopes,
                        nonce: stored.nonce,
                        codeChallenge: stored.codeChallenge,
                        resource: stored.resource,
                        status: "issued",
                        expiresAt: environment.clock.now() + CODE_MILLISECONDS,
                    }),
                ],
            },
            { transaction },
        );
        await this.#spend(stored, transaction);

        return this.#location(stored, { code: code.secret });
    }

    /** Return an OAuth refusal of a request to its redirect, passing on any other failure. */
    #refuse(stored: Pick<AuthorizationRequest, "redirectUri" | "state">, error: unknown): Response {
        // pass on a failure that is no OAuth refusal
        if (!OAuthError.is(error)) {
            throw error;
        }
        const { reason, message } = Failure.of(error);

        return found(
            this.#location(stored, { error: reason.toLowerCase(), error_description: message }),
        );
    }

    /** Address a request's redirect with an answer, its state and the issuer (RFC 9207). */
    #location(
        stored: Pick<AuthorizationRequest, "redirectUri" | "state">,
        answer: Readonly<Record<string, string>>,
    ): string {
        // add the answer, the client's state and the issuer
        const url = new URL(stored.redirectUri);
        for (const [name, value] of Object.entries(answer)) {
            url.searchParams.set(name, value);
        }
        if (stored.state !== null) {
            url.searchParams.set("state", stored.state);
        }
        url.searchParams.set("iss", this.#options.issuer);

        return url.href;
    }
}

/** The stored authorization requests a consent view reads and decides (Ory Hydra's). */
export class AuthorizationRequestEndpoint {
    /** The authorization endpoint storing and deciding the requests. */
    readonly #authorization: AuthorizationEndpoint;
    /** The origins a decision may come from. */
    readonly #context: Promise<Pick<AuthContext, "isTrustedOrigin">>;

    /** Mark each permission of a reviewed request's grant once the person granted it before. */
    static #marked(review: AuthorizationReview) {
        const asked = review.request.grant;

        return asked === null
            ? null
            : {
                  ...asked,
                  permissions: asked.permissions.map((permission) => ({
                      ...permission,
                      isGranted: GrantPermission.includes(review.granted, [
                          RequestedPermission.permission(permission),
                      ]),
                  })),
              };
    }

    /** Answer a consent view through the authorization endpoint. */
    constructor(
        authorization: AuthorizationEndpoint,
        context: Promise<Pick<AuthContext, "isTrustedOrigin">>,
    ) {
        this.#authorization = authorization;
        this.#context = context;
    }

    /** Read a stored request by its reference, or decide it as the signed-in person. */
    async fetch(request: Request): Promise<Response> {
        // read the request's client, scopes and whether to sign in first
        if (request.method === "GET") {
            const requestUri = requireQuery(request, "request_uri");
            const review = await this.#authorization.review(requestUri, request.headers);

            return Response.json(
                AuthorizationRequestReview.parse({
                    client: { id: review.party.clientId, name: review.party.name },
                    scopes: review.request.scopes,
                    isLoginDue: review.isLoginDue,
                    grant: AuthorizationRequestEndpoint.#marked(review),
                }),
            );
        }
        // decide it from a trusted origin, answering where the browser goes next
        else if (request.method === "POST") {
            await requireOrigin(request, this.#context);
            const { requestUri, decision } = await requireBody(
                request,
                AuthorizationRequestDecision,
            );
            const location = await this.#authorization.decide(
                requestUri,
                request.headers,
                decision,
            );

            return Response.json(AuthorizationRequestOutcome.parse({ location }));
        }

        return new Response(null, { status: 405, headers: { allow: "GET, POST" } });
    }
}

/** Require a decision posted from a trusted origin, refusing one another site sends. */
export async function requireOrigin(
    request: Request,
    context: Promise<Pick<AuthContext, "isTrustedOrigin">>,
): Promise<void> {
    const origin = request.headers.get("origin");
    if (origin === null || !(await context).isTrustedOrigin(origin)) {
        throw AccountError.of("ORIGIN_UNTRUSTED", `the decision came from ${origin ?? "nowhere"}`);
    }
}

/** Read a query parameter a request needs, refusing a request without it. */
export function requireQuery(request: Request, name: string): string {
    // refuse an absent or empty parameter
    const value = new URL(request.url).searchParams.get(name);
    if (value === null || value === "") {
        throw AccountError.of("QUERY_PARAMETER_MISSING", `the request has no ${name}`);
    }

    return value;
}

/** Read a request's JSON body by its schema, refusing one that is no JSON or does not match. */
export async function requireBody<Body>(
    request: Request,
    shape: schema.Schema<Body>,
): Promise<Body> {
    // read the body as JSON, an unreadable one failing the schema
    const body: unknown = await request.json().catch(() => undefined);
    const parsed = shape.safeParse(body);
    if (!parsed.success) {
        throw AccountError.of("BODY_INVALID", "the request's body does not match");
    }

    return parsed.data;
}

/** Redeem an authorization code for the person's tokens, with its verifier (RFC 6749 4.1.3). */
export function codeGrant(
    options: Pick<AuthorizationOptions, "database" | "objects" | "environment">,
): GrantType {
    return async ({ body, headers }) => {
        // authenticate the client and read the request
        const { database, environment } = options;
        const { party, asked } = await ClientAuthentication.grant(
            { body, headers },
            "authorization_code",
            CodeRequest,
            { database, crypto: environment.crypto },
        );

        // spend the code, then refuse another redirect than it was issued for
        const { row, consent } = await AuthorizationCode.redeem(
            asked.code,
            party.clientId,
            options,
        );
        const challenge = await Pkce.challenge(asked.code_verifier, environment.crypto);
        if (asked.redirect_uri !== row.redirectUri) {
            throw OAuthError.of("INVALID_GRANT", "redirect_uri does not match the code's");
        }
        // refuse a verifier its challenge does not commit to (RFC 7636 4.6)
        else if (challenge !== row.codeChallenge) {
            throw OAuthError.of("INVALID_GRANT", "the code verifier does not match its challenge");
        }
        // refuse another package than the authorized one
        else if (asked.resource !== undefined && asked.resource !== row.resource) {
            throw OAuthError.of("INVALID_TARGET", `${asked.resource} was not authorized`);
        }

        // describe the person through the session that consented, refusing one that ended
        const resource = row.resource ?? undefined;
        const sessionId = row.sessionId;
        if (sessionId === null) {
            throw OAuthError.of("INVALID_GRANT", "the session that consented ended");
        }
        const authentication = await AccountAuthentication.person(
            database,
            { userId: consent.scope, clientId: party.clientId, sessionId },
            PackageResource.audience(resource),
            environment.clock.now(),
        );

        return async () => ({
            authentication,
            clientId: party.clientId,
            scopes: row.scopes,
            ...(resource === undefined ? {} : { resource }),
            consent: {
                row: consent,
                family: row.id,
                sessionId,
                ...(row.nonce === null ? {} : { nonce: row.nonce }),
            },
        });
    };
}

/** Send the browser on to a URL. */
function found(location: string): Response {
    return new Response(null, { status: FOUND, headers: { location } });
}
