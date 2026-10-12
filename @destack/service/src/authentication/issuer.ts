import { schema } from "@destack/schema";
import type { JWTPayload } from "jose";
import type { PackageId } from "@destack/package";
import { Caller, principal } from "@destack/access";
import type { Subject } from "@destack/sync";
import { EXCHANGE_LIFETIME_MILLISECONDS, TokenCaller, TokenIssuerAuthority } from "./token.ts";
import { ServiceError } from "../error/index.ts";

/** Issue access tokens for verified callers. */
export class TokenIssuer {
    /** The signing configuration. */
    readonly options: TokenIssuerOptions;

    /** Create the issuer. */
    constructor(options: TokenIssuerOptions) {
        this.options = options;
    }

    /** Sign an access token for a caller until its credential lapses. */
    async issue(caller: Caller, now: number) {
        // require a principal, keeping the original verification and lapse
        const credential = Caller.requireCredential(caller);
        Caller.requireCurrent(caller, credential.audience, now, credential.scope);
        if (caller.subject === undefined) {
            throw new ServiceError("UNAUTHORIZED", {
                message: "an anonymous caller gets no token",
            });
        }
        const expiresAt = Math.floor(credential.expiresAt / 1000);
        const issuedAt = Math.floor(credential.verifiedAt / 1000);
        if (expiresAt * 1000 <= now || expiresAt <= issuedAt) {
            throw new ServiceError("UNAUTHORIZED", {
                message: "the caller's verification expired",
            });
        }

        // encode the verified identity fields
        const claims = TokenCaller.parse(
            schema.defined({
                spaceId: credential.scope,
                credential: schema.defined({
                    kind: credential.kind,
                    id: credential.id,
                    client: credential.client,
                }),
                subject: caller.subject,
                subjects: caller.subjects,
                assurance: caller.assurance,
                contacts: caller.contacts,
                delegates: caller.delegates,
                deployments: caller.deployments,
                permissions: caller.permissions,
                attributes: caller.attributes,
            }),
        );
        TokenIssuerAuthority.require(this.options.authority, caller);

        // sign the registered claims with the caller
        const accessToken = await this.options.sign({
            iss: this.options.issuer,
            sub: caller.subject.id,
            aud: credential.audience,
            iat: issuedAt,
            exp: expiresAt,
            jti: crypto.randomUUID(),
            token_use: "access",
            caller: claims,
        });

        return { accessToken, tokenType: "Bearer" as const, expiresAt: expiresAt * 1000 };
    }

    /** Exchange a person's verified call for a token that lends their authority (RFC 8693). */
    async exchange(
        caller: Caller,
        exchanged: {
            /** The installation the person called, acting with their authority. */
            readonly actor: Subject;
            /** The package receiving the calls the token authenticates. */
            readonly audience: PackageId;
        },
        now: number,
    ): Promise<string> {
        // require a current person calling directly
        const { authority } = this.options;
        const credential = Caller.requireCredential(caller);
        Caller.requireCurrent(caller, credential.audience, now, credential.scope);
        if (
            authority.kind !== "space" ||
            caller.subject === undefined ||
            !principal.user.is(caller.subject) ||
            (caller.delegates ?? []).length > 0 ||
            !principal.installation.is(exchanged.actor)
        ) {
            throw new ServiceError("FORBIDDEN", { message: "only a person's call is exchanged" });
        }

        // lend the person's identity to the installation for an hour
        const lent: Caller = {
            subject: caller.subject,
            subjects: caller.subjects,
            delegates: [{ subject: exchanged.actor, authority: "lent" }],
            ...(caller.permissions === undefined ? {} : { permissions: caller.permissions }),
            ...(caller.assurance === undefined ? {} : { assurance: caller.assurance }),
            credential: {
                ...credential,
                audience: exchanged.audience,
                scope: authority.space,
                verifiedAt: now,
                expiresAt: now + EXCHANGE_LIFETIME_MILLISECONDS,
            },
        };

        return (await this.issue(lent, now)).accessToken;
    }
}

/** The configuration of a token issuer. */
export interface TokenIssuerOptions {
    /** The issuer URL. */
    readonly issuer: string;
    /** The identity authority of this signer. */
    readonly authority: TokenIssuerAuthority;
    /** Sign with the authority's active ES256 key. */
    readonly sign: (payload: JWTPayload) => Promise<string>;
}
