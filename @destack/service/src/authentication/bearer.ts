import { ServiceError } from "../error/index.ts";

/** The shape of an authorization header with one bearer token, per RFC 6750 2.1. */
const BEARER = /^Bearer (\S+)$/u;

/** The bearer token of an authorization header, per RFC 6750. */
export const Bearer = {
    /** Read the one bearer token of an authorization header, absent for none or an invalid one. */
    token(authorization: string | null): string | undefined {
        return authorization === null ? undefined : BEARER.exec(authorization)?.[1];
    },

    /** Read the one bearer token of a request without cookies, absent for none, an invalid one, or cookies beside it. */
    read(headers: Headers): string | undefined {
        return headers.has("cookie") ? undefined : Bearer.token(headers.get("authorization"));
    },

    /** Require the one bearer token of a request without cookies, refusing it as unauthorized otherwise. */
    require(headers: Headers): string {
        const token = Bearer.read(headers);
        if (token === undefined) {
            throw new ServiceError("UNAUTHORIZED", { message: "invalid bearer credential" });
        }

        return token;
    },
};
