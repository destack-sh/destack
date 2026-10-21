import { Digest, type JsonObject } from "@destack/schema";
import { type ObjectReference } from "@destack/sync";
import { isUnchanged } from "@destack/build/store";
import type { JsonCondition } from "@destack/db";
import { ListPage, type CallInput } from "@destack/object";
import type { ObjectStore } from "@destack/object/server";
import { PackageName } from "@destack/package";
import { CallError } from "@destack/object";
import type { RequestContext } from "@destack/object/server";
import { SpanStatusCode, telemetry } from "@destack/telemetry";
import { release, tag, type Release, type Tag } from "../object/index.ts";
import forgePackage from "../package.ts";
import type { Forge } from "./forge.ts";

/** Instruments for npm HTTP requests. */
const { tracer, logger } = telemetry.scope(import.meta.destack.package);

/** The rows one page of a list reads, the most a list returns. */
const PAGE_ROWS = 1000;

/** The input of one page of a package's releases or tags, in creation order. */
type ListInput = CallInput<typeof release, "list">;

/** npm read endpoints over the forge's packages, read as the requesting caller. */
export class NpmServer {
    /** The canonical URL of the endpoints, ending in a slash. */
    readonly url: URL;
    /** The forge serving the packages. */
    readonly #forge: Forge;
    /** The object store the packages' releases and tags are read through. */
    readonly #objects: ObjectStore;

    /** Serve the endpoints below the npm adapter of the forge's URL over an object store. */
    constructor(endpoint: URL, forge: Forge, objects: ObjectStore) {
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
        const path = forgePackage.adapters.npm.path;
        this.url = new URL(`${endpoint.href.replace(/\/+$/u, "")}${path}/`);
        this.#forge = forge;
        this.#objects = objects;
    }

    /** Answer an npm read, its path below the npm adapter's. */
    async answer(request: Request, context: RequestContext): Promise<Response> {
        return await tracer.startActiveSpan("forge.npm", async (span) => {
            try {
                return await this.#serve(request, context);
            } catch (error) {
                // answer client failures with the message npm prints and their code
                span.setStatus({ code: SpanStatusCode.ERROR });
                const headers = { "cache-control": "no-store" };
                const failure = CallError.of(error);
                if (failure !== undefined && failure.status < 500) {
                    return Response.json(
                        { error: failure.message, code: failure.code },
                        { status: failure.status, headers },
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
    async #serve(request: Request, context: RequestContext): Promise<Response> {
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

        // read the path below the npm adapter's
        const pathname = new URL(request.url).pathname.slice(1);
        let path: string;
        try {
            path = decodeURIComponent(pathname);
        } catch (error) {
            if (!(error instanceof URIError)) {
                throw error;
            }
            throw new CallError("BAD_REQUEST", { message: "invalid URL encoding" });
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

        // find the package the caller may read and answer an archive or a document
        const selected = groups?.["selected"];
        const found = await this.#forge.find(name, context, this.#objects);
        const file =
            selected === undefined
                ? undefined
                : /^-\/(?<file>.+)\.tgz$/u.exec(selected)?.groups?.["file"];
        if (file !== undefined) {
            return this.#archive(request, context, found, local, file);
        }

        return this.#document(request, context, found, name, selected);
    }

    /** Stream a published release's archive, which npm calls by the release's version. */
    async #archive(
        request: Request,
        context: RequestContext,
        found: ObjectReference,
        local: string,
        file: string,
    ): Promise<Response> {
        // select the release of the file name's version, yanked ones too for exact pins
        if (!file.startsWith(`${local}-`)) {
            throw new CallError("NOT_FOUND", { message: "package archive not found" });
        }
        const [selected] = await this.#list(
            async (input) =>
                this.#objects.query(context.access(found.scope), release.call("list", input)),
            found,
            [{ version: file.slice(local.length + 1) }],
        );
        if (selected === undefined) {
            throw new CallError("NOT_FOUND", { message: "package version or tag not found" });
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
        if (isUnchanged(request, etag)) {
            return new Response(null, { status: 304, headers });
        }
        // answer a head without the archive
        else if (request.method === "HEAD") {
            return new Response(null, { headers });
        }

        // stream the archive
        const object = await this.#forge.store.open({
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
        context: RequestContext,
        found: ObjectReference,
        name: string,
        selected: string | undefined,
    ): Promise<Response> {
        // describe each release new installs take, skipping yanked ones
        const [releases, tags] = await Promise.all([
            this.#list(
                async (input) =>
                    this.#objects.query(context.access(found.scope), release.call("list", input)),
                found,
                [{ yankedAt: { isNull: true } }],
            ),
            this.#list(
                async (input) =>
                    this.#objects.query(context.access(found.scope), tag.call("list", input)),
                found,
                [],
            ),
        ]);
        if (!releases.length) {
            throw new CallError("NOT_FOUND", { message: "package lacks releases to install" });
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
            throw new CallError("NOT_FOUND", { message: "package version or tag not found" });
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
        if (isUnchanged(request, etag)) {
            return new Response(null, { status: 304, headers });
        }

        return new Response(request.method === "HEAD" ? null : body, { headers });
    }

    /** List a package's releases or tags that meet some conditions, as the caller. */
    #list<Item>(
        read: (input: ListInput) => Promise<ListPage<Item>>,
        found: ObjectReference,
        conditions: readonly JsonCondition[],
    ): Promise<Item[]> {
        return Array.fromAsync(
            ListPage.all(read, {
                spaceId: found.scope,
                where: { AND: [{ parentId: found.id }, ...conditions] },
                orderBy: { createdAt: "asc" },
                limit: PAGE_ROWS,
            }),
        );
    }
}
