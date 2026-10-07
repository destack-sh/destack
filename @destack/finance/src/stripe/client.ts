import type { Fetch } from "@destack/service";
import { ServiceError } from "@destack/service/error";

/** The Stripe API's origin. */
export const STRIPE_API = "https://api.stripe.com";

/** The API version every request pins, where periods live on subscription items. */
export const STRIPE_VERSION = "2025-09-30.clover";

/** A form parameter as Stripe's API encodes it: scalars, nested objects and arrays. */
export type StripeParameter =
    | string
    | number
    | boolean
    | null
    | undefined
    | readonly StripeParameter[]
    | { readonly [key: string]: StripeParameter };

/** A secret read through the vault, such as the API key. */
export interface StripeSecret {
    /** Read the secret's current value. */
    read(): Promise<{ readonly value: { readonly encoding: string; readonly value: string } }>;
}

/** A Stripe API client authenticating with a secret key read through the vault at each request. */
export class StripeClient {
    /** The API key. */
    readonly #key: StripeSecret;
    /** The fetch reaching the API. */
    readonly #fetch: Fetch;

    /** Call Stripe with a vault secret's key through a fetch. */
    constructor(key: StripeSecret, fetch: Fetch) {
        this.#key = key;
        this.#fetch = fetch;
    }

    /** Send a request with form parameters, once per idempotency key, and read the JSON object Stripe answers. */
    async request(
        method: "GET" | "POST" | "DELETE",
        path: string,
        parameters: Readonly<Record<string, StripeParameter>> = {},
        idempotencyKey?: string,
    ): Promise<Record<string, unknown>> {
        // authenticate with the current key and pin the version
        const key = await StripeClient.text(this.#key);
        const headers = new Headers({
            authorization: `Bearer ${key}`,
            "stripe-version": STRIPE_VERSION,
        });
        if (idempotencyKey !== undefined) {
            headers.set("idempotency-key", idempotencyKey);
        }

        // send the parameters in the query of a read, else as the form body
        const form = StripeClient.encode(parameters);
        const isRead = method === "GET";
        const url = `${STRIPE_API}${path}${isRead && form !== "" ? `?${form}` : ""}`;
        if (!isRead) {
            headers.set("content-type", "application/x-www-form-urlencoded");
        }
        const response = await this.#fetch(
            new Request(url, { method, headers, ...(isRead ? {} : { body: form }) }),
        );

        // refuse an error answer with Stripe's message
        const body: unknown = await response.json();
        if (!response.ok || typeof body !== "object" || body === null) {
            throw new ServiceError("BAD_GATEWAY", {
                message: `stripe answered ${response.status} to ${method} ${path}: ${messageOf(body)}`,
            });
        }

        return { ...body };
    }

    /** Read a secret's text value. */
    static async text(secret: StripeSecret): Promise<string> {
        const { value } = await secret.read();
        if (value.encoding !== "text") {
            throw new TypeError(`a stripe secret is ${value.encoding}, not text`);
        }

        return value.value;
    }

    /** Encode parameters as Stripe's forms nest them: `items[0][price]=price_1`. */
    static encode(parameters: Readonly<Record<string, StripeParameter>>): string {
        const pairs: [string, string][] = [];
        for (const [name, value] of Object.entries(parameters)) {
            appendPairs(pairs, name, value);
        }

        return new URLSearchParams(pairs).toString();
    }
}

/** Append a parameter's form pairs, nesting arrays by index and objects by key, and skipping absent values. */
function appendPairs(pairs: [string, string][], name: string, value: StripeParameter): void {
    // skip absent values, and write scalars as they are
    if (value === undefined || value === null) {
        return;
    } else if (typeof value !== "object") {
        pairs.push([name, String(value)]);
        return;
    }

    // nest arrays by index and objects by key
    const entries = isList(value)
        ? value.map((each, index) => [String(index), each] as const)
        : Object.entries(value);
    for (const [key, nested] of entries) {
        appendPairs(pairs, `${name}[${key}]`, nested);
    }
}

/** Decide whether a parameter is a list, which nests by index. */
function isList(value: StripeParameter): value is readonly StripeParameter[] {
    return Array.isArray(value);
}

/** Read the message of a Stripe error answer. */
function messageOf(body: unknown): string {
    // read `error.message` when the answer has one
    if (typeof body === "object" && body !== null && "error" in body) {
        const { error } = body;
        if (typeof error === "object" && error !== null && "message" in error) {
            return String(error.message);
        }
    }

    return "no message";
}
