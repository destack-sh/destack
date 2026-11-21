/** The scheme of Basic credentials in an authorization header, in any case (RFC 9110 11.1). */
const SCHEME = /^basic /iu;

/** A user and a password as HTTP Basic authentication sends them (RFC 7617). */
export const BasicCredential = {
    /** Report whether an authorization header has Basic credentials. */
    is(authorization: string | null): boolean {
        return authorization !== null && SCHEME.test(authorization);
    },

    /** Write a user and a password as an authorization header's value. */
    write(username: string, password: string): string {
        const encoded = new TextEncoder().encode(`${username}:${password}`);

        return `Basic ${encoded.toBase64()}`;
    },

    /** Read an authorization header's user and password, absent for any other value. */
    read(
        authorization: string | null,
    ): { readonly username: string; readonly password: string } | undefined {
        // leave another scheme
        if (authorization === null || !SCHEME.test(authorization)) {
            return undefined;
        }

        // decode the pair, absent for malformed base64 or text
        const encoded = authorization.replace(SCHEME, "");
        let decoded: string;
        try {
            const bytes = Uint8Array.fromBase64(encoded);
            decoded = new TextDecoder("utf-8", { fatal: true }).decode(bytes);
        } catch {
            return undefined;
        }

        // split it at its first colon, as a password may have colons of its own
        const colon = decoded.indexOf(":");

        return colon === -1
            ? undefined
            : { username: decoded.slice(0, colon), password: decoded.slice(colon + 1) };
    },

    /** Write the challenge of a realm, which a client answers with a user and a password. */
    challenge(realm: string): string {
        return `Basic realm="${realm}"`;
    },
};
