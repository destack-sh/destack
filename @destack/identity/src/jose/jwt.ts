import type { Crypto } from "@destack/environment";
import { schema } from "@destack/schema";
import { IdentityError } from "../error/error.ts";
import { Base64Url } from "./base64.ts";
import type { JwsAlgorithm } from "./algorithm.ts";
import { Jws, type JwsHeader, type JwsKey, type KeyLookup } from "./jws.ts";

/** The registered claims of a JWT, beside any others (RFC 7519 4.1). */
export const JwtClaims = schema.looseObject({
    /** The issuer. */
    iss: schema.string().exactOptional(),
    /** The subject. */
    sub: schema.string().exactOptional(),
    /** The audience: one, or several. */
    aud: schema.union([schema.string(), schema.array(schema.string())]).exactOptional(),
    /** The expiry, in seconds since the epoch. */
    exp: schema.number().exactOptional(),
    /** The time before which it is not valid, in seconds since the epoch. */
    nbf: schema.number().exactOptional(),
    /** The issue time, in seconds since the epoch. */
    iat: schema.number().exactOptional(),
    /** The token's identifier. */
    jti: schema.string().exactOptional(),
});
/** The claims of a JWT. */
export type JwtClaims = schema.Infer<typeof JwtClaims>;

/** How a JWT verifies (RFC 7519 7.2, RFC 8725 3). */
export interface JwtOptions {
    /** The algorithms its signature may use. */
    readonly algorithms: readonly JwsAlgorithm[];
    /** The time of the check, in Unix milliseconds. */
    readonly now: number;
    /** The issuers it may come from. */
    readonly issuer?: string | readonly string[];
    /** The audiences, one of which it must name. */
    readonly audience?: string | readonly string[];
    /** The subject it must name. */
    readonly subject?: string;
    /** The type its header must declare. */
    readonly typ?: string;
    /** The claims it must have. */
    readonly requiredClaims?: readonly string[];
    /** The longest age since its issue time, in seconds. */
    readonly maxTokenAge?: number;
    /** The seconds its times may be off by. */
    readonly clockTolerance?: number;
}

/** What a verified JWT has: its protected header and its claims. */
export interface VerifiedJwt {
    /** The protected header. */
    readonly header: JwsHeader;
    /** The claims. */
    readonly claims: JwtClaims;
}

/** JSON Web Tokens: signed claims (RFC 7519). */
export const Jwt = {
    /** Sign claims under a protected header with a key. */
    sign(
        claims: JwtClaims,
        header: JwsHeader & { readonly alg: JwsAlgorithm },
        key: JwsKey,
        crypto: Crypto,
    ): Promise<string> {
        return Jws.sign(new TextEncoder().encode(JSON.stringify(claims)), header, key, crypto);
    },

    /** Verify a JWT's signature and claims at a time. */
    async verify(
        token: string,
        keys: JwsKey | KeyLookup,
        options: JwtOptions,
        crypto: Crypto,
    ): Promise<VerifiedJwt> {
        // verify the signature, then read the claims and check them
        const { header, payload } = await Jws.verify(token, keys, options.algorithms, crypto);
        const claims = parseClaims(Base64Url.json(payload));
        Jwt.check(header, claims, options);

        return { header, claims };
    },

    /** Read a JWT's claims without verifying it. */
    claims(token: string): JwtClaims {
        return parseClaims(Base64Url.json(Jws.payload(token)));
    },

    /** Check a JWT's header and claims at a time, refusing each failed check by its reason. */
    check(header: JwsHeader, claims: JwtClaims, options: JwtOptions): void {
        // require the declared type and the required claims
        const tolerance = options.clockTolerance ?? 0;
        const now = Math.floor(options.now / 1000);
        if (options.typ !== undefined && !sameType(header.typ, options.typ)) {
            throw IdentityError.of("CLAIM_INVALID", `the token's type is not ${options.typ}`);
        }
        for (const claim of options.requiredClaims ?? []) {
            if (!(claim in claims)) {
                throw IdentityError.of("CLAIM_INVALID", `the token has no ${claim} claim`);
            }
        }

        // refuse an expired token and one not valid yet
        if (claims.exp !== undefined && claims.exp <= now - tolerance) {
            throw IdentityError.of("TOKEN_EXPIRED", "the token expired");
        } else if (claims.nbf !== undefined && claims.nbf > now + tolerance) {
            throw IdentityError.of("TOKEN_NOT_YET_VALID", "the token is not valid yet");
        }

        // refuse a token older than the longest age, or issued in the future
        if (options.maxTokenAge !== undefined) {
            if (claims.iat === undefined) {
                throw IdentityError.of("CLAIM_INVALID", "the token has no iat claim");
            } else if (now - claims.iat > options.maxTokenAge + tolerance) {
                throw IdentityError.of("TOKEN_EXPIRED", "the token is older than its longest age");
            } else if (claims.iat > now + tolerance) {
                throw IdentityError.of("CLAIM_INVALID", "the token was issued in the future");
            }
        }

        // require its issuer, audience and subject
        const issuers = options.issuer === undefined ? undefined : [options.issuer].flat();
        const audiences = options.audience === undefined ? undefined : [options.audience].flat();
        const named = claims.aud === undefined ? [] : [claims.aud].flat();
        if (issuers !== undefined && (claims.iss === undefined || !issuers.includes(claims.iss))) {
            throw IdentityError.of("ISSUER_REFUSED", "the token comes from another issuer");
        } else if (audiences !== undefined && !named.some((each) => audiences.includes(each))) {
            throw IdentityError.of("AUDIENCE_REFUSED", "the token is for another audience");
        } else if (options.subject !== undefined && claims.sub !== options.subject) {
            throw IdentityError.of("CLAIM_INVALID", "the token is for another subject");
        }
    },
};

/** Read decoded claims, refusing registered claims of the wrong type. */
function parseClaims(value: Record<string, unknown>): JwtClaims {
    const parsed = JwtClaims.safeParse(value);
    if (!parsed.success) {
        throw IdentityError.of("TOKEN_MALFORMED", "the token has invalid claims");
    }

    return parsed.data;
}

/** Report whether a header's type is the expected one, ignoring case and an application/ prefix. */
function sameType(actual: string | undefined, expected: string): boolean {
    return actual !== undefined && mediaType(actual) === mediaType(expected);
}

/** Read a media type in lower case without an application/ prefix. */
function mediaType(type: string): string {
    return type.toLowerCase().replace(/^application\//u, "");
}
