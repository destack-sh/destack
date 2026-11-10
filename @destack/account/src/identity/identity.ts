import { Bookmark } from "@destack/object/client";
import { schema, Uuid } from "@destack/schema";
import { type Environment } from "@destack/environment";
import { type Authentication, Caller, principal, universe } from "@destack/access";
import type { DatabaseConnection } from "@destack/db";
import { PackageId } from "@destack/package";
import { type Fetch, SpaceMount } from "@destack/object";
import {
    ASSERTION_TYPE,
    SPACE_KEY,
    TOKEN_LIFETIME_MILLISECONDS,
    TokenIssuer,
} from "@destack/access/token";
import { bearer, type Credential, IssuerCredential, type Signer } from "../client/credential.ts";
import { Issuer, type TokenBinding } from "../client/issuer.ts";
import accountPackage from "../package.ts";
import type { Subject } from "@destack/sync";
import { SECOND_MILLISECONDS, REQUEST_OBJECT_TYPE } from "../authentication/token.ts";
import { ACCESS_TOKEN_TYPE } from "../oauth/exchange.ts";
import type { GrantRequest } from "../authentication/request.ts";
import type { IdentityKeystore } from "./keystore.ts";
import { AccountError } from "../error/error.ts";

/** How long an assertion a space signs lives: a minute, one token request and its retries. */
const ASSERTION_SECONDS = 60;

/** The credential kind of a token the universe's key signs for the universe. */
const UNIVERSE_KEY = "universe-key";

/** The universe, a space or an installation, signing its tokens and assertions with its keys. */
export class Identity implements Credential, Signer {
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
    /** The subject's tokens the universe's issuer grants for its assertions. */
    readonly granted: IssuerCredential;

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
        this.identity = Identity.#identity(subject);
        this.issuer = network.issuer;
        this.send = network.send;
        this.environment = network.environment;
        this.granted = new IssuerCredential(
            this,
            network.issuer,
            network.send,
            network.environment,
        );

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

    /** Add a short token of only the subject, signed by its identity, bound to a space. */
    async authorize(
        request: Request,
        audience: PackageId,
        binding: TokenBinding,
    ): Promise<Request> {
        // sign the token for the package in the space the call goes to
        const { accessToken } = await this.#signing.issue(
            this.authentication(audience, this.#space(request, binding)),
            this.keystore.environment.clock.now(),
            this.keystore.environment,
        );

        return bearer(request, accessToken);
    }

    /** Act for a caller under the grant the subject asks, exchanging tokens (RFC 8693). */
    acting(caller: Pick<Caller, "subject" | "grants">, requested: GrantRequest): Credential {
        return {
            authorize: async (request, audience, binding) => {
                // exchange for the caller's token in the call's space, carrying the grant the subject asks
                const token = await this.exchange(audience, {
                    subject: caller.subject,
                    partition: schema.identifier("space").parse(this.#space(request, binding)),
                    ...(caller.grants.length === 0
                        ? {}
                        : { grants: caller.grants.map((grant) => grant.id) }),
                    requested,
                });

                return bearer(request, token);
            },
        };
    }

    /** Exchange the subject's token for a person's token of a package (RFC 8693). */
    async exchange(audience: PackageId, bound: TokenBinding): Promise<string> {
        // exchange the subject's account token at the universe's issuer
        const accountToken = await this.granted.token(accountPackage.id, {});
        const issuer = new Issuer(this.issuer, this.send, this.environment, new Bookmark());
        const exchanged = await issuer.exchange(
            { token: accountToken, type: ACCESS_TOKEN_TYPE },
            audience,
            bound,
        );

        return exchanged.accessToken;
    }

    /** Sign an assertion of the subject for a token endpoint (RFC 7523). */
    assert(endpoint: string): Promise<string> {
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

    /** Sign a request object of the subject for an issuer's authorization endpoint (RFC 9101). */
    request(issuer: string, parameters: Readonly<Record<string, unknown>>): Promise<string> {
        // sign the parameters as the identity, for itself or its installation, living as an assertion does
        const issuedAt = Math.floor(this.keystore.environment.clock.now() / SECOND_MILLISECONDS);
        const claims = {
            ...parameters,
            iss: this.identity,
            sub: this.subject.id,
            aud: issuer,
            iat: issuedAt,
            exp: issuedAt + ASSERTION_SECONDS,
            jti: Uuid.v4(this.keystore.environment.random),
        };

        return this.keystore.sign(this.database, this.identity, claims, {
            typ: REQUEST_OBJECT_TYPE,
        });
    }

    /** Read the space a call goes to: its partition, else the space below its mount. */
    #space(request: Request, binding: TokenBinding): string {
        const spaceId =
            binding.partition ?? SpaceMount.parse(new URL(request.url).pathname)?.spaceId;
        if (spaceId === undefined) {
            throw AccountError.of(
                "SPACE_REQUIRED",
                `${this.subject.id} can only sign calls to spaces, not ${request.url}`,
            );
        }

        return spaceId;
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
