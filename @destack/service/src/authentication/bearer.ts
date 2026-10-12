import { decodeJwt, errors } from "jose";
import { isServiceError, ServiceError } from "../error/index.ts";

/** The shape of an authorization header with one bearer token, per RFC 6750 2.1. */
const BEARER = /^Bearer (\S+)$/u;

/** The bearer token of an authorization header, per RFC 6750. */
export const Bearer = {
    /** Read the one bearer token of an authorization header, absent for none or an invalid one. */
    token(authorization: string | null): string | undefined {
        return authorization === null ? undefined : BEARER.exec(authorization)?.[1];
    },

    /** Read the one bearer token of a request without cookies. */
    read(headers: Headers): string | undefined {
        return headers.has("cookie") ? undefined : Bearer.token(headers.get("authorization"));
    },

    /** Require the one bearer token of a request without cookies. */
    require(headers: Headers): string {
        const token = Bearer.read(headers);
        if (token === undefined) {
            throw new ServiceError("UNAUTHORIZED", { message: "invalid bearer credential" });
        }

        return token;
    },

    /** Read a token's issuer without verifying it, absent for a token that is no JSON Web Token. */
    issuer(token: string): unknown {
        try {
            return decodeJwt(token).iss;
        } catch (error) {
            if (!(error instanceof errors.JWTInvalid)) {
                throw error;
            }

            return undefined;
        }
    },

    /** Answer a failed verification of a signed token. */
    refusal(error: unknown, message: string): ServiceError<string, unknown> {
        // keep a refusal the key source answered
        if (isServiceError(error)) {
            return error;
        }
        // refuse a token that fails its checks
        else if (
            error instanceof errors.JWTClaimValidationFailed ||
            error instanceof errors.JWTExpired ||
            error instanceof errors.JWTInvalid ||
            error instanceof errors.JWSInvalid ||
            error instanceof errors.JWSSignatureVerificationFailed ||
            error instanceof errors.JOSEAlgNotAllowed ||
            error instanceof errors.JOSENotSupported ||
            error instanceof errors.JWKSNoMatchingKey
        ) {
            return new ServiceError("UNAUTHORIZED", { message, cause: error });
        }
        // answer any other failure as unavailable keys
        else {
            return new ServiceError("SERVICE_UNAVAILABLE", {
                message: "authentication keys are unavailable",
                cause: error,
            });
        }
    },
};
