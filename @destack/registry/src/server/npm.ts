import { Digest } from "@destack/schema";
import { type ObjectReference } from "@destack/sync";
import { PackageServer } from "@destack/build/store";
import { Condition } from "@destack/db/query";
import type { ObjectType } from "@destack/object";
import { PackageName } from "@destack/package";
import { ServiceError } from "@destack/service/error";
import type { ServiceContext } from "@destack/service/server";
import { SpanStatusCode, telemetry } from "@destack/telemetry";
import { release, tag, type Release, type Tag } from "../object/index.ts";
import type { RegistryServer } from "./server.ts";

/** Instruments for npm HTTP requests. */
const { tracer, logger } = telemetry.scope(import.meta.destack.package);

/** The rows one page of a list reads, the most a list returns. */
const PAGE_ROWS = 1000;

/** npm read endpoints over the registry's objects, read as the requesting caller. */
export class NpmServer {
    /** The canonical URL of the endpoints, independent of request Host headers, ending in a slash. */
    readonly url: URL;
    /** The registry serving the packages. */
    readonly #server: RegistryServer;

    /** Serve the endpoints mounted at a URL. */
    constructor(url: URL, server: RegistryServer) {
        // require a plain HTTP URL, read as a directory
        if (
            !/^https?:$/.test(url.protocol) ||
            url.username ||
            url.password ||
            url.search ||
            url.hash
        ) {
            throw new TypeError("invalid registry URL");
        }
        this.url = new URL(url);
        if (!this.url.pathname.endsWith("/")) {
            this.url.pathname += "/";
        }
        this.#server = server;
    }

    /** Answer npm reads below the mount, and nothing elsewhere. */
    async route(request: Request, context: ServiceContext): Promise<Response | undefined> {
        // leave paths outside the mount
        if (!new URL(request.url).pathname.startsWith(this.url.pathname)) {
            return undefined;
        }

        return await tracer.startActiveSpan("registry.npm", async (span) => {
            try {
                return await this.#answer(request, context);
            } catch (error) {
                // answer client failures with their message, which npm prints, and their code
                span.setStatus({ code: SpanStatusCode.ERROR });
                const headers = { "cache-control": "no-store" };
                if (error instanceof ServiceError && error.status < 500) {
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
        // refuse writes, which Destack's own release and tag calls take
        if (request.method !== "GET" && request.method !== "HEAD") {
            return Response.json(
                {
                    error: "publish, tag and deprecate releases through Destack",
                    code: "METHOD_NOT_SUPPORTED",
                },
                { status: 405, headers: { allow: "GET, HEAD", "cache-control": "no-store" } },
            );
        }

        // read the path below the mount
        const pathname = new URL(request.url).pathname.slice(this.url.pathname.length);
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
        const match = /^(@[^/]+\/([^/]+))(?:\/(.*))?$/.exec(path);
        if (!match || !PackageName.safeParse(match[1]).success) {
            return new Response(null, { status: 404 });
        }

        // find the package the caller may read, then answer an archive or a document
        const name = match[1]!;
        const selected = match[3];
        const found = await this.#server.find(name, context);
        const archive = selected === undefined ? null : /^-\/(.+)\.tgz$/.exec(selected);
        if (archive) {
            return this.#archive(request, context, found, match[2]!, archive[1]!);
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
        const [selected] = await this.#list<Release>(release, found, context, [
            Condition.eq("version", file.slice(local.length + 1)),
            Condition.missing("unpublishedAt"),
        ]);
        if (selected === undefined) {
            throw new ServiceError("NOT_FOUND", { message: "package version or tag not found" });
        }

        // answer an unchanged archive and a head without reading it, else stream it
        const { digest, size } = selected.distribution;
        const headers = new Headers({
            "content-type": "application/gzip",
            "content-length": String(size),
            "cache-control": "private, no-cache",
            etag: `"${digest}"`,
            vary: "Authorization",
        });
        if (PackageServer.isUnchanged(request, headers.get("etag")!)) {
            return new Response(null, { status: 304, headers });
        } else if (request.method === "HEAD") {
            return new Response(null, { headers });
        }
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
            this.#list<Release>(release, found, context, [Condition.missing("unpublishedAt")]),
            this.#list<Tag>(tag, found, context, []),
        ]);
        if (!releases.length) {
            throw new ServiceError("NOT_FOUND", { message: "package has no published releases" });
        }
        const local = name.slice(name.indexOf("/") + 1);
        const pointed = Object.fromEntries(tags.map((entry) => [entry.name, entry.version]));
        const versions = Object.fromEntries(
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

        // select one version, else time the package by its releases and tag moves
        let document: object;
        if (selected !== undefined) {
            const version = versions[pointed[selected] ?? selected];
            if (!version) {
                throw new ServiceError("NOT_FOUND", {
                    message: "package version or tag not found",
                });
            }
            document = version;
        } else {
            const stamps = [
                ...releases.map((entry) => entry.createdAt),
                ...tags.map((entry) => entry.updatedAt),
            ];
            const time = {
                created: new Date(Math.min(...stamps)).toISOString(),
                modified: new Date(Math.max(...stamps)).toISOString(),
                ...Object.fromEntries(
                    releases.map((entry) => [
                        entry.version,
                        new Date(entry.createdAt).toISOString(),
                    ]),
                ),
            };
            document = { name, "dist-tags": pointed, versions, time };
        }

        // tag the document by its digest, answering an unchanged one without a body
        const body = new TextEncoder().encode(JSON.stringify(document));
        const etag = `"${await Digest.of(body)}"`;
        const headers = new Headers({
            "content-type": "application/json",
            "cache-control": "private, no-cache",
            etag,
            vary: "Authorization",
        });
        if (PackageServer.isUnchanged(request, etag)) {
            return new Response(null, { status: 304, headers });
        }

        return new Response(request.method === "HEAD" ? null : body, { headers });
    }

    /** List a package's releases or tags meeting some conditions as the caller, in creation order. */
    async #list<Row>(
        object: ObjectType,
        found: ObjectReference,
        context: ServiceContext,
        conditions: readonly Condition[],
    ): Promise<Row[]> {
        // read every page
        const rows: Row[] = [];
        let cursor: string | null = null;
        do {
            const page = (await this.#server.objects.query(
                object,
                "list",
                {
                    accountId: found.scope,
                    where: Condition.all(Condition.eq("parentId", found.id), ...conditions),
                    order: [{ column: "createdAt", direction: "asc" }],
                    limit: PAGE_ROWS,
                    ...(cursor === null ? {} : { cursor }),
                },
                context,
            )) as { readonly items: Record<string, unknown>[]; readonly cursor: string | null };
            rows.push(...page.items.map((item) => object.table.decode(item) as Row));
            cursor = page.cursor;
        } while (cursor !== null);

        return rows;
    }
}
