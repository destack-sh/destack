import type { Lease, LeaseMode } from "@destack/resource";
import { type JsonObject, schema } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import {
    GitAdvertisement,
    GitCredential,
    type Fetch,
    type GitListing,
    type GitStorage,
} from "../storage/index.ts";

/** How long a Git token lives, in seconds: an hour. */
const ACCESS_TTL_SECONDS = 3600;

/** The user name beside a token in Git basic authentication, which Artifacts accepts and ignores. */
const TOKEN_USER = "x";

/** The Cloudflare API base Artifacts answers under. */
const API = new URL("https://api.cloudflare.com/client/v4/");

/** A Cloudflare API answer: its result, or the errors refusing the request. */
const Envelope = schema.looseObject({
    success: schema.boolean(),
    errors: schema
        .array(schema.looseObject({ code: schema.number(), message: schema.string() }))
        .exactOptional(),
    result: schema.unknown().exactOptional(),
});

/** A repository token Artifacts issued: the secret and when it expires. */
const IssuedToken = schema.looseObject({
    plaintext: schema.string().min(1),
    expires_at: schema.string().min(1),
});

/** Where a Cloudflare account keeps its Artifacts repositories, and the API token managing them. */
export interface ArtifactsStorageOptions {
    /** The Cloudflare account identifier. */
    readonly account: string;
    /** The Artifacts namespace holding the repositories. */
    readonly namespace: string;
    /** The API token with Artifacts edit permission. */
    readonly token: string;
    /** The fetch calling Cloudflare. */
    readonly fetch?: Fetch;
}

/** Repositories stored in Cloudflare Artifacts through its REST API, read and written over Git's smart HTTP protocol. */
export class ArtifactsStorage implements GitStorage {
    /** The provider name repositories stored here record. */
    readonly provider = "cloudflare-artifacts";
    /** Where the account keeps the repositories, and its token. */
    readonly #options: ArtifactsStorageOptions;
    /** The fetch calling Cloudflare. */
    readonly #fetch: Fetch;

    /** Use an account's namespace and API token. */
    constructor(options: ArtifactsStorageOptions) {
        this.#options = options;
        this.#fetch = options.fetch ?? globalThis.fetch;
    }

    /** Build the Git remote of a repository without a credential. */
    remote(id: string): string {
        const { account, namespace } = this.#options;

        return `https://${account}.artifacts.cloudflare.net/git/${namespace}/${id}.git`;
    }

    /** Create an empty repository, accepting one that exists: POST /repos. */
    async create(id: string): Promise<void> {
        const response = await this.#request("POST", "repos", { name: id });
        if (response.status === 409 && (await this.#request("GET", `repos/${id}`)).ok) {
            return;
        }
        await this.#require(response, `create ${id}`);
    }

    /** Delete a repository and its history, accepting one already gone: DELETE /repos/{name}. */
    async delete(id: string): Promise<void> {
        const response = await this.#request("DELETE", `repos/${id}`);
        if (response.status === 404) {
            return;
        }
        await this.#require(response, `delete ${id}`);
    }

    /** List the branches and tags Git's advertisement names, with annotated tags' objects and commits. */
    async references(id: string): Promise<GitListing> {
        return GitAdvertisement.read(await this.open(id, "read"), this.#fetch);
    }

    /** Lease a Git client a token scoped to the repository and mode for an hour: POST /tokens. */
    async open(id: string, mode: LeaseMode): Promise<Lease> {
        // issue a token for the repository and mode
        const response = await this.#request("POST", "tokens", {
            repo: id,
            scope: mode,
            ttl: ACCESS_TTL_SECONDS,
        });
        const issued = IssuedToken.parse(await this.#require(response, `issue a token for ${id}`));

        // lease it to Git as basic authentication, refusing an unreadable expiry
        const expiresAt = Date.parse(issued.expires_at);
        if (Number.isNaN(expiresAt)) {
            throw new ServiceError("BAD_GATEWAY", {
                message: `cloudflare artifacts issued a token with an invalid expiry: ${issued.expires_at}`,
            });
        }
        const granted = { username: TOKEN_USER, password: issued.plaintext, expiresAt };

        return GitCredential.lease(granted, this.remote(id), mode);
    }

    /** Send an API request for the namespace under the account's token. */
    #request(
        method: "GET" | "POST" | "DELETE",
        path: string,
        body?: JsonObject,
    ): Promise<Response> {
        // address the namespace under the account's token
        const { account, namespace, token } = this.#options;
        const url = new URL(`accounts/${account}/artifacts/namespaces/${namespace}/${path}`, API);
        const headers = new Headers({ Authorization: `Bearer ${token}` });
        if (body !== undefined) {
            headers.set("Content-Type", "application/json");
        }

        return this.#fetch(url.href, {
            method,
            headers,
            ...(body === undefined ? {} : { body: JSON.stringify(body) }),
        });
    }

    /** Read a successful answer's result, failing with Cloudflare's errors otherwise. */
    async #require(response: Response, action: string): Promise<unknown> {
        // answer the result of a successful envelope
        const text = await response.text();
        const envelope = Envelope.safeParse(parseJson(text));
        if (response.ok && envelope.success && envelope.data.success) {
            return envelope.data.result;
        }

        // refuse with Cloudflare's errors, or the raw body
        const errors = envelope.success
            ? (envelope.data.errors ?? [])
                  .map((error) => `${error.code} ${error.message}`)
                  .join(", ")
            : text;
        throw new ServiceError("BAD_GATEWAY", {
            message: `cloudflare artifacts refused to ${action} with HTTP ${response.status}: ${errors}`,
        });
    }
}

/** Read a body as JSON, absent when it is not JSON. */
function parseJson(text: string): unknown {
    try {
        return JSON.parse(text);
    } catch (error) {
        if (!(error instanceof SyntaxError)) {
            throw error;
        }

        return undefined;
    }
}
