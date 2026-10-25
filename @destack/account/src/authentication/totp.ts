import type { Environment } from "@destack/environment";
import { AccountError } from "../error/error.ts";

/** The milliseconds of one time step, RFC 6238's default of thirty seconds. */
const STEP_MILLISECONDS = 30_000;

/** The steps a code is accepted either side of the current one, RFC 6238 5.2's recommended one. */
const WINDOW = 1;

/** The digits of a code, the six RFC 4226 recommends. */
const DIGITS = 6;

/** The HMAC an authenticator signs its counter with, RFC 6238's default SHA-1. */
const HMAC_SHA1 = { name: "HMAC", hash: "SHA-1" };

/** The bytes of an HOTP counter, RFC 4226's eight-byte big-endian moving factor. */
const COUNTER_BYTES = 8;

/** The time-based one-time passwords of an enrolled secret (RFC 6238 over RFC 4226 HOTP). */
export class Totp {
    /** The HMAC key of the enrolled secret. */
    readonly #key: CryptoKey;
    /** The digits of each code. */
    readonly digits: number;

    /** Create a code generator from a secret's HMAC key and the digits of its codes. */
    private constructor(key: CryptoKey, digits: number) {
        this.#key = key;
        this.digits = digits;
    }

    /** Import an enrolled secret, six digits per code unless given. */
    static async of(
        secret: Uint8Array<ArrayBuffer>,
        crypto: Environment["crypto"],
        digits = DIGITS,
    ): Promise<Totp> {
        const key = await crypto.importKey("raw", secret, HMAC_SHA1, false, ["sign"]);

        return new Totp(key, digits);
    }

    /** Read the time step a moment falls in. */
    static step(now: number): number {
        return Math.floor(now / STEP_MILLISECONDS);
    }

    /** Compute the code of a time step (RFC 4226 5.3). */
    async code(step: number, crypto: Environment["crypto"]): Promise<string> {
        // sign the step as an eight-byte big-endian counter
        const counter = new Uint8Array(COUNTER_BYTES);
        new DataView(counter.buffer).setBigUint64(0, BigInt(step));
        const signature = new DataView(await crypto.sign("HMAC", this.#key, counter));

        // truncate it dynamically to the code's digits
        const offset = signature.getUint8(signature.byteLength - 1) & 15;
        const truncated = signature.getUint32(offset) & 0x7fffffff;

        return String(truncated % 10 ** this.digits).padStart(this.digits, "0");
    }

    /** Accept a code of a step around a moment after the last accepted one, answering its step. */
    async verify(
        code: string,
        lastStep: number,
        environment: Pick<Environment, "clock" | "crypto">,
    ): Promise<number> {
        // find the step within the window whose code it is
        const { clock, crypto } = environment;
        const current = Totp.step(clock.now());
        const steps = Array.from(
            { length: 2 * WINDOW + 1 },
            (_, index) => current - WINDOW + index,
        );
        const codes = await Promise.all(steps.map((step) => this.code(step, crypto)));
        const matched = steps.find((_, index) => codes[index] === code);

        // refuse a code of no step around now, and a step at or before the last accepted one
        if (matched === undefined) {
            throw AccountError.of("CODE_INVALID", "the code is wrong or expired");
        } else if (matched <= lastStep) {
            throw AccountError.of("CODE_REUSED", "the code was already used");
        }

        return matched;
    }
}
