import {
    AccessContext,
    accessRelationship,
    accessRole,
    accessRolePermission,
    Authorizer,
    Caller,
    CHAIN_PUBLICATION,
    type ChainOptions,
    ChainParameters,
    CONTAINED,
    decisionTables,
    type Policy,
    principal,
    Relationship,
    UNIVERSE_PUBLICATION,
    UniverseParameters,
} from "@destack/access";
import { Scope, ObjectReference, ObjectTypeReference, Subject } from "@destack/sync";
import type { JsonValue } from "@destack/schema";
import {
    and,
    inArray,
    isNotNull,
    type Condition,
    type DatabaseConnection,
    eq,
    Key,
    LogPosition,
    Snapshot,
    TABLE,
    type Table,
    Transaction,
} from "@destack/db";

import { canonicalize, Digest, Duration, found, present, schema } from "@destack/schema";

import { CallError, refuseInput } from "@destack/runtime/error";

import { type RequestContext } from "@destack/runtime/server";
import { withEventMeta } from "@destack/runtime";
import * as sync from "@destack/sync";

import {
    ACCESS_PUBLICATION,
    EPHEMERAL_PUBLICATION,
    EXTERNAL_PUBLICATION,
    EphemeralParameters,
    QUERIES_PUBLICATION,
    QueriesParameters,
} from "../replica/replica.ts";
import { FOLLOW, ObjectType, REPLICATE, type ObjectStorage } from "../object/object.ts";
import { ExternalStorage } from "./external.ts";

import { Authorization } from "./authorization.ts";

import { EphemeralStorage } from "./ephemeral.ts";
import { ObjectAudience } from "./audience.ts";

import type { Controller } from "@destack/runtime/control";

import { until } from "@destack/runtime/timer";

import type { ObjectStore, Subscriber } from "./store.ts";

/** The largest broadcast event, in bytes: 16 KiB, far above the tens of bytes a cursor takes. */
const BROADCAST_BYTES = 16 * 1024;

/** An event an object sends, with the object it is about. */
const SENT_EVENT = ObjectReference.extend({ event: schema.json() });

/** The replicas a store streams: client feeds of scope queries, and databases below. */
export class ObjectSource {
    /** The store whose objects the replicas copy. */
    readonly store: ObjectStore;
    /** The replicas streaming now, by the scope or principal below they are for. */
    readonly #streaming = new Map<string, number>();

    /** Stream the replicas of a store's objects. */
    constructor(store: ObjectStore) {
        this.store = store;
    }

    /** Report whether a replica streams now for a scope or principal below. */
    follows(below: string): boolean {
        return this.#streaming.has(below);
    }

    /** Stream a publication's replica to the caller its audience admits. */
    async *relayed(
        subscription: sync.Subscription,
        context: RequestContext,
    ): AsyncGenerator<sync.Page> {
        // admit the caller by the publication's audience, counting its replica while it streams
        const publication = this.#publication(subscription);
        const follower = await this.#admit(publication, subscription, context);
        const below = subscription.below;
        this.#streaming.set(below, (this.#streaming.get(below) ?? 0) + 1);
        try {
            yield* this.#relay(publication, subscription, follower, context);
        } finally {
            const remaining = found(this.#streaming, below) - 1;
            if (remaining === 0) {
                this.#streaming.delete(below);
            } else {
                this.#streaming.set(below, remaining);
            }
        }
    }

    /** Stream an admitted follower's replica by its publication. */
    async *#relay(
        publication: sync.Publication,
        subscription: sync.Subscription,
        follower: ReplicaFollower,
        context: RequestContext,
    ): AsyncGenerator<sync.Page> {
        // follow a scope's durable objects
        if (publication === this.queriesPublication) {
            yield* this.#durable(subscription, follower, context);
        }
        // follow a scope's ephemeral objects
        else if (publication === this.ephemeralPublication) {
            yield* this.#ephemeral(subscription, follower, context);
        }
        // follow a scope's external objects
        else if (publication === this.externalPublication) {
            yield* this.#external(subscription, follower, context);
        }
        // follow the access rows of a scope's chain
        else if (publication === this.accessPublication) {
            yield* this.#access(subscription, follower, context);
        }
        // relay any other replica to the follower it is for
        else {
            const { after, ...request } = subscription;
            yield* this.replicate(request, after, follower, context.request.signal, context.signal);
        }
    }

    /** Admit a caller to a publication's replica for the scope below, returning who it is for. */
    async #admit(
        publication: sync.Publication,
        subscription: sync.Subscription,
        context: RequestContext,
    ): Promise<ReplicaFollower> {
        switch (publication.audience) {
            case "caller": {
                // decide for the caller, or for the principal it may follow on behalf of
                await this.store.enter(context, subscription.scope);
                if (subscription.subject === undefined) {
                    return { context };
                }
                const subject = subscription.subject;
                await this.#requireFollow(subject, context);

                return { subject };
            }
            case "contained":
            case "reader": {
                const { after: _after, ...request } = subscription;

                return this.#follower(request, context);
            }
        }
    }

    /** Read a follower's chain parameters, refusing malformed ones as a bad request. */
    static #chainParameters(request: Pick<sync.Subscription, "parameters">): ChainParameters {
        const parsed = ChainParameters.safeParse(request.parameters);
        if (!parsed.success) {
            throw refuseInput(request.parameters, parsed.error.issues);
        }

        return parsed.data;
    }

    /** Decide who a relayed replica is for: a served object's principal, or the caller. */
    async #follower(
        request: Omit<sync.Subscription, "after">,
        context: RequestContext,
    ): Promise<ReplicaFollower> {
        // find the served object standing for the scope
        const standing = await this.#standing(request.below);

        // decide for the subject a follower follows on behalf of, as its token states it may
        if (request.subject !== undefined) {
            const subject = request.subject;
            await this.#requireFollow(subject, context);

            return { subject };
        }

        // decide for the principal a served object stands for when it follows itself
        const caller = ObjectSource.#principal(context);
        if (standing !== undefined && Subject.same(caller, standing.subject)) {
            const { subject, parent } = standing;

            return { subject, ...(parent === undefined ? {} : { parent }) };
        }

        // decide for the scope's principal or a caller living in it, bound to the called space or the scope below
        const following = context.access(context.scope ?? request.below);
        const resident = [following.subject, ...following.groups].filter(
            (subject) => subject.id === request.below || subject.scope === request.below,
        );
        if (resident.length === 0) {
            throw new CallError("FORBIDDEN", {
                message: `only ${request.below} and its contents can follow the replicas held for it`,
            });
        }
        // decide a replica on its scope, and a via replica on each of its scopes
        const via =
            request.publication === CHAIN_PUBLICATION
                ? ObjectSource.#chainParameters(request).via
                : [];
        if (this.#publication(request).audience === "contained" && via.length === 0) {
            const isResident = resident.some((subject) => subject.scope === request.below);
            const decided = standing === undefined || isResident ? request.below : request.scope;
            await this.#requireReplicate(decided, resident, context);
        }

        return { context };
    }

    /** Read the principal following, refusing an anonymous follower as unauthenticated. */
    static #principal(context: RequestContext): Subject {
        const caller = context.caller();
        if (Caller.isAnonymous(caller)) {
            throw new CallError("UNAUTHORIZED", { message: "missing caller credential" });
        }

        return Caller.principal(caller);
    }

    /** Require the caller to be a principal, or to have `follow` on the principal's object. */
    async #requireFollow(subject: Subject, context: RequestContext): Promise<void> {
        // admit the principal itself
        const acting = ObjectSource.#principal(context);
        if (Subject.same(acting, subject)) {
            return;
        }

        // require a principal type its followers declare
        const policy = this.store.authorizer.policy(subject);
        if (!Object.hasOwn(policy.definition.permissions, FOLLOW)) {
            throw new CallError("FORBIDDEN", {
                message: `a ${subject.type} has no followers`,
            });
        }

        // decide `follow` on the principal from the caller's identity and stated sets
        const { database } = this.store;
        const caller = context.access();
        const { scope: _bound, ...credential } = present(
            caller.credential,
            "a follower's credential",
        );
        const authorization = await this.store.authorize(
            Transaction.reading(database),
            subject.scope,
            { ...caller, credential },
        );
        await authorization.require(policy.permission(FOLLOW), subject);
    }

    /** Require `replicate` on a caller's own object, or else on a scope. */
    async #requireReplicate(
        below: string,
        resident: readonly Subject[],
        context: RequestContext,
    ): Promise<void> {
        // list the permissions on the caller's own objects, at the rows standing for them here
        const authorization = await this.store.authorize(
            Transaction.reading(this.store.database),
            below,
            context.access(below),
        );
        const standings = await Promise.all(resident.map((subject) => this.#standing(subject.id)));
        const personal = standings.flatMap((standing) =>
            standing === undefined || !standing.object.permissions.includes(REPLICATE)
                ? []
                : [
                      {
                          permission: standing.object.permission(REPLICATE),
                          target: standing.reference,
                      },
                  ],
        );

        // add the scope's permission, and require the first one
        const scope = await Scope.object(Snapshot.live(this.store.database), below);
        const policy = this.store.authorizer.policy(scope);
        const granting = [...personal];
        if (Object.hasOwn(policy.definition.permissions, REPLICATE)) {
            granting.push({ permission: policy.permission(REPLICATE), target: scope });
        }
        const [first] = granting;
        if (first === undefined) {
            throw new CallError("FORBIDDEN", {
                message: `a ${scope.type} has no replicas of its chain`,
            });
        }
        await authorization.require(first.permission, first.target);
    }

    /** List the scopes above a principal's row, or an installation's space, else none. */
    async within(
        subject: Subject | undefined,
        database: DatabaseConnection,
    ): Promise<readonly Subject[]> {
        // take the principal standing for itself, or the space an installation lives in
        if (subject === undefined) {
            return [];
        }
        const standingFor = principal.installation.is(subject) ? subject.scope : subject.id;

        // read the scopes above the row standing for it
        const standing = await this.#standing(standingFor, database);
        if (standing?.parent === undefined) {
            return [];
        }

        return (await this.#within(standing.parent, database)).map((inside) => ({
            ...inside,
            relation: CONTAINED,
        }));
    }

    /** Find a principal's row in the decided type representing its kind, with its scope. */
    async #standing(
        below: string,
        database: DatabaseConnection = this.store.database,
    ): Promise<
        | {
              readonly object: ObjectType;
              readonly reference: ObjectReference;
              readonly subject: Subject;
              readonly parent: string | undefined;
          }
        | undefined
    > {
        // find the one decided type representing the principal kind of the identifier
        const representing = this.store.decided.filter(
            (type) =>
                type.represents !== undefined &&
                type.table[TABLE].column("id").definition.schema.safeParse(below).success,
        );
        if (representing.length > 1) {
            throw new TypeError(
                `types ${representing.map((type) => type.name).join(", ")} all represent ${below}`,
            );
        }
        const [object] = representing;
        const kind = object?.represents;
        if (object === undefined || kind === undefined) {
            return undefined;
        }

        // read the principal's row, and stand for the principal inside the scope holding it
        const [row] = await Snapshot.live(database).select(object.table, ["id"], [[below]]);
        if (row === undefined) {
            return undefined;
        }
        const scope = schema.string().parse(row["scope"]);
        const subject = {
            packageId: kind.definition.packageId,
            type: kind.name,
            scope: kind.definition.isGlobal === true ? Scope.universe.id : scope,
            id: below,
        };

        return {
            object,
            reference: object.reference(scope, below),
            subject,
            parent: scope === Scope.universe.id ? undefined : scope,
        };
    }

    /** The publication of a scope's durable objects: named queries, or every type of its level. */
    readonly queriesPublication = sync.definePublication({
        name: QUERIES_PUBLICATION,
        parameters: QueriesParameters,
        audience: "caller",
        isReadinessGate: true,
        replica: ({ name, scope, parameters }) => {
            // follow every durable table for a follower without queries
            const named = parameters.queries;
            if (named === undefined) {
                return new sync.Replica({
                    name,
                    scope,
                    tables: this.store.durable.map((object) => object.table),
                });
            }

            // compile the queries against the followed shapes' types first
            const types = [...this.store.followed, ...this.store.decided];
            const compiled = Object.values(ObjectType.queries(types, named, [scope]));

            // follow each queried table's rows under its queries' conditions
            const tables = [...new Set(compiled.map((query) => query.table))];
            const where = new Map<Table, Condition>();
            for (const table of tables) {
                const conditions = compiled
                    .filter((query) => query.table === table)
                    .map((query) => query.where);
                if (conditions.every((condition) => condition !== undefined)) {
                    where.set(
                        table,
                        conditions.length === 1
                            ? present(conditions[0], "a condition")
                            : { OR: conditions },
                    );
                }
            }
            const everywhere = compiled.filter((query) => query.scopes === "every");

            return new sync.Replica({
                name,
                scope,
                tables,
                where,
                everywhere: new Set(everywhere.map((query) => query.table)),
            });
        },
    });

    /** The publication of a scope's ephemeral objects a client follows, writing its own rows. */
    readonly ephemeralPublication = sync.definePublication({
        name: EPHEMERAL_PUBLICATION,
        parameters: EphemeralParameters,
        audience: "caller",
        isReadinessGate: true,
        replica: ({ name, scope }) =>
            new sync.Replica({
                name,
                scope,
                tables: this.store.requireEphemeral().objects.map((object) => object.table),
            }),
    });

    /** The publication of a scope's external objects from their files' database: named queries. */
    readonly externalPublication = sync.definePublication({
        name: EXTERNAL_PUBLICATION,
        parameters: QueriesParameters,
        audience: "caller",
        isReadinessGate: true,
        replica: ({ name, scope }) =>
            new sync.Replica({
                name,
                scope,
                tables: this.store.external.flatMap((storage) =>
                    storage.objects.map((object) => object.table),
                ),
            }),
    });

    /** The publication of a scope's chain replica: access rows, inherited rows and own object. */
    readonly chainPublication = sync.definePublication({
        name: CHAIN_PUBLICATION,
        parameters: ChainParameters,
        audience: "contained",
        isReadinessGate: true,
        replica: (request) => this.store.authorizer.replicateChain(request),
    });

    /** The publication of the universe's rows a scope reads, decided for their reader. */
    readonly universePublication = sync.definePublication({
        name: UNIVERSE_PUBLICATION,
        parameters: UniverseParameters,
        audience: "reader",
        isReadinessGate: true,
        replica: (request) => this.store.authorizer.replicateUniverse(request),
    });

    /** The publication of the access rows of the caller's own checks in a scope's chain. */
    readonly accessPublication = sync.definePublication({
        name: ACCESS_PUBLICATION,
        parameters: schema.object({}),
        audience: "caller",
        isReadinessGate: true,
        replica: ({ name, scope }) => new sync.Replica({ name, scope, tables: decisionTables }),
    });

    /** Follow a scope's durable objects for a follower. */
    async *#durable(
        subscription: sync.Subscription,
        follower: ReplicaFollower,
        context: RequestContext,
    ): AsyncGenerator<sync.Page> {
        // open the audience
        const { scope, after, previous, refresh, origin } = subscription;
        const { queries } = QueriesParameters.parse(subscription.parameters);
        const reader = this.#reader(follower, scope);
        const audience = await ObjectAudience.of(this.store, scope, reader);

        // follow the replica's queries, with the replicas the reader sees below the scope
        const { chain } = audience;
        const caller = Caller.principal(reader);
        const compile = (named: QueriesParameters["queries"]) =>
            ObjectType.queries(this.store.durable, named, chain);
        const earlier =
            previous === undefined ? undefined : QueriesParameters.parse(previous).queries;
        const options = ObjectSource.#followOptions(audience, context, {
            previous: earlier && compile(earlier),
            refresh,
            origin,
        });
        const follow = (feed: sync.Feed) =>
            this.#replicating(
                feed,
                scope,
                caller,
                chain,
                compile(queries),
                after,
                options,
                context.request.signal,
            );

        // audit watching each audited object type
        const tables = new Set(
            Object.values(compile(queries)).flatMap((query) => ObjectSource.#tablesOf(query)),
        );
        const audited = this.store.objects.filter(
            (object) =>
                (object.isReadAudited || this.store.isAccessAudited) && tables.has(object.table),
        );
        const pages = audited.reduce(
            (source, object) => () => {
                const { action, values } = object.auditCall("watch", {}, scope);

                return this.store
                    .audit(scope, context.access(scope), { requestId: context.requestId })
                    .stream(action, values, source);
            },
            () => follow(this.store.feed),
        );
        yield* pages();
    }

    /** Follow queries over their scopes and the replicas a caller sees below them. */
    async *#replicating(
        feed: sync.Feed,
        scope: string,
        caller: Subject | undefined,
        chain: readonly string[],
        queries: Record<string, sync.Query>,
        after: LogPosition | undefined,
        options: sync.FeedOptions,
        signal: AbortSignal,
    ): AsyncGenerator<sync.Page> {
        // read the replicas held now
        const { database } = this.store;
        let held = await sync.Replica.tables(database, scope, caller);
        let reflected = options.previous && ObjectSource.#withReplicas(options.previous, held);
        let position = after;
        while (!signal.aborted) {
            // watch the replicas until they change, its failure surfacing once the follow ends
            const current = canonicalize(held);
            const compiled = ObjectSource.#withReplicas(queries, held);
            const rekept = new AbortController();
            const following = AbortSignal.any([signal, rekept.signal]);
            const name = sync.replica[TABLE].sqlName;
            const changed = database.log.until(
                async (tables) =>
                    tables.has(name) &&
                    canonicalize(await sync.Replica.tables(database, scope, caller)) !== current,
                following,
            );
            changed.then(
                (isChanged) => isChanged && rekept.abort(),
                () => rekept.abort(),
            );

            // follow the queries from the last complete position
            const read = [...new Set([...chain, ...held.map((replica) => replica.scope)])];
            const { previous: _previous, ...rest } = options;
            const subscribed = feed.subscribe(compiled, position, following, {
                ...rest,
                ...(reflected === undefined ? {} : { previous: reflected }),
            });
            for await (const page of ObjectSource.#marking(subscribed, read)) {
                yield page;
                position = page.complete ? page.position : position;
            }
            rekept.abort();
            await changed;

            // follow again over the replicas held now
            reflected = compiled;
            held = await sync.Replica.tables(database, scope, caller);
        }
    }

    /** Follow a scope's ephemeral objects for a writer. */
    async *#ephemeral(
        subscription: sync.Subscription,
        follower: ReplicaFollower,
        context: RequestContext,
    ): AsyncGenerator<sync.Page> {
        // track the writer's rows
        const { scope, after, previous, refresh, origin } = subscription;
        const { queries, writerId } = EphemeralParameters.parse(subscription.parameters);
        const store = this.store.requireEphemeral();
        const release = store.track(EphemeralStorage.writerKey(context.access(scope), writerId));
        const compiled = ObjectType.queries(store.objects, queries, [scope]);
        const earlier =
            previous === undefined ? undefined : EphemeralParameters.parse(previous).queries;
        const reflected = earlier && ObjectType.queries(store.objects, earlier, [scope]);
        try {
            // follow until access changes, or to a completed page once the caller lapses
            yield* this.#decided(
                scope,
                follower,
                context,
                "ephemeral",
                (audience, signal, isFirst) => {
                    const options = ObjectSource.#followOptions(audience, context, {
                        previous: isFirst ? reflected : undefined,
                        refresh,
                        origin,
                    });
                    const pages = store.feed.subscribe(
                        compiled,
                        isFirst ? after : undefined,
                        signal,
                        options,
                    );

                    return this.#interleave(pages, scope, audience.authorization, signal);
                },
            );
        } finally {
            release();
        }
    }

    /** Follow a scope's external objects for the caller, decided in memory until access changes. */
    async *#external(
        subscription: sync.Subscription,
        follower: ReplicaFollower,
        context: RequestContext,
    ): AsyncGenerator<sync.Page> {
        // open the scope's files
        const { scope, after, previous, refresh, origin } = subscription;
        const { queries } = QueriesParameters.parse(subscription.parameters);
        const external = ExternalStorage.of(
            this.store.external,
            Object.values(queries ?? {}).map((query) => query.object),
        );
        const feed = new sync.Feed(
            await external.open(scope),
            external.objects.map((object) => object.table),
            { clock: this.store.clock },
        );
        const compiled = ObjectType.queries(external.objects, queries, [scope]);
        const earlier =
            previous === undefined ? undefined : QueriesParameters.parse(previous).queries;
        const reflected = earlier && ObjectType.queries(external.objects, earlier, [scope]);

        // follow until access changes, or to a completed page once the caller lapses
        yield* this.#decided(scope, follower, context, "external", (audience, signal, isFirst) => {
            const options = ObjectSource.#followOptions(audience, context, {
                previous: isFirst ? reflected : undefined,
                refresh,
                origin,
            });

            return feed.subscribe(compiled, isFirst ? after : undefined, signal, options);
        });
    }

    /** Follow the access rows the caller's own checks read in a scope's chain. */
    async *#access(
        subscription: sync.Subscription,
        follower: ReplicaFollower,
        context: RequestContext,
    ): AsyncGenerator<sync.Page> {
        // follow until access changes
        const { scope, after, refresh, origin } = subscription;
        yield* this.#decided(scope, follower, context, "durable", (audience, signal, isFirst) => {
            // replicate the chain's scopes and roles, and the caller's subjects' relationships
            const { chain } = audience;
            const subjects = audience.authorization.access.authority.subjects;
            const queries: Record<string, sync.Query> = {
                scopes: { table: Scope.table, scopes: chain },
                roles: { table: accessRole, scopes: chain },
                permissions: { table: accessRolePermission, scopes: chain },
                relationships: {
                    table: accessRelationship,
                    scopes: chain,
                    where: Relationship.heldBy(subjects),
                },
            };
            const options = ObjectSource.#followOptions(sync.EVERYONE, context, {
                previous: undefined,
                refresh,
                origin,
            });
            const pages = this.store.feed.subscribe(
                queries,
                isFirst ? after : undefined,
                signal,
                options,
            );

            return ObjectSource.#marking(pages, chain);
        });
    }

    /** Follow pages decided for a follower, restarting from a snapshot on chain access changes. */
    async *#decided(
        scope: string,
        follower: ReplicaFollower,
        context: RequestContext,
        storage: ObjectStorage,
        follow: (
            audience: ObjectAudience,
            signal: AbortSignal,
            isFirst: boolean,
        ) => AsyncGenerator<sync.Page>,
    ): AsyncGenerator<sync.Page> {
        let isFirst = true;
        while (!context.signal.aborted) {
            // open the audience and watch access rows until the request closes or access changes
            const revised = new AbortController();
            const signal = AbortSignal.any([context.request.signal, revised.signal]);
            const since = (await this.store.database.log.position()).sequence;
            const audience = await ObjectAudience.of(
                this.store,
                scope,
                this.#reader(follower, scope),
                {
                    storage,
                },
            );
            const watching = this.#watchAccess(audience.chain, since, revised, signal);

            // follow until access changes, and from a snapshot after
            yield* follow(audience, signal, isFirst);
            revised.abort();
            await watching;
            isFirst = false;
        }
    }

    /** Read the caller deciding a follower's rows in a scope: its principal, or the caller. */
    #reader(follower: ReplicaFollower, scope: string): AccessContext {
        return "subject" in follower
            ? AccessContext.of(Caller.of(follower.subject), this.store.clock.now())
            : follower.context.access(scope);
    }

    /** Revise a decided stream once the access rows of its chain change, failing with the log. */
    async #watchAccess(
        chain: readonly string[],
        since: number,
        revised: AbortController,
        signal: AbortSignal,
    ): Promise<void> {
        const tables = this.store.authorizer.watch(chain).map((watch) => watch.table);
        try {
            // abort the stream on the first page with changes
            const pages = this.store.database.log.follow(
                { tables, scopes: chain, after: since },
                signal,
            );
            for await (const page of pages) {
                if (page.changes.length > 0) {
                    revised.abort();
                }
            }
        } catch (error) {
            // abort the stream and fail with the log
            revised.abort();
            throw error instanceof Error
                ? error
                : new Error("following the log failed", { cause: error });
        }
    }

    /** Send an unstored event to an object's readers. */
    async broadcast(
        scope: string,
        target: Omit<ObjectReference, "scope">,
        event: JsonValue,
        context: RequestContext,
    ): Promise<void> {
        // require a small event and a reader
        const store = this.store.requireEphemeral();
        if (new TextEncoder().encode(JSON.stringify(event)).length > BROADCAST_BYTES) {
            throw new CallError("BAD_REQUEST", {
                message: `a broadcast has at most ${BROADCAST_BYTES} bytes`,
            });
        }
        await this.store.enter(context, scope);
        const reference = { ...target, scope };
        const authorization = await this.store.admit(
            Transaction.reading(this.store.database),
            scope,
            context.access(scope),
        );
        if (!(await this.#lists(authorization, reference))) {
            throw new CallError("NOT_FOUND", { message: `no ${target.type} ${target.id}` });
        }

        // send it to the scope's streams
        store.tracker.broadcast(scope, { ...reference, event });
    }

    /** The publications this store serves and follows, by name. */
    get publications(): ReadonlyMap<string, sync.Publication> {
        return new Map(
            [
                this.chainPublication,
                this.universePublication,
                this.queriesPublication,
                this.ephemeralPublication,
                this.externalPublication,
                this.accessPublication,
                ...this.store.publications,
            ].map((publication) => [publication.name, publication]),
        );
    }

    /** Build the replica a request of a served publication asks for. */
    buildReplica(request: sync.Subscription): sync.Replica {
        return this.#publication(request).replica(request);
    }

    /** Report whether a request's replica gates this store's readiness, as its publication says. */
    isReadinessGate(request: Pick<sync.Subscription, "publication">): boolean {
        return this.#publication(request).isReadinessGate;
    }

    /** Find the served publication a request asks for, refusing any other. */
    #publication(request: Pick<sync.Subscription, "publication">): sync.Publication {
        const publication = this.publications.get(request.publication);
        if (publication === undefined) {
            throw new CallError("NOT_FOUND", { message: `no publication ${request.publication}` });
        }

        return publication;
    }

    /** Build a subscription's replica key from its name and scope, shared per replica. */
    static #key(subscription: Pick<sync.Subscription, "name" | "scope">): string {
        return `${subscription.name} ${subscription.scope}`;
    }

    /** Build the controller following requested replicas and dropping unrequested held replicas. */
    controller(subscriber: Subscriber): Controller {
        // list the requested replicas and revise the listed ones
        const listed = new Map<string, ReplicaEntry>();
        const list = async (): Promise<readonly string[]> => {
            const replicas = await this.#replicas(subscriber);
            ObjectSource.#revise(listed, replicas);

            return [...replicas.keys()];
        };

        // reconcile each replica under its own key and lease, so no page lands after its drop
        return {
            name: "replica",
            mode: "follow",
            watches: this.#replicaWatches(subscriber),
            concurrency: Infinity,
            list,
            reconcile: (key, { signal }) => this.#reconcileReplica(key, listed, list, signal),
        };
    }

    /** List the tables whose changes list the replicas again. */
    #replicaWatches(subscriber: Subscriber): Table[] {
        // watch the replicated scope types and the rows each replica includes
        const scopes = this.store.replicated
            .filter((object) => object.policy.definition.scope === true)
            .map((object) => object.table);

        return [
            ...new Set([Scope.table, sync.replicaRow, ...scopes, ...(subscriber.watches ?? [])]),
        ];
    }

    /** Follow requested replicas from publishers and drop unrequested ones, by replica key. */
    async #replicas(
        subscriber: Subscriber,
    ): Promise<Map<string, ReplicaIntent & { digest: string }>> {
        // drop the held replicas no request asks for and follow the requested ones
        const entries = await subscriber.subscriptions();
        const named = new Set(entries.map((entry) => ObjectSource.#key(entry.subscription)));
        const held = await sync.Replica.subscriptions(this.store.database);
        const actions: ReplicaIntent[] = [
            ...held
                .filter((replica) => !named.has(ObjectSource.#key(replica)))
                .map((replica) => ({ action: "drop" as const, subscription: replica })),
            ...entries.map(({ subscription, publisher }) => ({
                action: "follow" as const,
                subscription,
                publisher,
            })),
        ];

        // digest each action with its subscription
        return new Map(
            await Promise.all(
                actions.map(
                    async (entry): Promise<[string, ReplicaIntent & { digest: string }]> => [
                        ObjectSource.#key(entry.subscription),
                        {
                            ...entry,
                            digest: await Digest.json({
                                action: entry.action,
                                subscription: entry.subscription,
                            }),
                        },
                    ],
                ),
            ),
        );
    }

    /** Follow or drop one listed replica until the controller stops it. */
    async #reconcileReplica(
        key: string,
        listed: ReadonlyMap<string, ReplicaEntry>,
        list: () => Promise<readonly string[]>,
        signal: AbortSignal,
    ): Promise<undefined> {
        while (!signal.aborted) {
            // wait for the loop to stop a replica no longer listed
            const replica = listed.get(key);
            if (replica === undefined) {
                await until(signal);

                return undefined;
            }

            // follow a requested replica from the position it reached until its request changes
            const revised = AbortSignal.any([signal, replica.revised.signal]);
            if (replica.action === "follow") {
                await this.#followReplica(key, replica, listed, list, revised);
            }
            // drop a held replica no request asks for and wait for a request asking for it again
            else {
                await this.buildReplica(replica.subscription).drop(this.store.database);
                await until(revised);
            }
        }

        return undefined;
    }

    /** Follow a requested replica, failing on its source's refusal unless the request changed. */
    async #followReplica(
        key: string,
        replica: ReplicaEntry & { readonly action: "follow" },
        listed: ReadonlyMap<string, ReplicaEntry>,
        list: () => Promise<readonly string[]>,
        revised: AbortSignal,
    ): Promise<void> {
        try {
            const { subscription, publisher } = replica;
            const { audience } = this.#publication(subscription);
            await this.buildReplica(subscription).follow(
                this.store.database,
                (from, stream) => publisher.stream({ ...subscription, ...from }, stream),
                revised,
                { subscription, audience, clock: this.store.clock },
            );
        } catch (error) {
            // list again once a source refuses a follow, failing it unless its request changed
            await list();
            if (listed.get(key) === replica) {
                throw error;
            }
        }
    }

    /** List the replicas a store's decisions read, each with the scope whose home publishes it. */
    async chains(below: string): Promise<
        {
            readonly subscription: sync.Subscription;
            readonly home: string;
            readonly brought?: readonly string[];
        }[]
    > {
        // follow nothing at the universe, nor for a scope without a chain
        const kind = below === Scope.universe.id ? undefined : await this.#chainKind();
        if (kind === undefined) {
            return [];
        }

        // replicate the scope's own chain and the universe rows it reads
        const served = this.store.objects.filter((object) => object.storage === "durable");
        const known = [...this.store.objects, ...this.store.replicated];
        const isLiving = served.some((object) =>
            object.scopes.some((scope) => !known.some((type) => type.same(scope))),
        );
        const universe = this.universeSubscription(below);
        const isOwn = isLiving || kind.kind === "home";
        const own = isOwn
            ? await this.subscriptions(below, kind)
            : universe === undefined
              ? []
              : [universe];

        // leave the own chain's scopes to it, else let the via replica cover the store's own scope
        const chained = new Set(
            isOwn ? [below, ...own.map((subscription) => subscription.scope)] : [],
        );

        // read the replicated scopes outside the chain that served objects live in or relate to
        const living = this.store.replicated.filter((scope) =>
            served.some((object) =>
                [...object.scopes, ...object.related].some((type) => type.same(scope)),
            ),
        );
        const outside = await Promise.all(
            living.map(async (scope) => {
                // read the scope rows the database has
                const table = scope.table[TABLE];
                const rows = await this.store.database
                    .select({ id: table.column("id"), scope: table.column("scope") })
                    .from(scope.table);

                // read who brought each row here, in one read per table
                const stored = rows
                    .map((row) => ({
                        id: schema.string().parse(row.id),
                        scope: schema.string().parse(row.scope),
                    }))
                    .filter((row) => !chained.has(row.id));
                const bringers = await this.#bringers(scope.table, stored);

                return stored.map((row) => ({
                    row,
                    ...bringers.get(Key.name(scope.table, row)),
                }));
            }),
        );

        // replicate the outside scopes' chains from the universe, once per store and per following subject
        const bySubject = Map.groupBy(outside.flat(), (entry) =>
            entry.subject === undefined ? undefined : Subject.key(entry.subject),
        );
        const storeVia = await this.store.authorizer.chainVia(
            this.store.database,
            { name: Authorizer.viaReplica(below), below },
            (bySubject.get(undefined) ?? []).map((entry) => entry.row),
            isOwn ? [] : this.reached(),
        );
        const onBehalf = await Promise.all(
            [...bySubject].flatMap(([key, entries]) => {
                const subject = entries[0]?.subject;

                return key === undefined || subject === undefined
                    ? []
                    : [
                          this.store.authorizer
                              .chainVia(
                                  this.store.database,
                                  { name: `${Authorizer.viaReplica(below)}:${key}`, below },
                                  entries.map((entry) => entry.row),
                                  [],
                              )
                              .then((via) =>
                                  via === undefined
                                      ? undefined
                                      : {
                                            subscription: { ...via, subject },
                                            brought: [
                                                ...new Set(
                                                    entries.flatMap((entry) => entry.name ?? []),
                                                ),
                                            ],
                                        },
                              ),
                      ];
            }),
        );
        const vias = [
            ...(storeVia === undefined ? [] : [{ subscription: storeVia }]),
            ...onBehalf.filter((via) => via !== undefined),
        ];

        return [
            ...own.map((subscription) => ({ subscription, home: below })),
            ...vias.map((via) => ({ ...via, home: Scope.universe.id })),
        ];
    }

    /** Read the subject and replica of the follow that brought each of some scope rows here. */
    async #bringers(
        table: Table,
        rows: readonly { readonly id: string; readonly scope: string }[],
    ): Promise<Map<string, { readonly subject: Subject; readonly name: string }>> {
        // read the replicas with the rows that follow on a subject's behalf
        const bringers = new Map<string, { readonly subject: Subject; readonly name: string }>();
        if (rows.length === 0) {
            return bringers;
        }
        const brought = await this.store.database
            .select({ key: sync.replicaRow.key, replica: sync.replica })
            .from(sync.replica)
            .innerJoin(
                sync.replicaRow,
                and(
                    eq(sync.replicaRow.name, sync.replica.name),
                    eq(sync.replicaRow.scope, sync.replica.scope),
                ),
            )
            .where(
                and(
                    eq(sync.replicaRow.table, table[TABLE].sqlName),
                    inArray(
                        sync.replicaRow.key,
                        rows.map((row) => Key.name(table, row)),
                    ),
                    isNotNull(sync.replica.subjectId),
                ),
            );

        // take the first such replica of each row
        for (const { key, replica } of brought) {
            const subscription = sync.Replica.subscriptionOf(replica);
            if (!bringers.has(key) && subscription?.subject !== undefined) {
                bringers.set(key, { subject: subscription.subject, name: subscription.name });
            }
        }

        return bringers;
    }

    /** List the object types whose universe relationships the decided types reach. */
    reached(): ObjectTypeReference[] {
        // walk the policies the decided types' relations reference, at any depth
        const reached = new Map<string, Policy>();
        const pending = this.store.decided.map((type) => type.policy);
        for (let policy = pending.pop(); policy !== undefined; policy = pending.pop()) {
            const key = ObjectTypeReference.key(policy.typeReference);
            if (!reached.has(key)) {
                reached.set(key, policy);
                pending.push(...policy.references);
            }
        }

        // read the universe's own relationships beside them
        const universe = { packageId: Scope.universe.packageId, type: Scope.universe.type };

        return [universe, ...[...reached.values()].map((policy) => policy.typeReference)].toSorted(
            (left, right) =>
                ObjectTypeReference.key(left).localeCompare(ObjectTypeReference.key(right)),
        );
    }

    /** Read how the store holds its scope's chain, as its host says. */
    #chainKind(): Promise<ChainOptions | undefined> {
        return this.store.chain(Snapshot.live(this.store.database));
    }

    /** List a database's replica subscriptions for a scope: its chain and its universe rows. */
    async subscriptions(
        below: string,
        options: ChainOptions,
        elsewhere: readonly ObjectType[] = [],
    ): Promise<sync.Subscription[]> {
        // replicate the chain and the universe rows the scope reads, except another replica's
        const chain = await this.store.authorizer.chain(
            this.store.database,
            below,
            options,
            this.reached(),
        );
        const universe = this.universeSubscription(below, elsewhere);

        return universe === undefined ? chain : [...chain, universe];
    }

    /** Request the universe rows a scope or machine may read, but those another replica has. */
    universeSubscription(
        below: string,
        elsewhere: readonly ObjectType[] = [],
    ): sync.Subscription | undefined {
        // leave inherited rows to the chains, and rows another replica or a follow has to it
        const followedElsewhere = [...elsewhere, ...this.store.followed];
        const replicated = this.store.replicated.filter(
            (object) =>
                object.inherited === undefined &&
                !followedElsewhere.some((type) => type.same(object)),
        );
        const isUnscoped = (object: ObjectType) =>
            object.scopes.length === 0 ||
            object.scopes.some((scope) => !replicated.some((type) => type.same(scope)));

        // replicate a type that lives in scope types outside the replica from every scope of its rows
        const listed = replicated.filter(isUnscoped);
        const rows: UniverseParameters["rows"][number][] = listed.map((object) =>
            object.scope === Scope.universe.id
                ? { type: object.typeReference, where: {} }
                : { type: object.typeReference, where: {}, isEverywhere: true },
        );

        // add the types living in the scopes of listed types, at any depth
        for (let isGrowing = true; isGrowing;) {
            isGrowing = false;
            for (const object of replicated.filter((candidate) => !listed.includes(candidate))) {
                const scopes = listed.filter((scope) =>
                    object.scopes.some((type) => type.same(scope)),
                );
                if (scopes.length > 0) {
                    rows.push({
                        type: object.typeReference,
                        where: {},
                        within: scopes.map((scope) => scope.typeReference),
                    });
                    listed.push(object);
                    isGrowing = true;
                }
            }
        }

        // request nothing where no universe object type is replicated
        if (rows.length === 0) {
            return undefined;
        }

        // replicate the universe's rows the scope or machine may read, under its own name
        return Authorizer.universeSubscription({
            name: below,
            scope: Scope.universe.id,
            below,
            parameters: { rows },
        });
    }

    /** Stream a replica's pages to a database in this process, never back to their source. */
    async *publish(
        subscription: sync.Subscription,
        signal: AbortSignal,
    ): AsyncGenerator<sync.Page> {
        // refuse a subscriber whose rows the replica would send back
        const replica = this.buildReplica(subscription);
        const { after, origin } = subscription;
        if (origin !== undefined) {
            await replica.requireAcyclic(this.store.database, origin);
        }

        // stream the replica, sending no bare positions of the subscriber's own writes
        yield* this.store.feed.subscribe(
            replica.queries,
            after,
            signal,
            origin === undefined ? {} : { origin },
        );
    }

    /** Stream a replica's pages to a database below. */
    async *replicate(
        request: sync.Subscription,
        after: LogPosition | undefined,
        follower: ReplicaFollower,
        signal: AbortSignal,
        drain?: AbortSignal,
    ): AsyncGenerator<sync.Page> {
        // require the replicated scope to contain the follower's for a chain
        const publication = this.#publication(request);
        if (publication.audience === "contained") {
            await this.#requireContaining(request, follower);
        }

        // admit the follower to a chain by containment and to the universe's rows as their reader
        const audience = await this.#replicaAudience(publication, request, follower);

        // relay a replicated scope once its replica has a position, to followers other than its source
        const replica = publication.replica(request);
        const { previous, origin } = request;
        await sync.Replica.requireRelayable(this.store.database, request.name, request.scope);
        if (origin !== undefined) {
            await replica.requireAcyclic(this.store.database, origin);
        }

        // stream the replica from the parameters it reflects, ending at a completed page once drained
        const reflected =
            previous === undefined
                ? undefined
                : publication.replica({ ...request, parameters: previous }).queries;
        yield* this.store.feed.subscribe(replica.queries, after, signal, {
            audience,
            ...(reflected === undefined ? {} : { previous: reflected }),
            ...(drain === undefined ? {} : { drain }),
            ...(origin === undefined ? {} : { origin }),
        });
    }

    /** Require a chain replica's scope to contain the follower's, via its parent if elsewhere. */
    async #requireContaining(request: sync.Subscription, follower: ReplicaFollower): Promise<void> {
        // require a follower outside the chains to replicate them via scopes inside them
        const { via, between } = ObjectSource.#chainParameters(request);
        if (via.length > 0) {
            await this.#requireVia(request.scope, via, between);

            return;
        }

        // read the scopes containing the follower's
        const parent = "subject" in follower ? follower.parent : undefined;
        const chain = await Scope.chain(
            Snapshot.live(this.store.database),
            parent ?? request.below,
        );
        const scopes = [
            ...(parent === undefined ? [] : [request.below]),
            ...chain.map((link) => link.object.id),
        ];
        if (!scopes.includes(request.scope)) {
            throw new CallError("NOT_FOUND", {
                message: `${request.scope} does not contain ${request.below}`,
            });
        }
    }

    /** Require the via scopes' chains to have the replicated scope and those between. */
    async #requireVia(
        scope: string,
        via: readonly string[],
        between: readonly string[],
    ): Promise<void> {
        // require each replicated scope's chain to contain the replicated scope
        const chains = await Scope.chains(Snapshot.live(this.store.database), via);
        const scopes = new Set<string>();
        for (const [relay, links] of chains) {
            const ids = links.map((link) => link.object.id);
            if (!ids.includes(scope)) {
                throw new CallError("NOT_FOUND", {
                    message: `${scope} does not contain ${relay}`,
                });
            }
            for (const id of ids) {
                scopes.add(id);
            }
        }

        // require each scope between to contain a replicated scope
        const outside = between.find((id) => !scopes.has(id));
        if (outside !== undefined) {
            throw new CallError("NOT_FOUND", {
                message: `${outside} contains none of the replicated scopes`,
            });
        }
    }

    /** Require a reader to have `replicate` on the scope it replicates a chain via. */
    async requireReplicateVia(via: string, reader: AccessContext): Promise<void> {
        // require a scope type holding replicas of its chain
        const { database } = this.store;
        const scope = await Scope.object(Snapshot.live(database), via);
        const policy = this.store.authorizer.policy(scope);
        if (!Object.hasOwn(policy.definition.permissions, REPLICATE)) {
            throw new CallError("FORBIDDEN", {
                message: `a ${scope.type} has no replicas of its chain`,
            });
        }

        // decide for the principal standing for the follower, or for the caller
        const authorization = !AccessContext.isRemote(reader)
            ? await this.store.authorize(Transaction.reading(database), via, reader, { within: [] })
            : await this.store.authorize(Transaction.reading(database), via, reader);
        await authorization.require(policy.permission(REPLICATE), scope);
    }

    /** Open the audience a relayed replica is decided for. */
    async #replicaAudience(
        publication: sync.Publication,
        request: sync.Subscription,
        follower: ReplicaFollower,
    ): Promise<sync.Audience> {
        // read the scopes a follower outside the chains replicas them via
        const isContained = publication.audience === "contained";
        const via = isContained ? ObjectSource.#chainParameters(request).via : [];
        const replicating = via.length === 0 ? {} : { via };

        // decide a principal's chain by containment in the universe as a principal inside its parent's chain
        if ("subject" in follower && isContained) {
            return ObjectAudience.of(
                this.store,
                Scope.universe.id,
                AccessContext.of(Caller.of(follower.subject), this.store.clock.now()),
                {
                    isContained,
                    within: await this.#within(follower.parent ?? request.below),
                    ...replicating,
                },
            );
        }
        // decide a principal's other rows in the replicated scope
        else if ("subject" in follower) {
            const { parent } = follower;
            const within = parent === undefined ? [] : await this.#within(parent);

            return ObjectAudience.of(
                this.store,
                request.scope,
                AccessContext.of(Caller.of(follower.subject), this.store.clock.now()),
                { within },
            );
        }
        // decide a caller's chain by containment, with its fields from the scope it acts in
        else if (isContained) {
            return ObjectAudience.of(
                this.store,
                request.below,
                follower.context.access(request.below),
                {
                    isContained,
                    ...replicating,
                },
            );
        }
        // decide a caller in a relayed replica's scope here, else in the replicated scope at its home
        else {
            const { database } = this.store;
            const { tables } = publication.replica(request);
            const replicated = await Promise.all(
                tables.map((table) =>
                    sync.Replica.isReplicated(Snapshot.live(database), request.scope, table),
                ),
            );
            const scope = replicated.every(Boolean) ? request.below : request.scope;

            return ObjectAudience.of(this.store, scope, follower.context.access(scope));
        }
    }

    /** List a principal's scopes: a scope and its enclosing scopes below the universe. */
    async #within(
        scope: string,
        database: DatabaseConnection = this.store.database,
    ): Promise<ObjectReference[]> {
        const links = await Scope.chain(Snapshot.live(database), scope);

        return links.flatMap((link) => (link.object.id === Scope.universe.id ? [] : [link.object]));
    }

    /** Decide whether a caller may list a served durable object. */
    async #lists(authorization: Authorization, reference: ObjectReference): Promise<boolean> {
        const object = this.store.objects.find(
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
        pages: AsyncGenerator<sync.Page>,
        scope: string,
        authorization: Authorization,
        signal: AbortSignal,
    ): AsyncGenerator<sync.Page> {
        // collect the scope's events, waking the loop when one arrives
        const received: JsonValue[] = [];
        const readable = new Map<string, boolean>();
        const waker = { arrival: Promise.withResolvers<void>() };
        let stop: (() => void) | undefined;
        let last: sync.Page | undefined;
        let next = pages.next();
        try {
            while (!signal.aborted) {
                // wait for the next page unless events wait
                const arrived =
                    received.length > 0
                        ? { kind: "events" as const }
                        : await Promise.race([
                              next.then((page) => ({ kind: "page" as const, page })),
                              waker.arrival.promise.then(() => ({ kind: "events" as const })),
                          ]);

                // yield the next page, listening for events after the first
                if (arrived.kind === "page") {
                    if (arrived.page.done === true) {
                        return;
                    }
                    stop ??= this.store.requireEphemeral().tracker.listen(scope, (event) => {
                        received.push(event);
                        waker.arrival.resolve();
                    });
                    last = arrived.page.value;
                    next = pages.next();
                    yield arrived.page.value;
                    continue;
                }
                waker.arrival = Promise.withResolvers<void>();

                // yield the readable events
                const broadcasts = await this.#readable(
                    received.splice(0),
                    readable,
                    authorization,
                );
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
            stop?.();
            await pages.return(undefined);
        }
    }

    /** Filter the events of objects the caller may list, remembering each object's decision. */
    async #readable(
        sent: readonly JsonValue[],
        readable: Map<string, boolean>,
        authorization: Authorization,
    ): Promise<sync.Broadcast[]> {
        const broadcasts: sync.Broadcast[] = [];
        for (const entry of sent) {
            // decide each object once
            const { event, ...reference } = SENT_EVENT.parse(entry);
            const topic = ObjectReference.key(reference);
            let isRead = readable.get(topic);
            if (isRead === undefined) {
                isRead = await this.#lists(authorization, reference);
                readable.set(topic, isRead);
            }

            // take a readable object's event
            if (isRead) {
                broadcasts.push({ topic, event });
            }
        }

        return broadcasts;
    }

    /** Revise each listed replica whose action changed or left, and list the new ones. */
    static #revise(
        listed: Map<string, ReplicaEntry>,
        replicas: ReadonlyMap<string, ReplicaIntent & { digest: string }>,
    ): void {
        // abort and forget each listed replica whose digest changed or left
        for (const [key, replica] of listed) {
            if (replicas.get(key)?.digest !== replica.digest) {
                replica.revised.abort();
                listed.delete(key);
            }
        }

        // list the new replicas
        for (const [key, replica] of replicas) {
            if (!listed.has(key)) {
                listed.set(key, { ...replica, revised: new AbortController() });
            }
        }
    }

    /** Build a caller's replica feed options, ending at a completed page once it lapses. */
    static #followOptions(
        audience: sync.Audience,
        context: RequestContext,
        options: {
            readonly previous: Readonly<Record<string, sync.Query>> | undefined;
            readonly refresh: sync.Subscription["refresh"];
            readonly origin: sync.Subscription["origin"];
        },
    ): sync.FeedOptions {
        const { previous, refresh, origin } = options;

        return {
            audience,
            drain: context.signal,
            ...(previous === undefined ? {} : { previous }),
            ...(refresh === undefined ? {} : { every: Duration.milliseconds(refresh.every) }),
            ...(origin === undefined ? {} : { origin }),
        };
    }

    /** List the tables a query reads: its rows', its includes' and its followed relations'. */
    static #tablesOf(query: sync.Query): Table[] {
        return new sync.Node("", query, query.scopes).nodes().map((node) => node.table);
    }

    /** Mark each page with the scopes it reads, and each completed one with its event. */
    static async *#marking(
        pages: AsyncIterable<sync.Page>,
        chain: readonly string[],
    ): AsyncGenerator<sync.Page> {
        for await (const page of pages) {
            yield ObjectSource.#marked({ ...page, scopes: [...chain] });
        }
    }

    /** Mark a completed page with its position as event identifier, for resumed streams. */
    static #marked(page: sync.Page): sync.Page {
        return page.complete
            ? withEventMeta(page, { id: `${page.position.epoch}/${page.position.sequence}` })
            : page;
    }

    /** Widen each query to the scopes a replica has rows of its table for. */
    static #withReplicas(
        queries: Record<string, sync.Query>,
        held: Awaited<ReturnType<typeof sync.Replica.tables>>,
    ): Record<string, sync.Query> {
        return Object.fromEntries(
            Object.entries(queries).map(([name, query]) => {
                // take a query of every scope
                if (query.scopes === "every") {
                    return [name, query];
                }

                // add each replicated scope with the query's table
                const table = query.table[TABLE].sqlName;
                const replicated = held.filter((replica) => replica.tables.includes(table));
                const scopes = [
                    ...new Set([...query.scopes, ...replicated.map((replica) => replica.scope)]),
                ];

                return [name, { ...query, scopes }];
            }),
        );
    }
}

/** What the replica controller does with a replica: follow it from its publisher, or drop it. */
type ReplicaIntent =
    | {
          /** A request follows the replica. */
          readonly action: "follow";
          /** The request following the replica. */
          readonly subscription: sync.Subscription;
          /** The publisher streaming the replica's pages. */
          readonly publisher: sync.Publisher;
      }
    | {
          /** The held replica drops for want of a request. */
          readonly action: "drop";
          /** The subscription the held replica recorded. */
          readonly subscription: sync.Subscription;
      };

/** A replica the replica controller lists, with the digest each listing compares. */
type ReplicaEntry = ReplicaIntent & {
    /** The digest of the action and subscription. */
    readonly digest: string;
    /** Aborts once a listing changes the action or the subscription. */
    readonly revised: AbortController;
};

/** Who a relayed replica is decided for: a principal where its rows live, or the caller. */
export type ReplicaFollower =
    | {
          /** The principal the replica is decided for. */
          readonly subject: Subject;
          /** The scope containing the follower's scope, when this database has no replica of it. */
          readonly parent?: string;
      }
    | { readonly context: RequestContext };
