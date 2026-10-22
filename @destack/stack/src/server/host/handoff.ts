import type { Clock } from "@destack/environment";
import { Failure } from "@destack/error";
import { RequestId, Page, Scope, type Uplink } from "@destack/sync";
import { AccessContext, Caller, principal } from "@destack/access";
import { Digest, type Identifier, present, schema } from "@destack/schema";
import { createClient } from "@destack/object/client";
import { type DatabaseConnection, LogPosition, type Row, Snapshot, type Table } from "@destack/db";
import { type IdentityKeystore } from "@destack/account/identity";
import type { MethodContext } from "@destack/object";
import type { AdapterFetch } from "@destack/object/server";
import type { Runtime } from "../../runtime/index.ts";
import { type ResourceId, type Rewrapper, type ContentStore } from "@destack/resource";
import { Recipient } from "@destack/identity";
import { CallError } from "@destack/object";
import { type Fence, snapshot, type Transfer } from "../../object/index.ts";
import type { BuildStore } from "../installation/build.ts";
import { SpaceTransfer } from "./transfer.ts";
import type { PackageLocation } from "@destack/package/manifest";
import type { ResourceState } from "@destack/package/declare";
import stackPackage, { HANDOFF_PATH } from "../../package.ts";
import type { ProviderIndex } from "../resource/resource.ts";

/** How a target calls a source's handoff. */
interface HandoffCall {
    /** Abort the call. */
    readonly signal?: AbortSignal;
}

/** The paths of a space's handoff below the adapter: its rows, states, blobs and builds. */
const HandoffPath = {
    /** Write the path of a space's handoff part. */
    of(spaceId: string, part: "watch" | "capture" | "states" | "blobs" | "builds"): string {
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
    ) => Promise<{ readonly digest: Digest | null }>;
    /** Read a blob of the source machine's content store as it streams its bytes. */
    readonly read: (
        spaceId: Identifier<"space">,
        digest: Digest,
        options?: HandoffCall,
    ) => AsyncIterable<Uint8Array>;
    /** Locate a build the space runs in the source machine's build store. */
    readonly locate: (
        spaceId: Identifier<"space">,
        manifest: string,
    ) => {
        readonly location: PackageLocation;
        readonly fetch: (input: URL, init: RequestInit) => Promise<Response>;
    };
}

/** How a target reaches a source's handoff. */
export const Handoff = {
    /** Connect to a source's handoff through a fetch reaching its stack package at a URL. */
    connect(url: string, fetch: (request: Request) => Promise<Response>, clock: Clock): Handoff {
        // call the source's stack package and its handoff adapter below it
        const stack = createClient(stackPackage, { url, fetch });
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
                    requestId: RequestId.create(clock.now()),
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
            snapshot: (transfer, input) =>
                stack.transfer.snapshot({
                    spaceId: transfer.scope,
                    id: transfer.id,
                    requestId: RequestId.create(clock.now()),
                    ...input,
                }),
            read: async function* (spaceId, digest, options = {}) {
                const path = `${adapter}${HandoffPath.of(spaceId, "blobs")}/${digest}`;
                const response = await send(new Request(path, { signal: options.signal ?? null }));
                yield* present(response.body, `the blob ${digest}`);
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
    const lines = present(response.body, "the handoff's pages")
        .pipeThrough(new TextDecoderStream())
        .pipeThrough(splitLines());
    for await (const line of lines) {
        yield Page.parse(JSON.parse(line));
    }
}

/** Split a text stream into its non-empty lines. */
function splitLines(): TransformStream<string, string> {
    let pending = "";

    return new TransformStream({
        transform: (chunk, lines) => {
            // emit each complete line, keeping the partial one
            const parts = `${pending}${chunk}`.split("\n");
            pending = parts.pop() ?? "";
            for (const part of parts.filter((line) => line.length > 0)) {
                lines.enqueue(part);
            }
        },
        flush: (lines) => {
            if (pending.length > 0) {
                lines.enqueue(pending);
            }
        },
    });
}

/** Answer pages as NDJSON, one page per line. */
function ndjson(pages: AsyncIterable<Page>): Response {
    const lines = streamOf(linesOf(pages)).pipeThrough(new TextEncoderStream());

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

/** Write each page as one JSON line. */
async function* linesOf(pages: AsyncIterable<Page>): AsyncGenerator<string> {
    for await (const page of pages) {
        yield `${JSON.stringify(page)}\n`;
    }
}

/** What a source hands its spaces off with. */
export interface HandoffOptions {
    /** The space's database. */
    readonly database: DatabaseConnection;
    /** The account package's uplink, which places the spaces as this machine. */
    readonly publisher: Uplink;
    /** The providers keeping the spaces' resources. */
    readonly providers: ProviderIndex;
    /** The keystore of the spaces, wrapping their keys under the machine's keyring. */
    readonly keystore: IdentityKeystore;
    /** The runtimes running the spaces' instances. */
    readonly runtimes: readonly Runtime[];
    /** The machine's content-addressed store, keeping its resources' snapshots. */
    readonly store: Pick<ContentStore, "read">;
    /** The machine's store of every build. */
    readonly builds: BuildStore;
}

/** Serve a moving space's target its rows, the blobs its snapshots read and the builds it runs. */
export function handoffAdapter(options: HandoffOptions): AdapterFetch {
    const content = new HandoffContent(options.database);

    return async (request, admitted, context, store) => {
        // read the space and the part, refusing a caller acting for no machine the space moves to
        const caller = present(admitted, "the machine the handoff adapter admits");
        const url = new URL(request.url);
        const parsed = HandoffPath.parse(url.pathname);
        if (parsed === undefined) {
            throw new CallError("NOT_FOUND", { message: `no handoff at ${url.pathname}` });
        }
        const { spaceId, part, rest } = parsed;
        const record = await SourceHandoff.requireTarget(options.database, spaceId, caller);

        // stream the space's rows from a position, ending at a completed page once drained
        if (part === "watch" && request.method === "GET") {
            const { space } = await SpaceTransfer.plan(options, record);
            const written = url.searchParams.get("after");
            const after = written === null ? undefined : LogPosition.parse(JSON.parse(written));

            return ndjson(
                space.feed(store.clock).subscribe(space.replica.queries, after, request.signal, {
                    drain: context.signal,
                }),
            );
        }
        // answer the desired states the space's main database applied
        else if (part === "states" && request.method === "GET") {
            return Response.json(await SpaceTransfer.states(options.database, spaceId));
        }
        // capture every column of the fenced space, sealing machine-bound values to the recipient
        else if (part === "capture" && request.method === "POST") {
            const { space } = await SpaceTransfer.plan(options, record);
            await SourceHandoff.requireFenced(options.database, spaceId);
            const { recipient } = schema
                .object({ recipient: schema.base64() })
                .parse(await request.json());
            const { seal } = machineBound(options, Recipient.of(recipient));

            return ndjson(
                space.feed(store.clock).capture(space.replica.captured, request.signal, { seal }),
            );
        }
        // serve the builds the space's rows refer to
        else if (part === "builds" && request.method === "GET") {
            const mount = new URL(`${url.origin}${HandoffPath.of(spaceId, "builds")}/`);
            const answer = await options.builds.packages.fetch(request, mount, {
                authorize: async (_request, manifest) => {
                    await content.requireBuild(record, manifest);

                    return {};
                },
            });

            return answer ?? new Response(null, { status: 404 });
        }
        // stream a blob a snapshot of the space reads
        else if (part === "blobs" && request.method === "GET") {
            const digest = Digest.parse(rest.slice(1));
            await content.requireBlob(record, digest);

            return streamBlob(options.store, digest);
        }
        // refuse any other request
        else {
            throw new CallError("NOT_FOUND", { message: `no handoff at ${url.pathname}` });
        }
    };
}

/** Stream a blob of the content store. */
function streamBlob(store: Pick<ContentStore, "read">, digest: Digest): Response {
    const body = streamOf(store.read(digest));

    return new Response(body, { headers: { "content-type": "application/octet-stream" } });
}

/** The builds and blobs each moving space's target reads, read once per transfer. */
class HandoffContent {
    /** The space's database. */
    readonly #database: DatabaseConnection;
    /** The builds and blobs read last, by kind and space. */
    readonly #read = {
        builds: new Map<Identifier<"space">, HandoffRead>(),
        blobs: new Map<Identifier<"space">, HandoffRead>(),
    };

    /** Read a source's handoff content from the space's database. */
    constructor(database: DatabaseConnection) {
        this.#database = database;
    }

    /** Refuse a build none of the space's builds or revisions runs. */
    async requireBuild(record: Transfer, manifest: string): Promise<void> {
        if (!(await this.#includes(record, "builds", manifest))) {
            throw new CallError("NOT_FOUND", {
                message: `space ${record.scope} runs no build ${manifest}`,
            });
        }
    }

    /** Refuse a blob no snapshot of the space reads. */
    async requireBlob(record: Transfer, digest: Digest): Promise<void> {
        if (!(await this.#includes(record, "blobs", digest))) {
            throw new CallError("NOT_FOUND", {
                message: `no snapshot of ${record.scope} reads blob ${digest}`,
            });
        }
    }

    /** Report whether a transfer's builds or blobs include a value, read again on a miss. */
    async #includes(record: Transfer, kind: "builds" | "blobs", value: string): Promise<boolean> {
        // take what the transfer read before
        const known = this.#read[kind].get(record.scope);
        if (known?.transfer === record.id && known.values.has(value)) {
            return true;
        }

        // read it again, as the space may have added a build or taken a snapshot since
        const live = Snapshot.live(this.#database);
        const values =
            kind === "builds"
                ? await SpaceTransfer.manifests(live, record.scope)
                : await HandoffContent.#blobs(live, record.scope);
        this.#read[kind].set(record.scope, { transfer: record.id, values });

        return values.has(value);
    }

    /** Read the blobs the snapshots of a space read. */
    static async #blobs(live: Snapshot, spaceId: Identifier<"space">): Promise<Set<string>> {
        const snapshots = await live.read(snapshot.table).findMany({ where: { spaceId } });

        return new Set(snapshots.flatMap((row) => row.blobs));
    }
}

/** The builds or blobs a transfer's target reads, as read for the transfer. */
interface HandoffRead {
    /** The transfer read for. */
    readonly transfer: Identifier<"transfer">;
    /** The manifests of the builds the space runs, or the digests its snapshots read. */
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
            throw new CallError("NOT_FOUND", {
                message: `${spaceId} is transferring to no machine the caller acts for`,
            });
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
        // refuse a resource the space keeps no record of
        const found = await providers.find(context, input.spaceId, input.resourceId);
        if (found === undefined) {
            throw new CallError("NOT_FOUND", {
                message: `${input.spaceId} has no resource ${input.resourceId}`,
            });
        }

        // request the snapshot once as the space, else answer the digest of the requested one
        const [existing] = await context.read(snapshot.table).findMany({
            where: {
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

    /** Require a space fenced against writes. */
    async requireFenced(database: DatabaseConnection, spaceId: Identifier<"space">): Promise<void> {
        const chain = await Scope.chain(Snapshot.live(database), spaceId);
        if (Scope.fenced(chain) === undefined) {
            throw new CallError("PRECONDITION_FAILED", {
                message: `${spaceId} is not fenced yet`,
            });
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
