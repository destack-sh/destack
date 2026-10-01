/** The shape of an authorization header carrying one bearer token, per RFC 6750 2.1. */
const BEARER = /^Bearer (\S+)$/;

/** The bearer token an authorization header carries, per RFC 6750. */
export const Bearer = {
    /** Read the one bearer token an authorization header carries, absent for none or an invalid one. */
    token(authorization: string | null): string | undefined {
        return authorization === null ? undefined : BEARER.exec(authorization)?.[1];
    },

    /** Read the one bearer token of a request without cookies, absent when it carries none, an invalid one, or cookies beside it. */
    read(headers: Headers): string | undefined {
        return headers.has("cookie") ? undefined : Bearer.token(headers.get("authorization"));
    },
};
