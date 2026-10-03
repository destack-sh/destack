import type { Directory } from "@destack/directory";
import type { PackageId } from "@destack/package";
import { schema } from "@destack/schema";
import { type Authentication, TokenVerifier, Bearer } from "@destack/service/authentication";
import { ServiceError } from "@destack/service/error";

/** The issuer claim of a bearer token as read before verification: a space for space tokens and the universe's URL for user tokens. */
const UnverifiedClaims = schema.looseObject({
    /** The issuer. */
    iss: schema.unknown(),
});

/** The tokens spaces sign with their keys, verified against their identities in the directory. */
export const SpaceToken = {
    /** Report whether a request has a space token: a bearer token whose issuer is a space. */
    accepts(request: Request): boolean {
        return schema.identifier("space").safeParse(readIssuer(request)).success;
    },

    /** Verify a token a space signed for a package's service, in a space or the universe. */
    async verify(
        request: Request,
        options: {
            /** The universe's directory of the spaces' identities. */
            readonly directory: Directory;
            /** The package receiving the call. */
            readonly audience: PackageId;
            /** The space the call targets, the signing space for universe services. */
            readonly spaceId?: string;
        },
        now = Date.now(),
    ): Promise<Authentication> {
        // read the signing space's identity
        const issuer = schema.identifier("space").safeParse(readIssuer(request));
        if (!issuer.success) {
            throw new ServiceError("UNAUTHORIZED", { message: "invalid access token" });
        }
        const space = issuer.data;
        const identity = await options.directory.identity(space);
        if (identity === undefined) {
            throw new ServiceError("UNAUTHORIZED", {
                message: "the token's space has no identity",
            });
        }

        // verify the token with the space's signing key as the authority of its callers
        const verifier = new TokenVerifier({
            authority: { kind: "space", spaceId: space },
            issuer: space,
            audience: options.audience,
            keys: { keys: [{ ...identity.signingKey, alg: "ES256" }] },
        });

        return verifier.authenticate(request, options.spaceId ?? space, now);
    },
};

/** Read a bearer token's issuer without verifying it, absent for another credential and an undecodable token. */
function readIssuer(request: Request): unknown {
    // read the payload of a bearer token
    const [, payload] = Bearer.read(request.headers)?.split(".") ?? [];
    if (payload === undefined) {
        return undefined;
    }

    // decode its JSON claims
    try {
        const bytes = Uint8Array.fromBase64(payload, { alphabet: "base64url" });
        const claims = UnverifiedClaims.safeParse(JSON.parse(new TextDecoder().decode(bytes)));

        return claims.success ? claims.data.iss : undefined;
    } catch (error) {
        if (!(error instanceof SyntaxError)) {
            throw error;
        }

        return undefined;
    }
}
