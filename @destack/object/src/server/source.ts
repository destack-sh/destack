import { AuditCall } from "@destack/audit";
import { Scope, ObjectReference, type Subject } from "@destack/sync";
import { LogPosition, Snapshot } from "@destack/db/log";

import { TABLE, type JsonValue, type Table } from "@destack/db";
import { Condition } from "@destack/db/query";

import { Digest, Duration, schema } from "@destack/schema";

import { ServiceError } from "@destack/service/error";

import { type ServiceContext } from "@destack/service/server";
import { withEventMeta } from "@destack/service";
import * as sync from "@destack/sync";

import { type ObjectQuery } from "../replica/replica.ts";
import { ObjectType, REPLICATE, REPRESENT } from "../object/object.ts";

import { Authorization } from "./authorization.ts";

import { EphemeralStorage } from "./ephemeral.ts";
import { ObjectAudience } from "./audience.ts";

import type { Controller } from "@destack/service/control";

import { until } from "@destack/service/timer";

import type { ObjectReplicas, ObjectServer } from "./server.ts";

/** The reserved name of the query following a caller's journal entries. */
const JOURNAL_QUERY = "#journal";
/** The largest broadcast event, in bytes: 16 KiB, far above the tens of bytes a cursor takes. */
const BROADCAST_BYTES = 16 * 1024;

/** The copies a server's objects stream: client feeds of a scope's queries, and replicas of scopes for databases below. */
export class ObjectSource {
    /** The server whose objects the copies hold. */
    readonly server: ObjectServer;

    /** Stream the copies of a server's objects. */
    constructor(server: ObjectServer) {
        this.server = server;
    }

    /**
     * Stream a copy to the caller asking for it.
     *
     * The copy is decided for the principal a served object stands for, where the caller may represent it.
     * Any other copy is decided for the caller, which copies a chain only with the scope's `replicate` permission.
     */
    async *relayed(
        request: sync.ReplicaRequest,
        context: ServiceContext,
    ): AsyncGenerator<sync.QueryPage> {
        // decide the copy for the principal a served object stands for, which the caller represents
        const standing = await this.#standing(request.below);
        let follower: ReplicaFollower;
        if (standing !== undefined) {
            const { object, reference, subject, parent } = standing;
            const authorization = await this.server.authorize(
                this.server.database,
                reference.scope,
                context,
            );
            await authorization.require(object.permission(REPRESENT), reference);
            follower = { subject, ...(parent === undefined ? {} : { parent }) };
        }
        // decide any other copy for the caller, which is the scope's principal or lives in the scope
        else {
            const { subjects } = this.server.accessContext(context, request.below);
            const kept = subjects.filter(
                (subject) => subject.id === request.below || subject.scope === request.below,
            );
            if (kept.length === 0) {
                throw new ServiceError("FORBIDDEN", {
                    message: `the caller keeps no copies for ${request.below}`,
                });
            }

            // copy a chain only with the permission on the caller's own object, or else on the scope's
            if (request.access) {
                const authorization = await this.server.authorize(
                    this.server.database,
                    request.below,
                    context,
                );
                const own = kept.flatMap((subject) =>
                    this.server.durable
                        .filter(
                            (object) =>
                                object.policy.is(subject) &&
                                (object.permissions as readonly string[]).includes(REPLICATE),
                        )
                        .map((object) => ({
                            permission: object.permission(REPLICATE),
                            target: object.reference(subject.scope, subject.id),
                        })),
                );
                const scope = await Scope.object(
                    Snapshot.live(this.server.database),
                    request.below,
                );
                const policy = this.server.authorizer.policy(scope);
                const granting = [
                    ...own,
                    ...(Object.hasOwn(policy.definition.permissions, REPLICATE)
                        ? [{ permission: policy.permission(REPLICATE), target: scope }]
                        : []),
                ];
                const [first] = granting;
                if (first === undefined) {
                    throw new ServiceError("FORBIDDEN", {
                        message: `no ${scope.type} keeps copies of its chain`,
                    });
                }
                await authorization.require(first.permission, first.target);
            }
            follower = { context };
        }
        yield* this.replicate(request, follower, context.request.signal, context.signal);
    }

    /** Find the object standing for a scope as a principal among the served types the database is home of, with the scope containing it. */
    async #standing(below: string): Promise<
        | {
              readonly object: ObjectType;
              readonly reference: ObjectReference;
              readonly subject: Subject;
              readonly parent: string | undefined;
          }
        | undefined
    > {
        for (const object of this.server.durable) {
            // skip the types nobody represents, copies, and the types with other identifiers
            const table = object.table as Table;
            const identifier = table[TABLE].columns.id?.definition.schema;
            if (
                !(object.permissions as readonly string[]).includes(REPRESENT) ||
                this.server.database.copies(table) ||
                identifier?.safeParse(below).success !== true
            ) {
                continue;
            }

            // read the object and the scope containing what it stands for, as a scope row's parent
            const [row] = await Snapshot.live(this.server.database).select(
                table,
                ["id"],
                [[below]],
            );
            const parent = row?.parent;
            // stand for the principal the object's identifier is, as a user's `self` is
            const held = Object.entries(object.mapping.relations).find(
                ([, field]) => field.column === object.mapping.id,
            );
            const stood =
                row === undefined || held === undefined
                    ? undefined
                    : this.server.authorizer.related(object.mapping, held[0], row);
            if (row !== undefined && stood !== undefined) {
                return {
                    object,
                    reference: object.reference(String(row.scope), below),
                    subject: stood,
                    parent: typeof parent === "string" ? parent : undefined,
                };
            }
        }

        return undefined;
    }

    /** Follow queries of a scope's objects from a resumed position. */
    sync(
        scope: string,
        context: ServiceContext,
        options: SyncOptions = {},
    ): AsyncGenerator<sync.QueryPage> {
        return options.client === undefined
            ? this.#syncDurable(scope, context, options)
            : this.#syncEphemeral(scope, context, options.client, options);
    }

    /** Follow queries of a scope's durable objects with the caller's journal entries. */
    async *#syncDurable(
        scope: string,
        context: ServiceContext,
        { after, queries, previous, refresh }: SyncOptions,
    ): AsyncGenerator<sync.QueryPage> {
        // enter and open the audience
        await this.server.enter(context, scope);
        const audience = await ObjectAudience.open(this.server, scope, context);

        // follow the caller's journal
        const caller = context.authentication?.id;
        const journal: Record<string, sync.Query> =
            caller === undefined
                ? {}
                : {
                      [JOURNAL_QUERY]: {
                          table: this.server.journal.table,
                          scopes: [scope],
                          where: Condition.eq("caller", caller),
                      },
                  };

        // follow the queries beside the journal
        const { chain } = audience;
        const compiled = { ...ObjectType.queries(this.server.durable, queries, chain), ...journal };
        const earlier =
            previous === undefined
                ? undefined
                : { ...ObjectType.queries(this.server.durable, previous, chain), ...journal };
        const table = this.server.journal.table[TABLE].sqlName;
        const follow = async function* (feed: sync.Feed): AsyncGenerator<sync.QueryPage> {
            for await (const page of feed.subscribe(compiled, after, context.request.signal, {
                audience,
                drain: context.signal,
                ...(earlier === undefined ? {} : { previous: earlier }),
                ...(refresh === undefined ? {} : { every: Duration.milliseconds(refresh.every) }),
            })) {
                yield withOutcomes(page, table, audience.chain);
            }
        };

        // audit watching each audited object type
        const tables = new Set(Object.values(compiled).flatMap(tablesOf));
        const audited = this.server.objects.filter(
            (object) =>
                (object.isReadAudited || this.server.isAccessAudited) && tables.has(object.table),
        );
        const pages = audited.reduce(
            (source, object) => () => {
                const { action, values } = object.auditCall("watch", {}, scope);

                return this.server.audit(scope, context).stream(action, values, source);
            },
            () => follow(this.server.feed),
        );
        try {
            yield* pages();
        } catch (error) {
            // report capacity failures as service failures
            throw serviceFailure(error);
        }
    }

    /** Follow queries of a scope's ephemeral objects as a client, restarting when access changes. */
    async *#syncEphemeral(
        scope: string,
        context: ServiceContext,
        client: string,
        { after, queries, previous, refresh }: SyncOptions,
    ): AsyncGenerator<sync.QueryPage> {
        // enter and hold the client's rows
        const store = this.server.store();
        await this.server.enter(context, scope);
        const release = store.hold(EphemeralStorage.clientKey(context, client));
        const compiled = ObjectType.queries(store.objects, queries, [scope]);
        let earlier =
            previous === undefined
                ? undefined
                : ObjectType.queries(store.objects, previous, [scope]);
        try {
            let position = after;
            while (!context.signal.aborted) {
                // open the audience and watch access rows until the request closes or access changes
                const revised = new AbortController();
                const signal = AbortSignal.any([context.request.signal, revised.signal]);
                const since = (await this.server.database.log.position()).sequence;
                const audience = await ObjectAudience.open(
                    this.server,
                    scope,
                    context,
                    "ephemeral",
                );
                const { authorization, chain } = audience;
                const tables = this.server.authorizer.watch(chain).map((watch) => watch.table);
                let failure: unknown;
                const watching = (async () => {
                    for await (const page of this.server.database.log.follow(
                        { tables, scopes: chain, after: since },
                        signal,
                    )) {
                        if (page.changes.length > 0) {
                            revised.abort();
                        }
                    }
                })().catch((error: unknown) => {
                    failure = error;
                    revised.abort();
                });

                // follow until access changes, or to a completed page once the caller lapses
                const pages = store.feed.subscribe(compiled, position, signal, {
                    audience,
                    drain: context.signal,
                    ...(earlier === undefined ? {} : { previous: earlier }),
                    ...(refresh === undefined
                        ? {}
                        : { every: Duration.milliseconds(refresh.every) }),
                });
                yield* this.#interleave(pages, scope, authorization, signal);
                revised.abort();
                await watching;
                if (failure !== undefined) {
                    throw failure;
                }
                position = undefined;
                earlier = undefined;
            }
        } catch (error) {
            // report capacity failures as service failures
            throw serviceFailure(error);
        } finally {
            release();
        }
    }

    /** Send an unstored event to an object's readers. */
    async broadcast(
        scope: string,
        target: Omit<ObjectReference, "scope">,
        event: JsonValue,
        context: ServiceContext,
    ): Promise<void> {
        // require a small event and a reader
        const store = this.server.store();
        if (new TextEncoder().encode(JSON.stringify(event)).length > BROADCAST_BYTES) {
            throw new ServiceError("BAD_REQUEST", {
                message: `a broadcast holds at most ${BROADCAST_BYTES} bytes`,
            });
        }
        await this.server.enter(context, scope);
        const reference = { ...target, scope };
        const authorization = await this.server.admit(this.server.database, scope, context);
        if (!(await this.#lists(authorization, reference))) {
            throw new ServiceError("NOT_FOUND", { message: `no ${target.type} ${target.id}` });
        }

        // send it to the scope's streams
        store.tracker.broadcast(scope, { ...reference, event });
    }

    /** Build the controller following each requested copy from its source, one key per request. */
    controller(replicas: ObjectReplicas): Controller {
        // list the requests again once the scopes above change
        const keyOf = async (request: Omit<sync.ReplicaRequest, "after">) =>
            `${request.name} ${request.scope} ${await Digest.json(request)}`;

        return {
            name: "replica",
            mode: "follow",
            watches: [Scope.table],
            concurrency: Infinity,
            list: async () => Promise.all((await replicas.requests()).map(keyOf)),
            reconcile: async (key, { signal }) => {
                // find the key's request, waiting for the loop to stop one no longer listed
                const requests = await replicas.requests();
                const keys = await Promise.all(requests.map(keyOf));
                const request = requests[keys.indexOf(key)];
                if (request === undefined) {
                    await until(signal);

                    return undefined;
                }

                // follow the copy from the position it reached for the same request
                const replica = this.server.authorizer.replicaOf(request);
                await replica.follow(
                    this.server.database,
                    (after, stream) =>
                        replicas.source.stream(
                            { ...request, ...(after === undefined ? {} : { after }) },
                            stream,
                        ),
                    signal,
                    { request },
                );

                return undefined;
            },
        };
    }

    /**
     * List the rows of the universe a scope reads: every row of each copied object type living in the universe that its principal may read.
     *
     * A copied type living in the scopes other listed types' rows are is copied across scopes, within those types.
     */
    universeRows(): sync.ReplicaRequest["rows"] {
        // list the copied types of the universe first
        const reference = (object: ObjectType) => ({
            packageId: object.policy.definition.packageId,
            type: object.policy.definition.name,
        });
        const copied = this.server.durable.filter((object) =>
            this.server.database.copies(object.table),
        );
        const listed = copied.filter((object) => object.scopes.length === 0);
        const rows: sync.ReplicaRequest["rows"][number][] = listed.map((object) => ({
            type: reference(object),
            where: Condition.all(),
        }));

        // add the types living in the scopes of listed types, at any depth
        for (let isGrowing = true; isGrowing;) {
            isGrowing = false;
            for (const object of copied.filter((candidate) => !listed.includes(candidate))) {
                const scopes = listed.filter((scope) =>
                    object.scopes.some((type) => type.same(scope)),
                );
                if (scopes.length > 0) {
                    rows.push({
                        type: reference(object),
                        where: Condition.all(),
                        within: scopes.map(reference),
                    });
                    listed.push(object);
                    isGrowing = true;
                }
            }
        }

        return rows;
    }

    /** List the requests of the copies a database keeps for a scope: its chain, and the rows of the universe it reads. */
    async replicaRequests(
        below: string,
        options: { readonly isHome: boolean },
    ): Promise<Omit<sync.ReplicaRequest, "after">[]> {
        // copy the chain, then the rows of the universe the scope reads
        const chain = await this.server.authorizer.chain(this.server.database, below, options);
        const universe = this.universeRequest(below);

        return universe === undefined ? chain : [...chain, universe];
    }

    /** Build the request of the copy of the universe's rows a scope or cell reads, absent when the server copies no type living there. */
    universeRequest(below: string): Omit<sync.ReplicaRequest, "after"> | undefined {
        // request nothing where no global object type is served
        const rows = this.universeRows();
        if (rows.length === 0) {
            return undefined;
        }

        // copy the universe's rows the scope or cell may read, under its own name
        return {
            name: below,
            scope: Scope.universe.id,
            below,
            access: false,
            held: [],
            copied: [],
            rows,
        };
    }

    /**
     * Stream a copy's pages to a database below.
     *
     * A chain's rows are decided by containment, with the guarded fields of the scope's own row decided for the follower.
     * The rows of the universe are decided for their reader.
     */
    async *replicate(
        request: sync.ReplicaRequest,
        follower: ReplicaFollower,
        signal: AbortSignal,
        drain?: AbortSignal,
    ): AsyncGenerator<sync.QueryPage> {
        // require the copied scope to contain the follower's for a chain, through its parent when held elsewhere
        if (request.access) {
            const parent = "subject" in follower ? follower.parent : undefined;
            const chain = await Scope.chain(
                Snapshot.live(this.server.database),
                parent ?? request.below,
            );
            const scopes = [
                ...(parent === undefined ? [] : [request.below]),
                ...chain.map((link) => link.object.id),
            ];
            if (!scopes.includes(request.scope)) {
                throw new ServiceError("NOT_FOUND", {
                    message: `${request.scope} does not contain ${request.below}`,
                });
            }
        }

        // relay a copied scope only once its copy holds a position
        try {
            await sync.Replica.requireRelayable(this.server.database, request.name, request.scope);
        } catch (error) {
            if (error instanceof sync.SyncError && error.code === "STALE") {
                throw new ServiceError("SERVICE_UNAVAILABLE", { message: error.message });
            }
            throw error;
        }

        // decide a chain by containment, and the universe's rows for a principal where they live or for a caller of the space it relays them to
        let audience: sync.Audience;
        if (request.access) {
            audience = await ObjectAudience.contained(
                this.server,
                "subject" in follower ? follower.subject : follower.context,
                request.below,
            );
        }
        // decide a caller where the rows live: in the scope a relayed copy is kept for, or in the copied scope at its home
        else {
            const isRelayed = await sync.Replica.isCopied(this.server.database, request.scope);
            audience =
                "subject" in follower
                    ? await ObjectAudience.of(this.server, request.scope, follower.subject)
                    : await ObjectAudience.open(
                          this.server,
                          isRelayed ? request.below : request.scope,
                          follower.context,
                      );
        }

        // stream the copy, ending at a completed page once drained
        const copy = this.server.authorizer.replicaOf(request);
        yield* this.server.feed.subscribe(copy.queries, request.after, signal, {
            audience,
            ...(drain === undefined ? {} : { drain }),
        });
    }

    /** Decide whether a caller may list a served durable object. */
    async #lists(authorization: Authorization, reference: ObjectReference): Promise<boolean> {
        const object = this.server.objects.find(
            (entry) =>
                entry.storage === "durable" &&
                entry.name === reference.type &&
                entry.policy.definition.packageId === reference.packageId,
        );
        const permission = object?.listing;

        return (
            permission !== undefined && (await authorization.check(permission, reference)).isAllowed
        );
    }

    /** Interleave a stream's pages with the readable objects' events. */
    async *#interleave(
        pages: AsyncGenerator<sync.QueryPage>,
        scope: string,
        authorization: Authorization,
        signal: AbortSignal,
    ): AsyncGenerator<sync.QueryPage> {
        // collect the scope's events
        const received: JsonValue[] = [];
        const readable = new Map<string, boolean>();
        let wake = () => {};
        let stop = () => {};
        let last: sync.QueryPage | undefined;
        let next = pages.next();
        try {
            while (!signal.aborted) {
                // yield the next page unless events wait
                const arrived =
                    received.length > 0
                        ? undefined
                        : await Promise.race([
                              next,
                              new Promise<undefined>(
                                  (resolve) => (wake = () => resolve(undefined)),
                              ),
                          ]);
                if (arrived !== undefined) {
                    if (arrived.done) {
                        return;
                    }
                    if (last === undefined) {
                        stop = this.server.store().tracker.listen(scope, (event) => {
                            received.push(event);
                            wake();
                        });
                    }
                    last = arrived.value;
                    next = pages.next();
                    yield arrived.value;
                    continue;
                }

                // yield the readable events
                const broadcasts: sync.Broadcast[] = [];
                for (const sent of received.splice(0)) {
                    const { event, ...reference } = sent as ObjectReference & { event: JsonValue };
                    const topic = ObjectReference.key(reference);
                    let isRead = readable.get(topic);
                    if (isRead === undefined) {
                        isRead = await this.#lists(authorization, reference);
                        readable.set(topic, isRead);
                    }
                    if (isRead) {
                        broadcasts.push({ topic, event });
                    }
                }
                if (broadcasts.length > 0 && last !== undefined) {
                    yield {
                        reset: false,
                        complete: true,
                        changes: [],
                        position: last.position,
                        broadcasts,
                    };
                }
            }
        } finally {
            stop();
            await pages.return(undefined);
        }
    }
}

/** Where a sync starts and what it follows. */
export interface SyncOptions {
    /** The position the subscriber holds, absent before its first snapshot. */
    readonly after?: LogPosition;
    /** The queries to follow by name, every listed object type when absent. */
    readonly queries?: Readonly<Record<string, ObjectQuery>>;
    /** The queries the subscriber followed before. */
    readonly previous?: Readonly<Record<string, ObjectQuery>>;
    /** How often merged pages arrive. */
    readonly refresh?: { readonly every: Duration };
    /** The client following ephemeral objects, absent for durable ones. */
    readonly client?: string;
}

/** Who a relayed copy is decided for: a principal where its rows live, or the calling principal. */
export type ReplicaFollower =
    | {
          /** The principal the copy is decided for. */
          readonly subject: Subject;
          /** The scope containing the follower's scope, when this database holds no copy of it. */
          readonly parent?: string;
      }
    | { readonly context: ServiceContext };

/** Convert capacity failures to service failures. */
function serviceFailure(error: unknown): unknown {
    return error instanceof sync.SyncError
        ? new ServiceError(
              error.code === "OVERLOADED" ? "SERVICE_UNAVAILABLE" : "UNPROCESSABLE_CONTENT",
              { message: error.message },
          )
        : error;
}

/** List the tables a query reads. */
function tablesOf(query: sync.Query | sync.Include | sync.Relation): Table[] {
    return [
        query.table,
        ...Object.values(("include" in query ? query.include : undefined) ?? {}).flatMap(tablesOf),
        ...Object.values(query.relations ?? {}).flatMap(tablesOf),
    ];
}

/** Replace a page's journal rows with mutation outcomes, and add the scopes the subscription reads. */
function withOutcomes(
    page: sync.QueryPage,
    journal: string,
    chain: readonly string[],
): sync.QueryPage {
    // move journal changes to outcomes, and add the scopes the subscription reads
    const outcomes = page.changes.filter((change) => change.table === journal).flatMap(settled);
    const settledPage: sync.QueryPage = {
        ...page,
        scopes: [...chain],
        changes: page.changes.filter((change) => change.table !== journal),
        ...(outcomes.length === 0 ? {} : { outcomes: [...(page.outcomes ?? []), ...outcomes] }),
    };

    return settledPage.complete
        ? withEventMeta(settledPage, {
              id: `${settledPage.position.epoch}/${settledPage.position.sequence}`,
          })
        : settledPage;
}

/** Read the mutation outcome a journal change records: its first call's success, or its final failure. */
function settled(change: sync.RowChange): sync.MutationOutcome[] {
    // skip removed calls and calls no retry replays
    if (change.operation === "delete" || change.row.request === null) {
        return [];
    }

    // read the outcome once per mutation
    const call = AuditCall.parse(change.row.call);
    const { requestId, outcome } = call.execution;
    const id = schema.string().parse(requestId);
    if (outcome?.kind === "success") {
        return change.row.position === 0 ? [{ id }] : [];
    }

    return outcome === undefined ? [] : [{ id, error: outcome.error }];
}
