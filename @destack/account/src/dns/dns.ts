import { ServiceError } from "@destack/service/error";

/** The resource record type of TXT records (RFC 1035 3.2.2). */
const TXT = 16;

/** The response code of a name that does not exist (RFC 1035 4.1.1). */
const NAME_ERROR = 3;

/** A character string of a TXT record's presentation form, quoted with escapes. */
const CHARACTER_STRING = /"((?:[^"\\]|\\.)*)"/g;

/** Resolve the DNS records domain verification reads. */
export interface DnsResolver {
    /** Read the TXT records of a name, each record's strings joined. */
    txt(name: string, signal?: AbortSignal): Promise<string[]>;
}

/** A resolver over DNS over HTTPS in the JSON form Cloudflare and Google serve. */
export class DnsOverHttps implements DnsResolver {
    /** The resolver's endpoint. */
    readonly endpoint: URL;
    /** Send the resolver's requests. */
    readonly #fetch: typeof fetch;

    /** Resolve through an endpoint, Cloudflare's by default. */
    constructor(
        endpoint = new URL("https://cloudflare-dns.com/dns-query"),
        send: typeof fetch = fetch,
    ) {
        this.endpoint = endpoint;
        this.#fetch = send;
    }

    /** Read the TXT records of a name, none for a name that does not exist. */
    async txt(name: string, signal?: AbortSignal): Promise<string[]> {
        // ask the resolver for the name's TXT records
        const url = new URL(this.endpoint);
        url.searchParams.set("name", name);
        url.searchParams.set("type", "TXT");
        const response = await this.#fetch(url, {
            headers: { accept: "application/dns-json" },
            ...(signal === undefined ? {} : { signal }),
        });
        if (!response.ok) {
            throw new ServiceError("SERVICE_UNAVAILABLE", {
                message: `the DNS resolver answered the lookup of ${name} with HTTP ${response.status}`,
            });
        }
        const answer = (await response.json()) as DnsAnswer;

        // read the records, or fail on a lookup the resolver did not answer
        if (answer.Status === 0 || answer.Status === NAME_ERROR) {
            return (answer.Answer ?? [])
                .filter((record) => record.type === TXT)
                .map((record) =>
                    Array.from(record.data.matchAll(CHARACTER_STRING), (match) =>
                        match[1]!.replace(/\\(.)/g, "$1"),
                    ).join(""),
                );
        }
        throw new ServiceError("SERVICE_UNAVAILABLE", {
            message: `the DNS lookup of ${name} failed with status ${answer.Status}`,
        });
    }
}

/** A DNS over HTTPS answer in its JSON form. */
interface DnsAnswer {
    /** The DNS response code, 0 for success. */
    readonly Status: number;
    /** The answer records. */
    readonly Answer?: readonly { readonly type: number; readonly data: string }[];
}
