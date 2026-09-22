import type { JWTPayload } from "jose";
import { Caller, CALLER_LIFETIME_MS } from "./caller.ts";
import {
    TokenAuthentication,
    verifyTokenAuthentication,
    type TokenIssuerAuthority,
} from "./token.ts";
import { ServiceError } from "../error/index.ts";

/** Issue bounded access tokens from authority-verified caller records. */
export class TokenIssuer {
    /** Trusted signing configuration, supplied only by the hosting authority. */
    readonly options: TokenIssuerOptions;

    /** Attach the authority's signer and permitted identity scope. */
    constructor(options: TokenIssuerOptions) {
        this.options = options;
    }

    /** Sign an authenticated caller without extending its identity or delegation lifetime. */
    async issue(caller: Caller<{ kind: string; id: string }>, now = Date.now()) {
        // retain the original verification deadline through repeated exchanges
        const current = caller.authentication;
        caller.context(current.audience, now, current.scope);
        const expiresAt = Math.floor(
            Math.min(
                current.expiresAt,
                current.verifiedAt + CALLER_LIFETIME_MS,
                ...(current.delegations ?? []).map((delegation) => delegation.expiresAt),
            ) / 1000,
        );
        const issuedAt = Math.floor(current.verifiedAt / 1000);
        if (expiresAt * 1000 <= now || expiresAt <= issuedAt) {
            throw new ServiceError("UNAUTHORIZED");
        }

        // encode only the verified identity fields supported by receiving services
        const claims = TokenAuthentication.parse({
            spaceId: current.scope,
            credential: current.credential,
            subject: current.subject,
            subjects: current.subjects,
            actor: current.actor,
            delegations: current.delegations,
            deployments: current.deployments,
            memberships: current.memberships ?? [],
            permissions: current.permissions,
            attributes: current.attributes,
        });
        verifyTokenAuthentication(
            { ...current, expiresAt: expiresAt * 1000 },
            this.options.authority,
            now,
        );

        // sign registered token claims together with the authenticated caller
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

/** Server-only signing configuration; private keys remain with the authority. */
export interface TokenIssuerOptions {
    /** Exact issuer configured at receiving services. */
    readonly issuer: string;
    /** Explicit identity authority assigned to this signer. */
    readonly authority: TokenIssuerAuthority;
    /** Sign with the authority's active ES256 key and include its key identifier. */
    readonly sign: (payload: JWTPayload) => Promise<string>;
}
