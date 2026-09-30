import { identifier, schema } from "@destack/schema";
import { ServiceError } from "../error/index.ts";
import { type Caller, CallerAuthentication } from "./caller.ts";

/** How long a lending holds, in milliseconds: an hour, far above the milliseconds an outbox takes to deliver a sent call. */
const LENDING_MILLISECONDS = 60 * 60_000;

/** The authority a caller lends: its subject, the subject sets it belongs to, the principals acting for it and the permissions it narrowed to. */
export const LentAuthority = CallerAuthentication.pick({
    subject: true,
    subjects: true,
    delegates: true,
    permissions: true,
});
/** The authority a caller lends. */
export type LentAuthority = schema.Infer<typeof LentAuthority>;

/** A caller's authority lent to the installation it called, for the calls that installation sends. */
export const LendingClaim = LentAuthority.extend({
    /** The installation called. */
    installation: CallerAuthentication.shape.subject,
    /** The space the installation serves. */
    scope: identifier("space"),
    /** When the lending lapses, in UTC epoch milliseconds. */
    expiresAt: schema.number().int(),
});
/** A caller's authority lent to the installation it called. */
export type LendingClaim = schema.Infer<typeof LendingClaim>;

/**
 * A holder's proof that a caller called one of its installations, lending the caller's authority to it.
 *
 * The holder's routers sign it with a key its cells share, and only those cells verify it.
 */
export class Lending {
    /** The key signing and verifying lendings. */
    readonly #key: CryptoKey;

    /** Sign and verify lendings with a key. */
    constructor(key: CryptoKey) {
        this.#key = key;
    }

    /** Create lendings under a new key. */
    static async generate(): Promise<Lending> {
        const key = await crypto.subtle.generateKey({ name: "HMAC", hash: "SHA-256" }, true, [
            "sign",
            "verify",
        ]);

        return new Lending(key);
    }

    /** Create lendings under a key kept as raw bytes. */
    static async import(bytes: Uint8Array<ArrayBuffer>): Promise<Lending> {
        const key = await crypto.subtle.importKey(
            "raw",
            bytes,
            { name: "HMAC", hash: "SHA-256" },
            true,
            ["sign", "verify"],
        );

        return new Lending(key);
    }

    /** Read the key as raw bytes, to keep it. */
    async export(): Promise<Uint8Array<ArrayBuffer>> {
        return new Uint8Array(await crypto.subtle.exportKey("raw", this.#key));
    }

    /** Sign a caller's authority lent to an installation of a space, lapsing after an hour. */
    async sign(
        caller: Caller,
        installation: LendingClaim["installation"],
        scope: LendingClaim["scope"],
        now = Date.now(),
    ): Promise<string> {
        // sign the claim's JSON with the holder's key
        const { subject, subjects, delegates, permissions } = caller.authentication;
        const claim: LendingClaim = {
            subject,
            subjects,
            ...(delegates === undefined ? {} : { delegates }),
            ...(permissions === undefined ? {} : { permissions }),
            installation,
            scope,
            expiresAt: now + LENDING_MILLISECONDS,
        };
        const body = new TextEncoder().encode(JSON.stringify(claim));
        const mac = new Uint8Array(await crypto.subtle.sign("HMAC", this.#key, body));

        return `${body.toBase64({ alphabet: "base64url" })}.${mac.toBase64({ alphabet: "base64url" })}`;
    }

    /** Read a lending this holder signed and that holds now, refusing any other. */
    async verify(token: string, now = Date.now()): Promise<LendingClaim> {
        // require a body with this holder's signature
        const [encoded, signature, ...rest] = token.split(".");
        const refusal = new ServiceError("UNAUTHORIZED", { message: "invalid lending" });
        if (encoded === undefined || signature === undefined || rest.length > 0) {
            throw refusal;
        }
        const body = Uint8Array.fromBase64(encoded, { alphabet: "base64url" });
        const mac = Uint8Array.fromBase64(signature, { alphabet: "base64url" });
        if (!(await crypto.subtle.verify("HMAC", this.#key, mac, body))) {
            throw refusal;
        }

        // require a lending that holds
        const claim = LendingClaim.parse(JSON.parse(new TextDecoder().decode(body)));
        if (claim.expiresAt <= now) {
            throw new ServiceError("UNAUTHORIZED", { message: "the lending lapsed" });
        }

        return claim;
    }
}
