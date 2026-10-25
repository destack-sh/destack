import { Uuid } from "@destack/schema";
import { type Environment } from "@destack/environment";
import { type Authentication, Caller, principal, universe } from "@destack/access";
import type { DatabaseConnection } from "@destack/db";
import { PackageId } from "@destack/package";
import { type Fetch, SpaceMount } from "@destack/object";
import {
    ASSERTION_TYPE,
    SPACE_KEY,
    TOKEN_LIFETIME_MILLISECONDS,
    type ExchangedToken,
    TokenCache,
    TokenIssuer,
} from "@destack/access/token";
import { Issuer, type TokenBinding } from "../native/issuer.ts";
import accountPackage from "../package.ts";
import type { Subject } from "@destack/sync";
import { TOKEN_PATH } from "../authentication/path.ts";
import { TokenResponse, SECOND_MILLISECONDS } from "../authentication/token.ts";
import { JWT_BEARER } from "../oauth/bearer.ts";
import { ACCESS_TOKEN_TYPE } from "../oauth/exchange.ts";
import type { IdentityKeystore } from "./keystore.ts";
import { AccountError } from "../error/error.ts";

/** How long an assertion a space signs lives: a minute, one token request and its retries. */
const ASSERTION_SECONDS = 60;

/** The credential kind of a token the universe's key signs for the universe. */
const UNIVERSE_KEY = "universe-key";

/** The universe, a space or one of its installations calling out with tokens its identity signs. */
export class TokenSource {
    /** The keystore that stores the identity's keys. */
    readonly keystore: IdentityKeystore;
    /** The connection reading the identity's keys. */
    readonly database: DatabaseConnection;
    /** The calling universe, space or installation. */
    readonly subject: Subject;
    /** The identity whose keys sign: the universe, the space, or the installation's space. */
    readonly identity: string;
    /** The universe's issuer, granting tokens for assertions and issuing the universe's tokens. */
    readonly issuer: string;
    /** The fetch sending the calls. */
    readonly send: Fetch;
    /** The environment the calls run in. */
    readonly environment: Environment;
    /** The issuer of the subject's tokens, signed with its identity's key. */
    readonly #signing: TokenIssuer;
    /** The tokens the universe's issuer granted for the subject's assertions, by package. */
    readonly #granted = new TokenCache();

    /** Call out as a subject whose identity's keys a keystore stores. */
    constructor(
        keystore: IdentityKeystore,
        database: DatabaseConnection,
        subject: Subject,
        network: {
            readonly issuer: string;
            readonly send: Fetch;
            readonly environment: Environment;
        },
    ) {
        // store the subject, its identity's keys and its network
        this.keystore = keystore;
        this.database = database;
        this.subject = subject;
        this.identity = TokenSource.#identity(subject);
        this.issuer = network.issuer;
        this.send = network.send;
        this.environment = network.environment;

        // sign as the universe under its issuer, or as the space
        this.#signing = universe.is(subject)
            ? new TokenIssuer({
                  authority: { kind: "universe" },
                  issuer: network.issuer,
                  sign: (claims, header) => keystore.sign(database, this.identity, claims, header),
              })
            : keystore.issuer(database, this.identity);
    }

    /** Authenticate the subject calling a package in a space now, by its identity's key. */
    authentication(audience: PackageId, scope: string): Authentication {
        const now = this.keystore.environment.clock.now();

        return {
            caller: Caller.of(this.subject),
            credential: {
                kind: universe.is(this.subject) ? UNIVERSE_KEY : SPACE_KEY,
                id: this.identity,
                audience,
                scope,
                verifiedAt: now,
                expiresAt: now + TOKEN_LIFETIME_MILLISECONDS,
            },
        };
    }

    /** Create a fetch calling a package with tokens the subject signs. */
    fetchSigned(audience: PackageId, partition?: string): Fetch {
        return async (request) => {
            // bind the token to the partition, else to the space below the request's mount, refusing a request to no space
            const scope = partition ?? SpaceMount.parse(new URL(request.url).pathname)?.spaceId;
            if (scope === undefined) {
                throw AccountError.of(
                    "SPACE_REQUIRED",
                    `${this.subject.id} can only sign calls to spaces, not ${request.url}`,
                );
            }

            // send the request with a short token of only the subject for the package there
            const { accessToken } = await this.#signing.issue(
                this.authentication(audience, scope),
                this.keystore.environment.clock.now(),
                this.keystore.environment,
            );

            return this.send(TokenSource.#bearer(request, accessToken));
        };
    }

    /** Create a fetch calling a universe package with tokens granted for the subject. */
    fetchGranted(audience: PackageId): Fetch {
        return async (request) =>
            this.send(TokenSource.#bearer(request, await this.grantedToken(audience)));
    }

    /** Read the subject's token of a package the universe's issuer granted, reused a while. */
    grantedToken(audience: PackageId): Promise<string> {
        return this.#granted.token(audience, this.keystore.environment.clock.now(), () =>
            this.#grant(audience),
        );
    }

    /** Exchange the subject's token for a person's token of a package (RFC 8693). */
    async exchange(audience: PackageId, bound: TokenBinding): Promise<ExchangedToken> {
        const accountToken = await this.grantedToken(accountPackage.id);

        return new Issuer(this.issuer, this.send, this.environment).exchange(
            { token: accountToken, type: ACCESS_TOKEN_TYPE },
            audience,
            bound,
        );
    }

    /** Sign an assertion of the subject for a token endpoint (RFC 7523). */
    assertion(endpoint: string): Promise<string> {
        // sign a short assertion as the identity, for itself or its installation
        const issuedAt = Math.floor(this.keystore.environment.clock.now() / SECOND_MILLISECONDS);
        const claims = {
            iss: this.identity,
            sub: this.subject.id,
            aud: endpoint,
            iat: issuedAt,
            exp: issuedAt + ASSERTION_SECONDS,
            jti: Uuid.v4(this.keystore.environment.random),
        };

        return this.keystore.sign(this.database, this.identity, claims, { typ: ASSERTION_TYPE });
    }

    /** Ask the universe's issuer for the subject's token of a package with its assertion. */
    async #grant(audience: PackageId) {
        // post an assertion of the subject for the package at the universe, where its relations decide
        const endpoint = `${this.issuer}${TOKEN_PATH}`;
        const body = new URLSearchParams({
            grant_type: JWT_BEARER,
            assertion: await this.assertion(endpoint),
            resource: PackageId.resource(audience),
        });
        const response = await this.send(new Request(endpoint, { method: "POST", body }));
        if (!response.ok) {
            throw await TokenResponse.refusal(response);
        }

        return TokenResponse.granted(
            TokenResponse.parse(await response.json()),
            this.environment.clock.now(),
        );
    }

    /** Copy a request with a bearer token. */
    static #bearer(request: Request, token: string): Request {
        const signed = new Request(request);
        signed.headers.set("authorization", `Bearer ${token}`);

        return signed;
    }

    /** Read the identity signing for a subject: the universe, a space, or an installation's. */
    static #identity(subject: Subject): string {
        // sign as the universe or a space itself
        if (universe.is(subject) || principal.space.is(subject)) {
            return subject.id;
        }
        // sign as an installation's space
        else if (principal.installation.is(subject)) {
            return subject.scope;
        }
        // refuse anyone else
        else {
            throw AccountError.of(
                "SIGNER_REQUIRED",
                `${subject.type} ${subject.id} is not a universe or space signer`,
            );
        }
    }
}
