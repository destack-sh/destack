import type { Lease, LeaseMode } from "@destack/resource";
import { JsonWebToken } from "../token/token.ts";
import { schema } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import {
    GitAdvertisement,
    type Fetch,
    GitCredential,
    type GitListing,
    type GitStorage,
} from "../storage/index.ts";

/** How long a token managing repositories lives, in seconds: one request's worth. */
const MANAGEMENT_TTL_SECONDS = 60;

/** How long a Git credential lives, in seconds: the hour code.storage's CI example grants. */
const ACCESS_TTL_SECONDS = 3600;

/** The user name code.storage expects beside a JWT in Git basic authentication. */
const TOKEN_USER = "t";

/** The client name code.storage logs for tokens this storage signs. */
const SUBJECT = "destack-repository";

/** The problem codes of creating a repository code.storage has already. */
const CREATED_CODES = ["conflict"];

/** The problem codes of deleting a repository code.storage deleted already or never had. */
const DELETED_CODES = ["repository_not_found", "repository_deleted"];

/** The problem details code.storage returns for a failed request. */
const Problem = schema.object({ code: schema.string(), detail: schema.string() }).passthrough();

/** The code.storage scopes each Git mode needs; pushing needs reading too, since scopes match exactly. */
const MODE_SCOPES: Readonly<Record<LeaseMode, readonly string[]>> = {
    read: ["git:read"],
    write: ["git:read", "git:write"],
};

/** Where a code.storage organisation serves its API and Git, and the key signing its JWTs. */
export interface CodeStorageOptions {
    /** The organisation identifier, the JWT issuer. */
    readonly organization: string;
    /** The organisation's ES256 or RS256 private API key. */
    readonly key: CryptoKey;
    /** The API base, such as https://api.your-org.code.storage/api. */
    readonly api: URL;
    /** The Git host, such as https://your-org.code.storage. */
    readonly git: URL;
    /** The fetch reaching code.storage. */
    readonly fetch?: Fetch;
}

/** Repositories stored at code.storage through its HTTP API, read and written over Git's smart HTTP protocol. */
export class CodeStorage implements GitStorage {
    /** The provider name repositories stored here record. */
    readonly provider = "code-storage";
    /** Where the organisation serves its API and Git, and its key. */
    readonly #options: CodeStorageOptions;
    /** The fetch reaching code.storage. */
    readonly #fetch: Fetch;

    /** Use an organisation's API key. */
    constructor(options: CodeStorageOptions) {
        this.#options = options;
        this.#fetch = options.fetch ?? globalThis.fetch;
    }

    /** Build the Git remote of a repository without a credential. */
    remote(id: string): string {
        return `${this.#options.git.href.replace(/\/$/, "")}/${id}.git`;
    }

    /** Create an empty repository named by the JWT's repo claim, accepting one that exists: POST /repos. */
    async create(id: string): Promise<void> {
        await this.#request("POST", "/repos", id, ["repo:write"], CREATED_CODES, {
            repo_name: id,
        });
    }

    /** Delete a repository and accept one already gone: DELETE /repos/{name}. */
    async delete(id: string): Promise<void> {
        await this.#request(
            "DELETE",
            `/repos/${encodeURIComponent(id)}`,
            id,
            ["repo:write"],
            DELETED_CODES,
        );
    }

    /** List the branches and tags Git's advertisement names, with annotated tags' objects and commits. */
    async references(id: string): Promise<GitListing> {
        return GitAdvertisement.read(await this.open(id, "read"), this.#fetch);
    }

    /** Lease a Git client a JWT scoped to the repository and mode for an hour. */
    async open(id: string, mode: LeaseMode): Promise<Lease> {
        const granted = await this.#credential(id, MODE_SCOPES[mode], ACCESS_TTL_SECONDS);

        return GitCredential.lease(granted, this.remote(id), mode);
    }

    /** Send an API request under a JWT for one repository and fail with code.storage's problem. */
    async #request(
        method: "POST" | "DELETE",
        path: string,
        name: string,
        scopes: readonly string[],
        settled: readonly string[],
        body?: Readonly<Record<string, unknown>>,
    ): Promise<void> {
        // authorize the request for the named repository alone
        const credential = await this.#credential(name, scopes, MANAGEMENT_TTL_SECONDS);
        const headers = new Headers({ Authorization: `Bearer ${credential.password}` });
        if (body !== undefined) {
            headers.set("Content-Type", "application/json");
        }
        const response = await this.#fetch(`${this.#options.api.href.replace(/\/$/, "")}${path}`, {
            method,
            headers,
            ...(body === undefined ? {} : { body: JSON.stringify(body) }),
        });

        // accept success and the problems naming the state the request asks for, and report others
        if (response.ok) {
            return;
        }
        const text = await response.text();
        const problem = Problem.safeParse(parseJson(text));
        if (!problem.success) {
            throw new ServiceError("BAD_GATEWAY", {
                message: `code.storage answered ${method} ${path} with HTTP ${response.status}: ${text}`,
            });
        }
        if (!settled.includes(problem.data.code)) {
            throw new ServiceError("BAD_GATEWAY", {
                message: `code.storage refused ${method} ${path}: ${problem.data.code}, ${problem.data.detail}`,
            });
        }
    }

    /** Sign a JWT for one repository with the given scopes and lifetime. */
    async #credential(
        name: string,
        scopes: readonly string[],
        ttl: number,
    ): Promise<GitCredential> {
        const now = Math.floor(Date.now() / 1000);
        const token = await JsonWebToken.sign(
            {
                iss: this.#options.organization,
                sub: SUBJECT,
                repo: name,
                scopes,
                iat: now,
                exp: now + ttl,
            },
            this.#options.key,
        );

        return { username: TOKEN_USER, password: token, expiresAt: (now + ttl) * 1000 };
    }
}

/** Read a body as JSON, absent when it is not JSON. */
function parseJson(text: string): unknown {
    try {
        return JSON.parse(text);
    } catch {
        return undefined;
    }
}
