import type { JWTPayload } from "jose";
import { Caller, CALLER_LIFETIME_MILLISECONDS } from "./caller.ts";
import { TokenAuthentication, type TokenIssuerAuthority } from "./token.ts";
import { ServiceError } from "../error/index.ts";

/** Issue access tokens for verified callers. */
export class TokenIssuer {
    /** The signing configuration. */
    readonly options: TokenIssuerOptions;

    /** Create the issuer. */
    constructor(options: TokenIssuerOptions) {
        this.options = options;
    }

    /** Sign an access token for a caller within its verified lifetime. */
    async issue(caller: Caller<{ kind: string; id: string }>, now = Date.now()) {
        // keep the original verification deadline
        const current = caller.authentication;
        caller.requireCurrent(current.audience, now, current.scope);
        const expiresAt = Math.floor(
            Math.min(current.expiresAt, current.verifiedAt + CALLER_LIFETIME_MILLISECONDS) / 1000,
        );
        const issuedAt = Math.floor(current.verifiedAt / 1000);
        if (expiresAt * 1000 <= now || expiresAt <= issuedAt) {
            throw new ServiceError("UNAUTHORIZED");
        }

        // encode the verified identity fields
        const claims = TokenAuthentication.parse({
            spaceId: current.scope,
            credential: current.credential,
            subject: current.subject,
            subjects: current.subjects,
            assurance: current.assurance,
            identifiers: current.identifiers,
            delegates: current.delegates,
            deployments: current.deployments,
            permissions: current.permissions,
            attributes: current.attributes,
        });
        caller.requireAuthority(this.options.authority);

        // sign the registered claims with the caller
        const accessToken = await this.options.sign({
            iss: this.options.issuer,
            sub: current.subject.id,
            aud: current.audience,
            iat: issuedAt,
            exp: expiresAt,
            jti: crypto.randomUUID(),
            token_use: "access",
            caller: claims,
        });

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
