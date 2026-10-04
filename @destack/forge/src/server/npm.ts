import { Digest, type JsonObject } from "@destack/schema";
import { type ObjectReference } from "@destack/sync";
import { PackageServer } from "@destack/build/store";
import type { JsonCondition } from "@destack/db";
import { PackageName } from "@destack/package";
import { isServiceError, ServiceError } from "@destack/service/error";
import type { ServiceContext } from "@destack/service/server";
import { SpanStatusCode, telemetry } from "@destack/telemetry";
import { release, tag, type Release, type Tag } from "../object/index.ts";
import { NPM_PATH } from "../service/index.ts";
import type { ForgeServer } from "./server.ts";

/** Instruments for npm HTTP requests. */
const { tracer, logger } = telemetry.scope(import.meta.destack.package);

/** The rows one page of a list reads, the most a list returns. */
const PAGE_ROWS = 1000;

/** The input of one page of a package's releases or tags, in creation order. */
type ListInput = {
    /** The account with the package. */
    readonly accountId: ObjectReference["scope"];
    /** The package's rows meeting the conditions. */
    readonly where: JsonCondition;
    /** The creation order. */
    readonly orderBy: { readonly createdAt: "asc" };
    /** The rows of one page. */
    readonly limit: number;
    /** The cursor after the previous page, absent for the first. */
    readonly cursor?: string;
};

/** npm read endpoints over the forge's packages, read as the requesting caller. */
export class NpmServer {
    /** The canonical URL of the endpoints, independent of request Host headers, ending in a slash. */
    readonly url: URL;
    /** The forge serving the packages. */
    readonly #server: ForgeServer;

    /** Serve the endpoints below the npm path of the forge service's URL. */
    constructor(endpoint: URL, server: ForgeServer) {
        // require a plain HTTP URL, read as a directory
        if (
            !/^https?:$/u.test(endpoint.protocol) ||
            endpoint.username ||
            endpoint.password ||
            endpoint.search ||
            endpoint.hash
        ) {
            throw new TypeError("invalid registry URL");
        }
        this.url = new URL(`${endpoint.href.replace(/\/+$/u, "")}${NPM_PATH}`);
        this.#server = server;
    }

    /** Answer npm reads below the npm path, and nothing elsewhere. */
    async route(request: Request, context: ServiceContext): Promise<Response | undefined> {
        // leave paths outside the npm path
        if (!new URL(request.url).pathname.startsWith(NPM_PATH)) {
            return undefined;
        }

        return await tracer.startActiveSpan("forge.npm", async (span) => {
            try {
                return await this.#answer(request, context);
            } catch (error) {
                // answer client failures with the message npm prints and their code
                span.setStatus({ code: SpanStatusCode.ERROR });
                const headers = { "cache-control": "no-store" };
                if (isServiceError(error) && error.status < 500) {
                    return Response.json(
                        { error: error.message, code: error.code },
                        { status: error.status, headers },
                    );
                }

                // log other failures, concealing their details
                logger.emit({
                    severityText: "ERROR",
                    body: "Registry request failed",
                    attributes: {
                        "error.type": error instanceof Error ? error.name : typeof error,
                    },
                });

                return Response.json(
                    { error: "registry request failed", code: "INTERNAL_SERVER_ERROR" },
                    { status: 500, headers },
                );
            } finally {
                span.end();
            }
        });
    }

    /** Answer a package's documents and archives, as npm installs read them. */
    async #answer(request: Request, context: ServiceContext): Promise<Response> {
        // refuse writes, which the Destack release and tag calls take
        if (request.method !== "GET" && request.method !== "HEAD") {
            return Response.json(
                {
                    error: "publish, tag and deprecate releases through Destack",
                    code: "METHOD_NOT_SUPPORTED",
                },
                { status: 405, headers: { allow: "GET, HEAD", "cache-control": "no-store" } },
            );
        }

        // read the path below the npm path
        const pathname = new URL(request.url).pathname.slice(NPM_PATH.length);
        let path: string;
        try {
            path = decodeURIComponent(pathname);
        } catch (error) {
            if (!(error instanceof URIError)) {
                throw error;
            }
            throw new ServiceError("BAD_REQUEST", { message: "invalid URL encoding" });
        }

        // read the package and what the path selects below it
        const groups = /^(?<name>@[^/]+\/(?<local>[^/]+))(?:\/(?<selected>.*))?$/u.exec(
            path,
        )?.groups;
        const name = groups?.["name"];
        const local = groups?.["local"];
        if (name === undefined || local === undefined || !PackageName.safeParse(name).success) {
            return new Response(null, { status: 404 });
        }

        // find the package the caller may read, then answer an archive or a document
        const selected = groups?.["selected"];
        const found = await this.#server.find(name, context);
        const file =
            selected === undefined
                ? undefined
                : /^-\/(?<file>.+)\.tgz$/u.exec(selected)?.groups?.["file"];
        if (file !== undefined) {
            return this.#archive(request, context, found, local, file);
        }

        return this.#document(request, context, found, name, selected);
    }

    /** Stream a published release's archive, named by its version as npm names archives. */
    async #archive(
        request: Request,
        context: ServiceContext,
        found: ObjectReference,
        local: string,
        file: string,
    ): Promise<Response> {
        // select the release the file name's version names
        if (!file.startsWith(`${local}-`)) {
            throw new ServiceError("NOT_FOUND", { message: "package archive not found" });
        }
        const [selected] = await this.#list(
            async (input) => this.#server.objects.call(release, "list", input, context),
            found,
            [{ version: file.slice(local.length + 1) }, { unpublishedAt: { isNull: true } }],
        );
        if (selected === undefined) {
            throw new ServiceError("NOT_FOUND", { message: "package version or tag not found" });
        }

        // answer an unchanged archive without reading it
        const { digest, size } = selected.distribution;
        const etag = `"${digest}"`;
        const headers = new Headers({
            "content-type": "application/gzip",
            "content-length": String(size),
            "cache-control": "private, no-cache",
            etag,
            vary: "Authorization",
        });
        if (PackageServer.isUnchanged(request, etag)) {
            return new Response(null, { status: 304, headers });
        }
        // answer a head without the archive
        else if (request.method === "HEAD") {
            return new Response(null, { headers });
        }

        // stream the archive
        const object = await this.#server.store.open({
            path: "package.tgz",
            digest,
            size,
            mediaType: "application/gzip",
        });

        return new Response(object.body, { headers });
    }

    /** Answer a package's packument, or the version a tag or an exact version selects. */
    async #document(
        request: Request,
        context: ServiceContext,
        found: ObjectReference,
        name: string,
        selected: string | undefined,
    ): Promise<Response> {
        // describe each published release as npm installs it
        const [releases, tags] = await Promise.all([
            this.#list(
                async (input) => this.#server.objects.call(release, "list", input, context),
                found,
                [{ unpublishedAt: { isNull: true } }],
            ),
            this.#list(
                async (input) => this.#server.objects.call(tag, "list", input, context),
                found,
                [],
            ),
        ]);
        if (!releases.length) {
            throw new ServiceError("NOT_FOUND", { message: "package has no published releases" });
        }

        // answer one version a tag or an exact version selects, else the whole packument
        const versions = this.#versions(name, releases);
        const pointed = Object.fromEntries(tags.map((entry) => [entry.name, entry.version]));
        const document =
            selected === undefined
                ? { name, "dist-tags": pointed, versions, time: NpmServer.#time(releases, tags) }
                : NpmServer.#version(versions, pointed[selected] ?? selected);

        return NpmServer.#respond(request, document);
    }

    /** Describe each published release as npm installs it, by version. */
    #versions(name: string, releases: readonly Release[]): Record<string, JsonObject> {
        const local = name.slice(name.indexOf("/") + 1);

        return Object.fromEntries(
            releases.map((entry) => [
                entry.version,
                {
                    ...entry.metadata,
                    ...(entry.deprecation === null ? {} : { deprecated: entry.deprecation }),
                    dist: {
                        tarball: new URL(`${name}/-/${local}-${entry.version}.tgz`, this.url).href,
                        shasum: entry.distribution.shasum,
                        integrity: entry.distribution.integrity,
                        fileCount: entry.distribution.fileCount,
                        unpackedSize: entry.distribution.unpackedSize,
                    },
                },
            ]),
        );
    }

    /** Select one version's document, refusing an unknown version. */
    static #version(versions: Record<string, JsonObject>, version: string): JsonObject {
        const document = versions[version];
        if (document === undefined) {
            throw new ServiceError("NOT_FOUND", { message: "package version or tag not found" });
        }

        return document;
    }

    /** Time a package by its releases and tag moves, and each version by its release. */
    static #time(releases: readonly Release[], tags: readonly Tag[]): Record<string, string> {
        const stamps = [
            ...releases.map((entry) => entry.createdAt),
            ...tags.map((entry) => entry.updatedAt),
        ];

        return {
            created: new Date(Math.min(...stamps)).toISOString(),
            modified: new Date(Math.max(...stamps)).toISOString(),
            ...Object.fromEntries(
                releases.map((entry) => [entry.version, new Date(entry.createdAt).toISOString()]),
            ),
        };
    }

    /** Answer a JSON document tagged by its digest, without a body when unchanged or for a head. */
    static async #respond(request: Request, document: JsonObject): Promise<Response> {
        // tag the document by its digest
        const body = new TextEncoder().encode(JSON.stringify(document));
        const etag = `"${await Digest.of(body)}"`;
        const headers = new Headers({
            "content-type": "application/json",
            "cache-control": "private, no-cache",
            etag,
            vary: "Authorization",
        });

        // answer an unchanged document without a body
        if (PackageServer.isUnchanged(request, etag)) {
            return new Response(null, { status: 304, headers });
        }

        return new Response(request.method === "HEAD" ? null : body, { headers });
    }

    /** List a package's releases or tags meeting some conditions as the caller, in creation order. */
    async #list<Item>(
        read: (
            input: ListInput,
        ) => Promise<{ readonly items: Item[]; readonly cursor: string | null }>,
        found: ObjectReference,
        conditions: readonly JsonCondition[],
    ): Promise<Item[]> {
        // read every page
        const items: Item[] = [];
        let cursor: string | null = null;
        do {
            const page: { readonly items: Item[]; readonly cursor: string | null } = await read({
                accountId: found.scope,
                where: { AND: [{ parentId: found.id }, ...conditions] },
                orderBy: { createdAt: "asc" },
                limit: PAGE_ROWS,
                ...(cursor === null ? {} : { cursor }),
            });
            items.push(...page.items);
            cursor = page.cursor;
        } while (cursor !== null);

        return items;
    }
}
