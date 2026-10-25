import type { Environment } from "@destack/environment";
import type { Keyring } from "@destack/identity";
import { Digest } from "@destack/schema";
import { AccountError } from "../error/error.ts";
import type { Select } from "@destack/db";
import type { challenge } from "../object/challenge.ts";
import type { factor } from "../object/factor.ts";
import { Totp } from "./totp.ts";
import { Credential } from "../credential/credential.ts";

/** The label the keyring seals each factor's secret under, with its user as context. */
const FACTOR_LABEL = "@destack/account/factor";

/** The bytes of a TOTP secret, RFC 4226 4's recommended 160 bits. */
const SECRET_BYTES = 20;

/** The recovery codes an enrollment issues, each usable once. */
const RECOVERY_CODES = 10;

/** The base32 characters of each half of a recovery code. */
const RECOVERY_HALF = 5;

/** The RFC 4648 base32 alphabet authenticator apps read secrets in. */
const BASE32 = "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

/** The bits each base32 character encodes. */
const BASE32_BITS = 5;

/** The issuer authenticator apps list a factor under. */
const ISSUER = "Destack";

/** The milliseconds a sign-in's second-factor challenge lasts: ten minutes. */
const CHALLENGE_MILLISECONDS = 10 * 60_000;

/** The codes a factor tries before it locks, as Better Auth's two-factor lockout counts them. */
export const MAX_FAILURES = 5;

/** How long a factor stays locked once its failures run out: 5 codes per 15 minutes, 480 a day. */
export const LOCKOUT_MILLISECONDS = 15 * 60_000;

/** A new enrollment: its row, and what its person copies into an authenticator app. */
export interface Enrollment {
    /** The row's sealed secret, recovery code digests, pending status and zeroed counters. */
    readonly row: Pick<
        Select<typeof factor.table>,
        "secret" | "recoveryCodes" | "status" | "lastStep" | "failures" | "lockedUntil"
    >;
    /** The otpauth URI an authenticator app enrolls from. */
    readonly uri: string;
    /** The recovery codes, shown once. */
    readonly recoveryCodes: readonly string[];
}

/** A user's authenticator: enrolled once, then checked at each sign-in and step-up. */
export const Factor = {
    /** Enroll a user's authenticator, sealing its secret and digesting its recovery codes. */
    async enroll(
        user: { readonly id: string; readonly email: string },
        keyring: Keyring,
        environment: Pick<Environment, "crypto" | "random">,
    ): Promise<Enrollment> {
        // draw the secret and the recovery codes
        const { crypto, random } = environment;
        const secret = random.bytes(SECRET_BYTES);
        const recoveryCodes = Array.from({ length: RECOVERY_CODES }, () => recoveryCode(random));

        // seal the secret under the user, and digest each recovery code
        const sealed = await keyring.encrypt(secret, { label: FACTOR_LABEL, context: user.id });
        const digests = await Promise.all(recoveryCodes.map((code) => Digest.of(code, crypto)));

        // describe the enrollment an authenticator app reads (Key Uri Format)
        const uri = new URL(`otpauth://totp/${encodeURIComponent(`${ISSUER}:${user.email}`)}`);
        uri.searchParams.set("secret", base32(secret));
        uri.searchParams.set("issuer", ISSUER);

        return {
            row: {
                secret: sealed,
                recoveryCodes: digests,
                status: "pending",
                lastStep: 0,
                failures: 0,
                lockedUntil: 0,
            },
            uri: uri.href,
            recoveryCodes,
        };
    },

    /** Accept a code of a factor's authenticator, answering its step to store as the last. */
    async verify(
        row: Pick<Select<typeof factor.table>, "userId" | "secret" | "lastStep">,
        code: string,
        keyring: Keyring,
        environment: Pick<Environment, "clock" | "crypto">,
    ): Promise<number> {
        const secret = await keyring.decrypt(row.secret, {
            label: FACTOR_LABEL,
            context: row.userId,
        });
        const totp = await Totp.of(secret, environment.crypto);

        return totp.verify(code, row.lastStep, environment);
    },

    /** Refuse a factor locked after too many failures. */
    requireUnlocked(row: Pick<Select<typeof factor.table>, "lockedUntil">, now: number): void {
        if (row.lockedUntil > now) {
            throw AccountError.of("FACTOR_LOCKED", "too many wrong codes: try again later");
        }
    },

    /** Spend a recovery code, answering the digests left. */
    async recover(
        row: Pick<Select<typeof factor.table>, "recoveryCodes">,
        code: string,
        crypto: Environment["crypto"],
    ): Promise<string[]> {
        // refuse a code that is no unused recovery code
        const digest = await Digest.of(code, crypto);
        if (!row.recoveryCodes.includes(digest)) {
            throw AccountError.of("CODE_INVALID", "the recovery code is wrong or used");
        }

        return row.recoveryCodes.filter((each) => each !== digest);
    },
};

/** A sign-in's pending second-factor step, stored as its token's digest. */
export const Challenge = {
    /** Issue a challenge's token, answering it with its digest and expiry. */
    async issue(
        environment: Pick<Environment, "clock" | "crypto" | "random">,
    ): Promise<{ readonly token: string; readonly digest: string; readonly expiresAt: number }> {
        const drawn = await Credential.create("challenge", environment);
        const expiresAt = environment.clock.now() + CHALLENGE_MILLISECONDS;

        return { token: drawn.secret, digest: drawn.digest, expiresAt };
    },

    /** Read a token's digest, as challenges are found by it. */
    digest(token: string, crypto: Environment["crypto"]): Promise<string> {
        return Digest.of(token, crypto);
    },

    /** Refuse a challenge that lapsed. */
    require(row: Pick<Select<typeof challenge.table>, "expiresAt">, now: number): void {
        if (row.expiresAt <= now) {
            throw AccountError.of("CHALLENGE_EXPIRED", "the sign-in's second step expired");
        }
    },
};

/** Draw a recovery code: two halves of base32 characters joined by a hyphen. */
function recoveryCode(random: Environment["random"]): string {
    // draw a byte per character, keeping its lowest five bits
    const half = () =>
        Array.from(random.bytes(RECOVERY_HALF), (byte) => BASE32.charAt(byte & 31)).join("");

    return `${half()}-${half()}`;
}

/** Encode bytes in RFC 4648 base32 without padding, as otpauth URIs carry secrets. */
function base32(bytes: Uint8Array): string {
    // read the bytes as one bit string, five bits per character
    const bits = Array.from(bytes, (byte) => byte.toString(2).padStart(8, "0")).join("");
    const length = Math.ceil(bits.length / BASE32_BITS);

    return Array.from({ length }, (_, index) => {
        const chunk = bits.slice(index * BASE32_BITS, (index + 1) * BASE32_BITS);

        return BASE32.charAt(Number.parseInt(chunk.padEnd(BASE32_BITS, "0"), 2));
    }).join("");
}
