import { AccessToken } from "@destack/access/token";
import { decodeJwt, type JWTPayload, UnsecuredJWT } from "jose";

/** JSON Web Tokens as a test reads and writes them. */
export const TestJwt = {
    /** Read the claims of an access token the issuer signed, unverified. */
    claims(token: string): AccessToken {
        return AccessToken.parse(decodeJwt(token));
    },

    /** Write an unsigned token of some claims, as a provider's test double answers it. */
    encode(claims: JWTPayload): string {
        return new UnsecuredJWT(claims).encode();
    },
};
