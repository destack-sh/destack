import { present, schema } from "@destack/schema";
import type { Fetch } from "@destack/object";
import { CallError } from "@destack/object";

/** The resource record type of TXT records (RFC 1035 3.2.2). */
const TXT = 16;

/** The response code of a name that does not exist (RFC 1035 4.1.1). */
const NAME_ERROR = 3;

/** A character string of a TXT record's presentation form, quoted with escapes. */
const CHARACTER_STRING = /"((?:[^"\\]|\\.)*)"/gu;

/** A DNS over HTTPS answer in its JSON form. */
const DnsAnswer = schema.looseObject({
    /** The DNS response code, 0 for success. */
    Status: schema.number().int(),
    /** The answer records. */
    Answer: schema
        .array(schema.looseObject({ type: schema.number().int(), data: schema.string() }))
        .exactOptional(),
});

/** Resolve the DNS records domain verification reads. */
export interface DnsResolver {
    /** Read the TXT records of a name, each record's strings joined. */
    readonly txt: (name: string, signal?: AbortSignal) => Promise<string[]>;
}

/** A resolver over DNS over HTTPS in the JSON form Cloudflare and Google serve. */
export class DnsOverHttps implements DnsResolver {
    /** The resolver's endpoint. */
    readonly endpoint: URL;
    /** The fetch sending the resolver's requests. */
    readonly #fetch: Fetch;

    /** Resolve through an endpoint, Cloudflare's by default. */
    constructor(endpoint = new URL("https://cloudflare-dns.com/dns-query"), send: Fetch = fetch) {
        this.endpoint = endpoint;
        this.#fetch = send;
    }

    /** Read the TXT records of a name, none for a name that does not exist. */
    async txt(name: string, signal?: AbortSignal): Promise<string[]> {
        // ask the resolver for the name's TXT records
        const url = new URL(this.endpoint);
        url.searchParams.set("name", name);
        url.searchParams.set("type", "TXT");
        const response = await this.#fetch(
            new Request(url.href, {
                headers: { accept: "application/dns-json" },
                ...(signal === undefined ? {} : { signal }),
            }),
        );
        if (!response.ok) {
            throw new CallError("SERVICE_UNAVAILABLE", {
                message: `the DNS resolver answered the lookup of ${name} with HTTP ${response.status}`,
            });
        }

        // read the records of a found or missing name, joining each record's unescaped strings
        const answer = DnsAnswer.parse(await response.json());
        if (answer.Status === 0 || answer.Status === NAME_ERROR) {
            return (answer.Answer ?? [])
                .filter((record) => record.type === TXT)
                .map((record) =>
                    Array.from(record.data.matchAll(CHARACTER_STRING), (match) =>
                        present(match[1], "a TXT character string").replace(/\\(.)/gu, "$1"),
                    ).join(""),
                );
        }
        // refuse a failed lookup
        else {
            throw new CallError("SERVICE_UNAVAILABLE", {
                message: `the DNS lookup of ${name} failed with status ${answer.Status}`,
            });
        }
    }
}
