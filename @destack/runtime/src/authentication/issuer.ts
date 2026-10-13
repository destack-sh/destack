import { schema } from "@destack/schema";
import type { JWTPayload } from "jose";
import { Caller } from "@destack/access";
import { type AccessTokenClaims, TokenIdentity, TokenIssuerAuthority } from "./token.ts";
import { CallError } from "../error/index.ts";

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
            throw new CallError("UNAUTHORIZED", {
                message: "an anonymous caller gets no token",
            });
        }
        const expiresAt = Math.floor(credential.expiresAt / 1000);
        const issuedAt = Math.floor(credential.verifiedAt / 1000);
        if (expiresAt * 1000 <= now || expiresAt <= issuedAt) {
            throw new CallError("UNAUTHORIZED", {
                message: "the caller's verification expired",
            });
        }

        // encode the verified identity and the delegation it carries
        TokenIssuerAuthority.require(this.options.authority, caller);
        const identity = TokenIdentity.parse(
            schema.defined({
                subject: caller.subject,
                subjects: caller.subjects,
                assurance: caller.assurance,
                contacts: caller.contacts,
                deployments: caller.deployments,
                attributes: caller.attributes,
                credential: { kind: credential.kind, id: credential.id },
            }),
        );
        const { delegates = [], grant } = caller;

        // sign the registered claims with the caller
        const accessToken = await this.options.sign({
            iss: this.options.issuer,
            sub: caller.subject.id,
            aud: credential.audience,
            iat: issuedAt,
            exp: expiresAt,
            jti: crypto.randomUUID(),
            token_use: "access",
            ...(delegates.length === 0 ? {} : { act: [...delegates] }),
            ...(grant === undefined ? {} : { grant }),
            ...(credential.client === undefined ? {} : { cnf: { client: credential.client } }),
            ...(credential.scope === undefined
                ? {}
                : { space: schema.identifier("space").parse(credential.scope) }),
            caller: identity,
        } satisfies AccessTokenClaims);

        return { accessToken, tokenType: "Bearer" as const, expiresAt: expiresAt * 1000 };
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
