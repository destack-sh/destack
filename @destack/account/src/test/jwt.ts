import { AccessToken } from "@destack/access/token";
import { Base64Url, Jwt, type JwtClaims } from "@destack/identity";

/** JSON Web Tokens as a test reads and writes them. */
export const TestJwt = {
    /** Read the claims of an access token the issuer signed, unverified. */
    claims(token: string): AccessToken {
        return AccessToken.parse(Jwt.claims(token));
    },

    /** Write an unsigned token of some claims, as a provider's test double answers it. */
    encode(claims: JwtClaims): string {
        return `${Base64Url.encode('{"alg":"none"}')}.${Base64Url.encode(JSON.stringify(claims))}.`;
    },
};
