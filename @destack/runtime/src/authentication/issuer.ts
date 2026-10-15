import type { JWTPayload } from "jose";
import { type Authentication, Caller, Credential } from "@destack/access";
import { PackageId } from "@destack/package";
import { ACCESS_TOKEN_TYPE, AccessToken, TokenIdentity, TokenIssuerAuthority } from "./token.ts";
import { CallError } from "../error/index.ts";

/** Issue access tokens for verified callers. */
export class TokenIssuer {
    /** The signing configuration. */
    readonly options: TokenIssuerOptions;

    /** Create the issuer. */
    constructor(options: TokenIssuerOptions) {
        this.options = options;
    }

    /** Sign an access token for a caller until the credential it presented lapses. */
    async issue(authentication: Authentication, now: number) {
        // require an identified caller, keeping the original verification and lapse
        const { caller, credential } = authentication;
        Credential.requireCurrent(credential, credential.audience, now, credential.scope);
        if (Caller.isAnonymous(caller)) {
            throw new CallError("UNAUTHORIZED", {
                message: "an anonymous caller cannot get a token",
            });
        }
        const expiresAt = Math.floor(credential.expiresAt / 1000);
        const issuedAt = Math.floor(credential.verifiedAt / 1000);
        if (expiresAt * 1000 <= now || expiresAt <= issuedAt) {
            throw new CallError("UNAUTHORIZED", {
                message: "the caller's verification expired",
            });
        }

        // require the caller's claims within the issuer's authority
        TokenIssuerAuthority.require(this.options.authority, authentication);

        // issue to the caller's client, else to the sending principal itself
        const sender = Caller.sender(caller);
        const client = AccessToken.shape.client_id.safeParse(credential.client ?? sender.id);
        if (!client.success) {
            throw new CallError("UNAUTHORIZED", { message: "a user's token needs a client" });
        }

        // sign the registered claims with the caller
        const claims = {
            iss: this.options.issuer,
            sub: caller.subject.id,
            aud: PackageId.resource(credential.audience),
            client_id: client.data,
            iat: issuedAt,
            exp: expiresAt,
            jti: crypto.randomUUID(),
            ...TokenIdentity.claims(authentication),
        } satisfies AccessToken;
        const accessToken = await this.options.sign(claims, { typ: ACCESS_TOKEN_TYPE });

        return { accessToken, tokenType: "Bearer" as const, expiresAt: expiresAt * 1000 };
    }
}

/** The configuration of a token issuer. */
export interface TokenIssuerOptions {
    /** The issuer URL. */
    readonly issuer: string;
    /** The identity authority of this signer. */
    readonly authority: TokenIssuerAuthority;
    /** Sign claims under a header's type with the authority's active ES256 key. */
    readonly sign: (payload: JWTPayload, header: { readonly typ: string }) => Promise<string>;
}
