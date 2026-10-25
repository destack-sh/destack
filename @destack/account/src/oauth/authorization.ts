import { Pkce } from "@destack/access/token";
import { type DatabaseConnection, Snapshot } from "@destack/db";
import type { Environment } from "@destack/environment";
import { CALL_ERROR_STATUSES, Failure } from "@destack/error";
import type { ObjectStore } from "@destack/object/server";
import { schema } from "@destack/schema";
import { RequestId, Scope } from "@destack/sync";
import { OAuthError, SECOND_MILLISECONDS } from "../authentication/token.ts";
import { AccountAuthentication } from "../better-auth/authentication.ts";
import { ActiveSession } from "../better-auth/active-session.ts";
import { alert, render } from "../better-auth/markup.ts";
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
import { spaceSeparated, type TokenGrant } from "./token.ts";
import { Credential } from "../credential/credential.ts";

/** The prefix of a stored request's reference (RFC 9126 2.2). */
const REQUEST_URI = "urn:ietf:params:oauth:request_uri:";

/** How long a stored request waits for the person: ten minutes to sign in and consent. */
const REQUEST_MILLISECONDS = 10 * 60_000;

/** How long a code waits for its redemption: a minute, as the client redeems it at once. */
const CODE_MILLISECONDS = 60_000;

/** A person's answer to a client's request: approve it or deny it. */
export const Decision = schema.enum(["approve", "deny"]);
/** A person's answer to a client's request. */
export type Decision = schema.Infer<typeof Decision>;

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
    /** The longest age of the person's sign-in, in seconds, at most a year. */
    max_age: schema
        .string()
        .regex(/^\d+$/u)
        .transform((seconds) => Number(seconds))
        .pipe(schema.number().int().max(MAX_AGE_SECONDS))
        .exactOptional(),
});

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
    async answer(request: Request): Promise<Response> {
        // find the client and the request, answering a refusal before the redirect as a page
        const query = new URL(request.url).searchParams;
        const asked = await this.#read(query).catch(refusalPage);
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

        return {
            request,
            party,
            session,
            isLoginDue: session === undefined || this.#isLoginDue(request, session),
        };
    }

    /** Decide a stored request as its person, answering the redirect with a code or a denial. */
    async decide(requestUri: string, headers: Headers, decision: Decision): Promise<string> {
        // refuse a decision without a session that meets the request
        const { request, party, session, isLoginDue } = await this.review(requestUri, headers);
        if (session === undefined || isLoginDue) {
            throw AccountError.of("SIGN_IN_REQUIRED", "the person is not signed in");
        }

        // return a denial to the client
        if (decision === "deny") {
            await this.#spend(request);

            return this.#location(request, {
                error: "access_denied",
                error_description: "the person denied access",
            });
        }

        // register a device's client for the person, grant the client the scopes and return the code
        const { objects, environment } = this.#options;
        const now = environment.clock.now();
        if (party.registration === undefined) {
            await Client.register(objects, party.clientId, session.userId, now);
        }
        const consent = await OAuthConsent.grant(
            objects,
            { clientId: party.clientId, userId: session.userId, scopes: request.scopes },
            now,
        );

        return this.#issue(request, consent, session);
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

        // check a new request's redirect, then store the request, returning its refusal to the redirect
        const redirectUri = query.get("redirect_uri");
        if (redirectUri === null) {
            throw OAuthError.of("INVALID_REQUEST", "the request names no redirect_uri");
        }
        party.requireRedirect(redirectUri);
        const stored = await this.#push(query, party, redirectUri).catch((error: unknown) =>
            this.#refuse({ redirectUri, state: query.get("state") }, error),
        );

        return stored instanceof Response ? stored : { party, stored };
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
        const isConsentDue =
            consent === undefined ||
            stored.prompts.includes("consent") ||
            stored.scopes.some((scope) => !consent.scopes.includes(scope));

        // refuse to interact when the client asked for no prompt (OIDC Core 3.1.2.6)
        if (stored.prompts.includes("none") && isLoginDue) {
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
        else if (isConsentDue || consent === undefined) {
            return found(this.#page(this.#options.consent, stored));
        }
        // return the code of the standing consent
        else {
            return found(await this.#issue(stored, consent, session));
        }
    }

    /** Store a new request from its parameters, refusing a request the client may not make. */
    async #push(
        query: URLSearchParams,
        party: OAuthClient,
        redirectUri: string,
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
                    expiresAt: environment.clock.now() + REQUEST_MILLISECONDS,
                }),
            ],
        });

        return this.#stored(`${REQUEST_URI}${created.id}`);
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
    async #spend(stored: AuthorizationRequest): Promise<void> {
        const { objects, environment } = this.#options;
        await objects.mutate(objects.as(authorizationRequest), {
            id: RequestId.create(environment),
            calls: [authorizationRequest.call("delete", authorizationRequest.target(stored))],
        });
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

    /** Spend a request on a code of its consent, answering the client's redirect with the code. */
    async #issue(
        stored: AuthorizationRequest,
        consent: OAuthConsent,
        session: ActiveSession,
    ): Promise<string> {
        // draw the code
        const { objects, environment } = this.#options;
        const code = await Credential.create("authorization-code", environment);

        // store its digest in the person's scope, then spend the request
        await objects.mutate(objects.as(authorizationCode), {
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
        });
        await this.#spend(stored);

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

/** Redeem an authorization code for the person's tokens, with its verifier (RFC 6749 4.1.3). */
export function codeGrant(
    options: Pick<AuthorizationOptions, "database" | "objects" | "environment">,
): TokenGrant {
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

        return {
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
        };
    };
}

/** Send the browser on to a URL. */
function found(location: string): Response {
    return new Response(null, { status: FOUND, headers: { location } });
}

/** Answer a request refused before its redirect is known as a page (RFC 6749 4.1.2.1). */
function refusalPage(error: unknown): Promise<Response> {
    // pass on a failure that is no OAuth refusal
    if (!OAuthError.is(error)) {
        throw error;
    }
    const { code, message } = Failure.of(error);

    return render("Authorization failed", alert(message), CALL_ERROR_STATUSES[code]);
}
