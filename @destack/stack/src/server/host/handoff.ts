import type { Environment } from "@destack/environment";
import {
    type BlobStore,
    type DatabaseConnection,
    LogPosition,
    type Row,
    Snapshot,
    type Table,
} from "@destack/db";
import { Failure } from "@destack/error";
import { RequestId, Page, Scope, type Uplink } from "@destack/sync";
import { AccessContext, Caller, principal } from "@destack/access";
import { Digest, type Identifier, present, schema } from "@destack/schema";
import { createClient } from "@destack/object/client";
import { type IdentityKeystore } from "@destack/account/identity";
import type { MethodContext } from "@destack/object";
import type { AdapterFetch } from "@destack/object/server";
import type { Runtime } from "../../runtime/index.ts";
import { ResourceId, type Rewrapper, type BlobEntry } from "@destack/resource";
import { Tarball } from "@destack/package/archive";
import { Recipient } from "@destack/identity";
import { type Fence, snapshot, type Transfer } from "../../object/index.ts";
import type { BuildStore } from "../installation/build.ts";
import { SpaceTransfer } from "./transfer.ts";
import type { PackageLocation } from "@destack/package/manifest";
import type { ResourceState } from "@destack/package/declare";
import stackPackage, { HANDOFF_PATH } from "../../package.ts";
import type { ProviderIndex } from "../resource/resource.ts";
import type {} from "@destack/package/import-meta";
import { StackError } from "../../error/index.ts";

/** How a target calls a source's handoff. */
interface HandoffCall {
    /** Abort the call. */
    readonly signal?: AbortSignal;
}

/** A taken snapshot's digest, as the source answers a target waiting for it. */
const TakenSnapshot = schema.object({
    /** The digest of the snapshot in the source's content store. */
    digest: Digest,
});

/** The paths of a space's handoff below the adapter: rows, states, snapshots, blobs and builds. */
const HandoffPath = {
    /** Write the path of a space's handoff part. */
    of(
        spaceId: string,
        part: "watch" | "capture" | "states" | "snapshots" | "blobs" | "builds",
    ): string {
        return `/${spaceId}/${part}`;
    },

    /** Read the space, the part and the rest of a path below the adapter, absent for another. */
    parse(
        path: string,
    ):
        | { readonly spaceId: Identifier<"space">; readonly part: string; readonly rest: string }
        | undefined {
        const [, spaceId, part, rest] = /^\/([^/]+)\/([a-z]+)(\/.*)?$/u.exec(path) ?? [];
        const space = schema.identifier("space").safeParse(spaceId);

        return !space.success || part === undefined
            ? undefined
            : { spaceId: space.data, part, rest: rest ?? "" };
    },
};

/** A source machine's handoff of the spaces it moves away, as a target calls it. */
export interface Handoff {
    /** Follow the space's rows from a position, or from a complete run without one. */
    readonly watch: (
        input: { readonly spaceId: Identifier<"space">; readonly after?: LogPosition },
        options?: HandoffCall,
    ) => AsyncIterable<Page>;
    /** Read the desired states the space's main database applied. */
    readonly states: (
        spaceId: Identifier<"space">,
        options?: HandoffCall,
    ) => Promise<ResourceState[]>;
    /** Fence the space for the transfer's target, answering its position and digest. */
    readonly fence: (transfer: Pick<Transfer, "scope" | "id">) => Promise<Fence>;
    /** Read the fenced space's rows whole, sealing machine-bound values to a recipient. */
    readonly capture: (
        input: { readonly spaceId: Identifier<"space">; readonly recipient: string },
        options?: HandoffCall,
    ) => AsyncIterable<Page>;
    /** Request a snapshot of a resource for a recipient, answering its digest once taken. */
    readonly snapshot: (
        transfer: Pick<Transfer, "scope" | "id">,
        input: { readonly resourceId: ResourceId; readonly recipient: string },
        options?: HandoffCall,
    ) => Promise<Digest>;
    /** List the blobs the space's rows reference that the source stores. */
    readonly blobs: (spaceId: Identifier<"space">, options?: HandoffCall) => Promise<Digest[]>;
    /** Read many blobs of the source machine's store in one request, each streamed in order. */
    readonly batch: (
        spaceId: Identifier<"space">,
        digests: readonly Digest[],
        options?: HandoffCall,
    ) => AsyncIterable<BlobEntry>;
    /** Locate a build the space runs in the source machine's build store. */
    readonly locate: (
        spaceId: Identifier<"space">,
        manifest: Digest,
    ) => {
        readonly location: PackageLocation;
        readonly fetch: (input: URL, init: RequestInit) => Promise<Response>;
    };
}

/** How a target connects to a source's handoff. */
export const Handoff = {
    /** Connect to a source's handoff through a fetch reaching its stack package at a URL. */
    connect(
        url: string,
        fetch: (request: Request) => Promise<Response>,
        environment: Environment,
    ): Handoff {
        // call the source's stack package and its handoff adapter below it
        const stack = createClient(stackPackage, { url, fetch }, environment);
        const adapter = `${url}${HANDOFF_PATH}`;
        const send = async (request: Request): Promise<Response> => {
            // refuse a failed answer as the source answered it
            const response = await fetch(request);
            if (!response.ok || response.body === null) {
                throw Failure.error(await Failure.read(response));
            }

            return response;
        };

        return {
            watch: async function* ({ spaceId, after }, options = {}) {
                const query =
                    after === undefined
                        ? ""
                        : `?after=${encodeURIComponent(JSON.stringify(after))}`;
                const path = `${adapter}${HandoffPath.of(spaceId, "watch")}${query}`;
                yield* pagesOf(await send(new Request(path, { signal: options.signal ?? null })));
            },
            states: async (spaceId, options = {}) => {
                const path = `${adapter}${HandoffPath.of(spaceId, "states")}`;
                const response = await send(new Request(path, { signal: options.signal ?? null }));

                return schema.array(schema.jsonObject()).parse(await response.json());
            },
            fence: (transfer) =>
                stack.transfer.fence({
                    spaceId: transfer.scope,
                    id: transfer.id,
                    requestId: RequestId.create(environment),
                }),
            capture: async function* ({ spaceId, recipient }, options = {}) {
                const path = `${adapter}${HandoffPath.of(spaceId, "capture")}`;
                const request = new Request(path, {
                    method: "POST",
                    headers: { "content-type": "application/json" },
                    body: JSON.stringify({ recipient }),
                    signal: options.signal ?? null,
                });
                yield* pagesOf(await send(request));
            },
            snapshot: async (transfer, input, options = {}) => {
                // request the snapshot once, answering one the source took already
                const requested = await stack.transfer.snapshot({
                    spaceId: transfer.scope,
                    id: transfer.id,
                    requestId: RequestId.create(environment),
                    ...input,
                });
                if (requested.digest !== null) {
                    return requested.digest;
                }

                // wait at the source until it took the snapshot
                const query = new URLSearchParams({ recipient: input.recipient });
                const path = `${adapter}${HandoffPath.of(transfer.scope, "snapshots")}/${encodeURIComponent(input.resourceId)}?${query}`;
                const response = await send(new Request(path, { signal: options.signal ?? null }));

                return TakenSnapshot.parse(await response.json()).digest;
            },
            blobs: async (spaceId, options = {}) => {
                const path = `${adapter}${HandoffPath.of(spaceId, "blobs")}`;
                const response = await send(new Request(path, { signal: options.signal ?? null }));

                return schema.array(Digest).parse(await response.json());
            },
            batch: async function* (spaceId, digests, options = {}) {
                // ask for the blobs in one request, reading the tarball of them as it streams
                const path = `${adapter}${HandoffPath.of(spaceId, "blobs")}`;
                const response = await send(
                    new Request(path, {
                        method: "POST",
                        headers: { "content-type": "application/json" },
                        body: JSON.stringify(digests),
                        signal: options.signal ?? null,
                    }),
                );
                const archive = present(response.body, "the blobs' tarball");
                for await (const entry of Tarball.read(archive, environment.compression)) {
                    yield { digest: Digest.parse(entry.path), size: entry.size, body: entry.body };
                }
            },
            locate: (spaceId, manifest) => ({
                location: {
                    manifest,
                    url: `${adapter}${HandoffPath.of(spaceId, "builds")}/${manifest}/`,
                },
                fetch: (input, options) => fetch(new Request(input.href, options)),
            }),
        };
    },
};

/** Read the pages of an NDJSON answer, one page per line. */
async function* pagesOf(response: Response): AsyncGenerator<Page> {
    // decode the body as it arrives, parsing each complete line as a page
    const decoder = new TextDecoder();
    let pending = "";
    for await (const chunk of present(response.body, "the handoff's pages")) {
        const lines = `${pending}${decoder.decode(chunk, { stream: true })}`.split("\n");
        pending = lines.pop() ?? "";
        for (const line of lines.filter((each) => each.length > 0)) {
            yield Page.parse(JSON.parse(line));
        }
    }

    // parse a last line without its newline
    const last = `${pending}${decoder.decode()}`;
    if (last.length > 0) {
        yield Page.parse(JSON.parse(last));
    }
}

/** Answer pages as NDJSON, one page per line. */
function ndjson(pages: AsyncIterable<Page>): Response {
    const lines = streamOf(linesOf(pages));

    return new Response(lines, { headers: { "content-type": "application/x-ndjson" } });
}

/** Stream the values of an iterable, ending it once the stream is cancelled. */
function streamOf<Value>(values: AsyncIterable<Value>): ReadableStream<Value> {
    const iterator = values[Symbol.asyncIterator]();

    return new ReadableStream<Value>({
        pull: async (stream) => {
            const next = await iterator.next();
            if (next.done === true) {
                stream.close();
            } else {
                stream.enqueue(next.value);
            }
        },
        cancel: async () => {
            await iterator.return?.();
        },
    });
}

/** Write each page as one encoded JSON line. */
async function* linesOf(pages: AsyncIterable<Page>): AsyncGenerator<Uint8Array> {
    const encoder = new TextEncoder();
    for await (const page of pages) {
        yield encoder.encode(`${JSON.stringify(page)}\n`);
    }
}

/** What a source hands its spaces off with. */
export interface HandoffOptions {
    /** The environment whose compression packs the blobs a target asks for. */
    readonly environment: Pick<Environment, "compression">;
    /** The space's database. */
    readonly database: DatabaseConnection;
    /** The account package's uplink, which places the spaces as this machine. */
    readonly publisher: Uplink;
    /** The providers that store the spaces' resources. */
    readonly providers: ProviderIndex;
    /** The keystore of the spaces, wrapping their keys under the machine's keyring. */
    readonly keystore: IdentityKeystore;
    /** The runtimes running the spaces' instances. */
    readonly runtimes: readonly Runtime[];
    /** The space's blobs, which a moving space's target reads its snapshots' blobs from. */
    readonly blobs: BlobStore;
    /** The space's builds, read through to the machine's cache. */
    readonly builds: BuildStore;
}

/** Serve a moving space's target its rows, the blobs its snapshots read and the builds it runs. */
export function handoffAdapter(options: HandoffOptions): AdapterFetch {
    const content = new HandoffContent(options.database, options.blobs);

    return async (request, admitted, context) => {
        // read the space and the part, refusing a caller acting for no machine the space moves to
        const caller = present(admitted, "the machine the handoff adapter admits");
        const url = new URL(request.url);
        const parsed = HandoffPath.parse(url.pathname);
        if (parsed === undefined) {
            throw StackError.of("HANDOFF_NOT_FOUND", `no handoff at ${url.pathname}`);
        }
        const { spaceId, part, rest } = parsed;
        const record = await SourceHandoff.requireTarget(options.database, spaceId, caller);

        // stream the space's rows from a position, ending at a completed page once drained
        if (part === "watch" && request.method === "GET") {
            const { space } = await SpaceTransfer.plan(options, record);
            const written = url.searchParams.get("after");
            const after = written === null ? undefined : LogPosition.parse(JSON.parse(written));

            return ndjson(
                space.feed().subscribe(space.replica.queries, after, request.signal, {
                    drain: context.signal,
                }),
            );
        }
        // answer the desired states the space's main database applied
        else if (part === "states" && request.method === "GET") {
            return Response.json(await SpaceTransfer.states(options, spaceId));
        }
        // capture every column of the fenced space, sealing machine-bound values to the recipient
        else if (part === "capture" && request.method === "POST") {
            const { space } = await SpaceTransfer.plan(options, record);
            await SourceHandoff.requireFenced(options.database, spaceId);
            const { recipient } = schema
                .object({ recipient: schema.base64() })
                .parse(await request.json());
            const { seal } = machineBound(
                options,
                Recipient.of(recipient, options.keystore.environment),
            );

            return ndjson(space.feed().capture(space.replica.captured, request.signal, { seal }));
        }
        // serve the builds the space's rows refer to
        else if (part === "builds" && request.method === "GET") {
            const mount = new URL(`${url.origin}${HandoffPath.of(spaceId, "builds")}/`);
            const answer = await options.builds.space.fetch(request, mount, {
                authorize: async (_request, manifest) => {
                    await content.requireBuild(record, manifest);
                },
            });

            return answer ?? new Response(null, { status: 404 });
        }
        // answer a requested snapshot's digest once the source took it
        else if (part === "snapshots" && request.method === "GET") {
            const resourceId = ResourceId.parse(decodeURIComponent(rest.slice(1)));
            const recipient = present(
                url.searchParams.get("recipient"),
                "the snapshot's recipient",
            );
            const digest = await SourceHandoff.taken(
                options.database,
                { spaceId, resourceId, recipient },
                request.signal,
            );

            return Response.json({ digest });
        }
        // list the blobs the space's rows reference that this machine stores
        else if (part === "blobs" && rest === "" && request.method === "GET") {
            return Response.json([...(await content.blobs(record))]);
        }
        // stream the asked blobs the space's rows reference as one tarball named by digest
        else if (part === "blobs" && rest === "" && request.method === "POST") {
            const digests = schema.array(Digest).parse(await request.json());
            for (const digest of digests) {
                await content.requireBlob(record, digest);
            }
            const entries = (async function* () {
                for await (const blob of options.blobs.storage.batch(digests)) {
                    yield { path: blob.digest, size: blob.size, body: blob.body };
                }
            })();

            return new Response(Tarball.stream(entries, options.environment.compression), {
                headers: { "content-type": "application/gzip" },
            });
        }
        // refuse any other request
        else {
            throw StackError.of("HANDOFF_NOT_FOUND", `no handoff at ${url.pathname}`);
        }
    };
}

/** The builds and blobs each moving space's target reads, read once per transfer. */
class HandoffContent {
    /** The space's database. */
    readonly #database: DatabaseConnection;
    /** The space's blobs, which its rows reference. */
    readonly #blobs: BlobStore;
    /** The builds and blobs read last, by kind and space. */
    readonly #read = {
        builds: new Map<Identifier<"space">, HandoffRead>(),
        blobs: new Map<Identifier<"space">, HandoffRead>(),
    };

    /** Read a source's handoff content from the space's database and blobs. */
    constructor(database: DatabaseConnection, blobs: BlobStore) {
        this.#database = database;
        this.#blobs = blobs;
    }

    /** Refuse a build none of the space's builds or revisions runs. */
    async requireBuild(record: Transfer, manifest: Digest): Promise<void> {
        if (!(await this.#includes(record, "builds", manifest))) {
            throw StackError.of(
                "BUILD_NOT_RUNNING",
                `space ${record.scope} does not run build ${manifest}`,
            );
        }
    }

    /** Refuse a blob no row of the space references, or one this machine lacks. */
    async requireBlob(record: Transfer, digest: Digest): Promise<void> {
        if (!(await this.#includes(record, "blobs", digest))) {
            throw StackError.of(
                "BLOB_NOT_FOUND",
                `no row of ${record.scope} references a blob ${digest} stored here`,
            );
        }
    }

    /** List the blobs the space's rows reference that this machine stores. */
    blobs(record: Transfer): Promise<ReadonlySet<string>> {
        return this.#reread(record, "blobs");
    }

    /** Report whether a transfer's builds or blobs include a value, read again on a miss. */
    async #includes(record: Transfer, kind: "builds" | "blobs", value: string): Promise<boolean> {
        // take what the transfer read before, else read it again
        const known = this.#read[kind].get(record.scope);

        return (
            (known?.transfer === record.id && known.values.has(value)) ||
            (await this.#reread(record, kind)).has(value)
        );
    }

    /** Read a transfer's builds or blobs again, which the space may have added since. */
    async #reread(record: Transfer, kind: "builds" | "blobs"): Promise<ReadonlySet<string>> {
        // read the builds the space runs or the blobs its rows reference, and remember them
        const live = Snapshot.live(this.#database);
        const values: ReadonlySet<string> =
            kind === "builds"
                ? await SpaceTransfer.manifests(live, record.scope)
                : await this.#stored();
        this.#read[kind].set(record.scope, { transfer: record.id, values });

        return values;
    }

    /** Read the blobs the space's rows reference that this machine stores. */
    async #stored(): Promise<ReadonlySet<string>> {
        const referenced = await this.#blobs.list();
        const missing = new Set<string>(await this.#blobs.storage.missing(referenced));

        return new Set<string>(referenced.filter((digest) => !missing.has(digest)));
    }
}

/** The builds or blobs a transfer's target reads, as read for the transfer. */
interface HandoffRead {
    /** The transfer read for. */
    readonly transfer: Identifier<"transfer">;
    /** The manifests of the builds the space runs, or the blobs its rows reference. */
    readonly values: ReadonlySet<string>;
}

/** The source machine's side of a space's handoff. */
export const SourceHandoff = {
    /** Read a space's active transfer, refusing a caller acting for no machine it targets. */
    async requireTarget(
        database: DatabaseConnection,
        spaceId: Identifier<"space">,
        caller: Caller,
    ): Promise<Transfer> {
        // read the active transfer and refuse a caller acting for none of its machines
        const record = await SpaceTransfer.readActive(database, spaceId);
        const acting = new Set([caller.subject, ...caller.actors].map((subject) => subject.id));
        if (!acting.has(record.targetId)) {
            throw StackError.of(
                "TRANSFER_TO_CALLER_MISSING",
                `${spaceId} is not transferring to a machine the caller acts for`,
            );
        }

        return record;
    },

    /** Request a snapshot of a space's resource for a recipient once, answering its digest. */
    async snapshot(
        providers: ProviderIndex,
        context: Pick<MethodContext, "mutate" | "now" | "read">,
        input: {
            readonly spaceId: Identifier<"space">;
            readonly resourceId: ResourceId;
            readonly recipient: string;
        },
    ): Promise<{ readonly digest: Digest | null }> {
        // refuse a resource without a record in the space
        const found = await providers.find(context, input.spaceId, input.resourceId);
        if (found === undefined) {
            throw StackError.of(
                "RESOURCE_NOT_FOUND",
                `${input.spaceId} has no resource ${input.resourceId}`,
            );
        }

        // request the snapshot once as the space, else answer the digest of the requested one
        const [existing] = await context.read(snapshot.table).findMany({
            where: {
                spaceId: input.spaceId,
                resourceId: input.resourceId,
                recipient: input.recipient,
            },
        });
        if (existing === undefined) {
            const itself = principal.space.reference(Scope.universe.id, input.spaceId);
            await context.mutate(AccessContext.of(Caller.of(itself), context.now), [
                snapshot.call("create", input),
            ]);

            return { digest: null };
        }

        return { digest: existing.digest };
    },

    /** Wait until a requested snapshot of a space's resource is taken, answering its digest. */
    async taken(
        database: DatabaseConnection,
        input: {
            readonly spaceId: Identifier<"space">;
            readonly resourceId: ResourceId;
            readonly recipient: string;
        },
        signal: AbortSignal,
    ): Promise<Digest> {
        // read the requested snapshot after each commit to the snapshots until it has a digest
        const read = async () => {
            const [row] = await Snapshot.live(database)
                .read(snapshot.table)
                .findMany({
                    where: {
                        spaceId: input.spaceId,
                        resourceId: input.resourceId,
                        recipient: input.recipient,
                    },
                });
            if (row === undefined) {
                throw StackError.of(
                    "SNAPSHOT_NOT_REQUESTED",
                    `${input.spaceId} has no snapshot of ${input.resourceId} requested for the recipient`,
                );
            }

            return row.digest;
        };
        await database.log.until(async () => (await read()) !== null, signal);
        signal.throwIfAborted();

        return present(await read(), `the snapshot of ${input.resourceId}`);
    },

    /** Require a space fenced against writes. */
    async requireFenced(database: DatabaseConnection, spaceId: Identifier<"space">): Promise<void> {
        const chain = await Scope.chain(Snapshot.live(database), spaceId);
        if (Scope.fenced(chain) === undefined) {
            throw StackError.of("FENCE_REQUIRED", `${spaceId} is not fenced yet`);
        }
    },
};

/** Encrypt the providers' machine-bound rows and space keys to a recipient, and decrypt them. */
export function machineBound(
    options: Pick<HandoffOptions, "providers" | "keystore">,
    recipient: Recipient,
): Record<"seal" | "open", (table: Table, row: Row) => Promise<Row>> {
    const bound: Rewrapper<Table, Row>[] = [
        ...options.providers.machineBound(),
        options.keystore.rewrap,
    ];
    const rewrapper = (table: Table) => bound.find((entry) => entry.table === table);

    return {
        seal: async (table, row) => rewrapper(table)?.seal(row, recipient) ?? row,
        open: async (table, row) => rewrapper(table)?.open(row, recipient) ?? row,
    };
}
