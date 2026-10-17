import { Secret } from "@destack/vault";
import type { Fetch } from "@destack/runtime";
import { CallError } from "@destack/runtime/error";

/** The Stripe API's origin. */
export const STRIPE_API = "https://api.stripe.com";

/** The API version every request sends, where periods live on subscription items. */
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

/** A Stripe API client authenticating with a secret key read through the vault at each request. */
export class StripeClient {
    /** The API key. */
    readonly #key: Secret;
    /** The fetch calling the API. */
    readonly #fetch: Fetch;

    /** Call Stripe with a vault secret's key through a fetch. */
    constructor(key: Secret, fetch: Fetch) {
        this.#key = key;
        this.#fetch = fetch;
    }

    /** Send a form request once per idempotency key, and read Stripe's JSON object answer. */
    async request(
        method: "GET" | "POST" | "DELETE",
        path: string,
        parameters: Readonly<Record<string, StripeParameter>> = {},
        idempotencyKey?: string,
    ): Promise<Record<string, unknown>> {
        // authenticate with the current key and send the version
        const key = await Secret.text(this.#key);
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

        // refuse a missing object as missing, and any other error answer with Stripe's message
        const body: unknown = await response.json();
        if (response.status === 404) {
            throw new CallError("NOT_FOUND", {
                message: `stripe has no ${path}: ${readMessage(body)}`,
            });
        } else if (!response.ok || typeof body !== "object" || body === null) {
            throw new CallError("BAD_GATEWAY", {
                message: `stripe answered ${response.status} to ${method} ${path}: ${readMessage(body)}`,
            });
        }

        return { ...body };
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

/** Append a parameter's form pairs, nesting arrays by index and objects by key. */
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

/** Read the message of a Stripe error answer, else the whole answer. */
function readMessage(body: unknown): string {
    // read the error's message
    if (typeof body === "object" && body !== null && "error" in body) {
        const { error } = body;
        if (typeof error === "object" && error !== null && "message" in error) {
            return String(error.message);
        }
    }

    return JSON.stringify(body);
}
