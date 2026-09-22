import { AccessError } from "../error/index.ts";

/** A bearer credential whose current state is checked during authorization. */
export interface ShareToken {
    /** The stable credential identifier used by grants. */
    readonly id: string;
    /** The scope in which the credential can receive grants. */
    readonly scope: string;
    /** The SHA-256 digest of the bearer secret. */
    readonly digest: string;
    /** The creation time in Unix milliseconds. */
    readonly createdAt: number;
    /** The exclusive expiry time in Unix milliseconds, or null for no expiry. */
    readonly expiresAt: number | null;
    /** The revocation time in Unix milliseconds, or null while unrevoked. */
    readonly revokedAt: number | null;
}

/** Create an unguessable credential; expose the secret only to the creating caller. */
export async function createTokenCredential(): Promise<{ secret: string; digest: string }> {
    const bytes = crypto.getRandomValues(new Uint8Array(32));
    const secret = Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");

    return { secret, digest: await digestTokenCredential(secret) };
}

/** Hash a validated bearer secret without retaining plaintext. */
export async function digestTokenCredential(secret: string): Promise<string> {
    if (!/^[0-9a-f]{64}$(?![\s\S])/.test(secret)) {
        throw new AccessError("FORBIDDEN", "invalid share credential");
    }
    const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(secret));

    return Array.from(new Uint8Array(digest), (byte) => byte.toString(16).padStart(2, "0")).join(
        "",
    );
}
