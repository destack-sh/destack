import { ServiceError } from "@destack/service/error";

/** The header carrying a webhook delivery's signature. */
export const STRIPE_SIGNATURE_HEADER = "stripe-signature";

/** How far a delivery's signed time may lie from now, five minutes as Stripe's libraries allow. */
const TOLERANCE_MILLISECONDS = 300_000;

/** The signatures of Stripe's webhook deliveries: HMAC-SHA256 over the signed time and the payload. */
export const StripeSignature = {
    /** Sign a payload at a time with an endpoint's secret, as Stripe signs a delivery. */
    async sign(payload: string, secret: string, time: number): Promise<string> {
        const seconds = Math.floor(time / 1000);
        const digest = await hmac(secret, `${seconds}.${payload}`);

        return `t=${seconds},v1=${digest}`;
    },

    /** Verify a delivery's signature, refusing a stale or forged one. */
    async verify(
        payload: string,
        header: string | null,
        secret: string,
        now: number,
    ): Promise<void> {
        // read the signed time and the v1 signatures
        const parts = (header ?? "").split(",").map((part) => part.split("="));
        const time = Number(parts.find(([key]) => key === "t")?.[1]);
        const signatures = parts.filter(([key]) => key === "v1").map(([, value]) => value);
        if (!Number.isInteger(time) || signatures.length === 0) {
            throw new ServiceError("UNAUTHORIZED", { message: "invalid stripe signature header" });
        }

        // refuse a delivery signed outside the tolerance, or by another secret
        const expected = await hmac(secret, `${time}.${payload}`);
        if (Math.abs(now - time * 1000) > TOLERANCE_MILLISECONDS) {
            throw new ServiceError("UNAUTHORIZED", { message: "stale stripe signature" });
        } else if (!signatures.some((signature) => equal(signature ?? "", expected))) {
            throw new ServiceError("UNAUTHORIZED", { message: "forged stripe signature" });
        }
    },
};

/** Compute an HMAC-SHA256 as lowercase hexadecimal. */
async function hmac(secret: string, message: string): Promise<string> {
    // key the HMAC with the secret's UTF-8 bytes and sign the message's
    const encoder = new TextEncoder();
    const key = await crypto.subtle.importKey(
        "raw",
        encoder.encode(secret),
        { name: "HMAC", hash: "SHA-256" },
        false,
        ["sign"],
    );
    const signed = await crypto.subtle.sign("HMAC", key, encoder.encode(message));

    return new Uint8Array(signed).toHex();
}

/** Compare two hexadecimal signatures in time independent of where they differ. */
function equal(left: string, right: string): boolean {
    let difference = left.length ^ right.length;
    for (let index = 0; index < Math.min(left.length, right.length); index++) {
        difference |= left.charCodeAt(index) ^ right.charCodeAt(index);
    }

    return difference === 0;
}
