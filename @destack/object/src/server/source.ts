import { ObjectError } from "../error/error.ts";
import {
    AccessContext,
    accessRelationship,
    accessRole,
    accessRolePermission,
    Caller,
    CHAIN_PUBLICATION,
    ChainParameters,
    CONTAINED,
    decisionTables,
    principal,
    Relationship,
} from "@destack/access";
import { Scope, ObjectReference, Subject, SyncError } from "@destack/sync";
import type { JsonObject, JsonValue } from "@destack/schema";
import {
    type Condition,
    type DatabaseConnection,
    LogPosition,
    Snapshot,
    TABLE,
    type Table,
    Transaction,
} from "@destack/db";

import { canonicalize, Duration, present, schema } from "@destack/schema";
import { CallError } from "../error/call.ts";
import { refuseInput } from "../error/error.ts";
import { type RequestContext } from "./context.ts";
import { withEventMeta } from "../procedure/index.ts";
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
import type { ObjectStore } from "./store.ts";

import type {} from "@destack/package/import-meta";

/** How long a caller's session authorizes a stream: an hour, as Firebase's ID tokens last. */
const SESSION_MILLISECONDS = 60 * 60_000;

/** The largest broadcast event, in bytes: 16 KiB, far above the tens of bytes a cursor takes. */
const BROADCAST_BYTES = 16 * 1024;

/** An event an object sends, with the object it is about. */
const SENT_EVENT = ObjectReference.extend({ event: schema.json() });

/** The events sent to a followed scope's objects, waiting for the follow to yield them. */
interface ScopeEvents {
    /** The events received and not yet yielded. */
    readonly received: JsonValue[];
    /** Settles once an event arrives, renewed as the follow takes them. */
    arrival: PromiseWithResolvers<void>;
}

/** The replicas a store streams: client feeds of scope queries, and databases below. */
export class ObjectSource {
    /** The store whose objects the replicas copy. */
    readonly store: ObjectStore;

    /** The publication of a scope's durable objects: named queries, or every type of its level. */
    readonly queriesPublication = sync.definePublication({
        name: QUERIES_PUBLICATION,
        parameters: QueriesParameters,
        audience: "caller",
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
            const compiled = Object.values(ObjectType.queries(this.store.followed, named, [scope]));

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

    /** The publication of a scope's ephemeral objects a client follows, writing its rows. */
    readonly ephemeralPublication = sync.definePublication({
        name: EPHEMERAL_PUBLICATION,
        parameters: EphemeralParameters,
        audience: "caller",
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
        replica: ({ name, scope }) =>
            new sync.Replica({
                name,
                scope,
                tables: this.store.external.flatMap((storage) =>
                    storage.objects.map((object) => object.table),
                ),
            }),
    });

    /** The publication of a scope's access and handed-down rows, for scopes inside or outside. */
    readonly chainPublication = sync.definePublication({
        name: CHAIN_PUBLICATION,
        parameters: ChainParameters,
        audience: "contained",
        replica: ({ name, scope, target, parameters }) =>
            this.store.authorizer.replicateChain({ name, scope, target }, parameters),
    });

    /** The publication of the access rows of the caller's checks in a scope's chain. */
    readonly accessPublication = sync.definePublication({
        name: ACCESS_PUBLICATION,
        parameters: schema.object({}),
        audience: "caller",
        replica: ({ name, scope }) => new sync.Replica({ name, scope, tables: decisionTables }),
    });

    /** Stream the replicas of a store's objects. */
    constructor(store: ObjectStore) {
        this.store = store;
    }

    /** Stream a publication's replica to the caller its audience allows. */
    async *relayed(
        subscription: sync.Subscription,
        context: RequestContext,
    ): AsyncGenerator<sync.Page> {
        // allow the caller by the publication's audience, serving its request until it closes or lapses
        const publication = this.publication(subscription);
        const follower = await this.#authorizeFollower(publication, subscription, context);
        // refuse a session whose credential lapsed already, for the follower to resume with a fresh one
        const credential = context.authentication?.credential;
        if (credential !== undefined && credential.expiresAt <= this.store.clock.now()) {
            throw SyncError.of("CREDENTIAL_LAPSED", "the caller's credential lapsed");
        }

        // record the caller's request for its session, and keep the caller's live link open while it streams
        const caller = ObjectSource.#caller(context);
        const requested =
            caller === undefined
                ? undefined
                : await this.#record(
                      subscription,
                      caller,
                      this.store.clock.now() + SESSION_MILLISECONDS,
                  );
        const subject = context.authentication?.caller.subject;
        const leave = subject === undefined ? undefined : await this.store.connect?.(subject);
        let isPaged = false;
        try {
            for await (const page of this.#relay(
                publication,
                subscription,
                follower,
                context,
                requested,
            )) {
                isPaged = true;
                yield page;
            }
        } catch (error) {
            // end a stream its caller's session lapsed after pages, refusing one before any for the follower to resume anew
            if (!isPaged || !SyncError.is(error, "CREDENTIAL_LAPSED")) {
                throw error;
            }
        } finally {
            await leave?.();
        }
    }

    /** Stream an authorized follower's replica by its publication. */
    async *#relay(
        publication: sync.Publication,
        subscription: sync.Subscription,
        follower: ReplicaFollower,
        context: RequestContext,
        requested: Recorded | undefined,
    ): AsyncGenerator<sync.Page> {
        // follow a scope's durable objects
        if (publication === this.queriesPublication) {
            yield* this.#durable(subscription, follower, context, requested);
        }
        // follow a scope's ephemeral objects
        else if (publication === this.ephemeralPublication) {
            yield* this.#ephemeral(subscription, follower, context, requested);
        }
        // follow a scope's external objects
        else if (publication === this.externalPublication) {
            yield* this.#external(subscription, follower, context, requested);
        }
        // follow the access rows of a scope's chain
        else if (publication === this.accessPublication) {
            yield* this.#access(subscription, follower, context, requested);
        }
        // relay any other replica to the follower it is for
        else {
            const { after, ...request } = subscription;
            yield* this.#replicate(request, after, follower, context.request.signal, {
                requested,
                drain: context.closing,
            });
        }
    }

    /** Allow a caller to a publication's replica for its target scope, returning who it is for. */
    async #authorizeFollower(
        publication: sync.Publication,
        subscription: sync.Subscription,
        context: RequestContext,
    ): Promise<ReplicaFollower> {
        // follow on behalf of the caller's own principal as the caller itself
        const { subject: given, after: _after, ...rest } = subscription;
        const isSelf = given !== undefined && Subject.same(ObjectSource.#principal(context), given);
        const subject = isSelf ? undefined : given;
        const request: Omit<sync.Subscription, "after"> =
            subject === undefined ? rest : { ...rest, subject };

        // decide by the publication's audience
        switch (publication.audience) {
            case "caller": {
                // decide for the caller, or for the principal it may follow on behalf of
                await this.store.enter(context, request.scope);
                if (subject === undefined) {
                    return { context };
                }
                await this.#requireFollow(subject, context);

                return { subject };
            }
            case "contained":
            case "reader": {
                return this.#follower(request, context);
            }
        }
    }

    /** Read a chain request's parameters, absent for other publications, refusing bad ones. */
    static #chainParameters(
        request: Pick<sync.Subscription, "publication" | "parameters">,
    ): ChainParameters | undefined {
        // read no chain parameters for other publications
        if (request.publication !== CHAIN_PUBLICATION) {
            return undefined;
        }

        // parse them, refusing malformed parameters as a bad request
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
        const standing = await this.#standing(request.target);

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

        // decide a via replica on its via scopes, in its audience
        const via = ObjectSource.#chainParameters(request)?.via ?? [];
        if (via.length > 0) {
            return { context };
        }

        // decide for the scope's principal or a caller living in it
        const following = context.access(request.target);
        const resident = [following.subject, ...following.groups].filter(
            (subject) => subject.id === request.target || subject.scope === request.target,
        );

        // let an outsider the target's replicate permission allows follow it, such as the machine a move targets
        if (resident.length === 0 && (await this.#mayReplicate(request.target, context))) {
            return { context };
        }
        // refuse any other outsider
        else if (resident.length === 0) {
            throw ObjectError.of(
                "FOLLOWER_OUTSIDE_TARGET",
                `only ${request.target} and its contents can follow its replicas`,
            );
        }

        // decide a replica on its scope
        if (this.publication(request).audience === "contained") {
            const isResident = resident.some((subject) => subject.scope === request.target);
            const decided = standing === undefined || isResident ? request.target : request.scope;
            await this.#requireReplicate(decided, resident, context);
        }

        return { context };
    }

    /** Read the principal following, refusing an anonymous follower as unauthenticated. */
    static #principal(context: RequestContext): Subject {
        const caller = context.caller();
        if (Caller.isAnonymous(caller)) {
            throw ObjectError.of("CREDENTIAL_MISSING", "missing caller credential");
        }

        return Caller.principal(caller);
    }

    /** Require the caller to have `follow` on a principal's object. */
    async #requireFollow(subject: Subject, context: RequestContext): Promise<void> {
        // require a principal type its followers declare
        const policy = this.store.authorizer.policy(subject);
        if (!Object.hasOwn(policy.definition.permissions, FOLLOW)) {
            throw ObjectError.of("FOLLOWERS_MISSING", `a ${subject.type} has no followers`);
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

    /** Decide whether a caller outside a scope has `replicate` on it, as a relation grants it. */
    async #mayReplicate(target: string, context: RequestContext): Promise<boolean> {
        // decide the scope's permission, none for a scope type without replicas of its chain
        const scope = await Scope.object(Snapshot.live(this.store.database), target);
        const policy = this.store.authorizer.policy(scope);
        if (!Object.hasOwn(policy.definition.permissions, REPLICATE)) {
            return false;
        }
        const authorization = await this.store.authorize(
            Transaction.reading(this.store.database),
            target,
            context.access(target),
        );
        const decision = await authorization.check(policy.permission(REPLICATE), scope);

        return decision.isAllowed;
    }

    /** Require `replicate` on a caller's object, or else on a scope. */
    async #requireReplicate(
        target: string,
        resident: readonly Subject[],
        context: RequestContext,
    ): Promise<void> {
        // list the permissions on the caller's objects, at the rows standing for them here
        const authorization = await this.store.authorize(
            Transaction.reading(this.store.database),
            target,
            context.access(target),
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
        const scope = await Scope.object(Snapshot.live(this.store.database), target);
        const policy = this.store.authorizer.policy(scope);
        const granting = [...personal];
        if (Object.hasOwn(policy.definition.permissions, REPLICATE)) {
            granting.push({ permission: policy.permission(REPLICATE), target: scope });
        }
        const [first] = granting;
        if (first === undefined) {
            throw ObjectError.of(
                "CHAIN_REPLICAS_MISSING",
                `a ${scope.type} has no replicas of its chain`,
            );
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

    /** Follow the row standing for a principal from the universe where it is missing. */
    async stand(subject: Subject | undefined, signal: AbortSignal): Promise<void> {
        // take the principal standing for itself, or the space an installation lives in
        if (subject === undefined || this.store.standing.length === 0) {
            return;
        }
        const standingFor = principal.installation.is(subject) ? subject.scope : subject.id;

        // find the standing follow of the type representing it, unless its row is here
        const object = this.#representing(standingFor);
        const kind = object?.represents;
        const follow = this.store.standing.find((each) =>
            each.shape.objects.some((type) => object?.same(type) === true),
        );
        if (
            kind === undefined ||
            follow === undefined ||
            (await this.#standing(standingFor)) !== undefined
        ) {
            return;
        }

        // follow the row's key, its parameter named for the principal kind
        await this.store.acquire(
            follow,
            { key: { scope: Scope.universe.id, [kind.name]: standingFor } },
            signal,
        );
    }

    /** Find the one decided type representing the principal kind of an identifier. */
    #representing(target: string): ObjectType | undefined {
        const representing = this.store.decided.filter(
            (type) =>
                type.represents !== undefined &&
                type.table[TABLE].column("id").definition.schema.safeParse(target).success,
        );
        if (representing.length > 1) {
            throw new TypeError(
                `types ${representing.map((type) => type.name).join(", ")} all represent ${target}`,
            );
        }

        return representing[0];
    }

    /** Find a principal's row in the decided type representing its kind, with its scope. */
    async #standing(
        target: string,
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
        const object = this.#representing(target);
        const kind = object?.represents;
        if (object === undefined || kind === undefined) {
            return undefined;
        }

        // read the principal's row, and stand for the principal inside the scope of the row
        const [row] = await Snapshot.live(database).select(object.table, ["id"], [[target]]);
        if (row === undefined) {
            return undefined;
        }
        const scope = schema.string().parse(row["scope"]);
        const subject = {
            packageId: kind.definition.packageId,
            type: kind.name,
            scope: kind.definition.isGlobal === true ? Scope.universe.id : scope,
            id: target,
        };

        return {
            object,
            reference: object.reference(scope, target),
            subject,
            parent: scope === Scope.universe.id ? undefined : scope,
        };
    }

    /** Filter a follow's queries to the types this store serves, as others have no rows here. */
    #served(named: QueriesParameters["queries"]): QueriesParameters["queries"] {
        return named === undefined
            ? undefined
            : Object.fromEntries(
                  Object.entries(named).filter(([, query]) =>
                      this.store.durable.some((object) => object.name === query.object),
                  ),
              );
    }

    /** Follow a scope's durable objects for a follower. */
    async *#durable(
        subscription: sync.Subscription,
        follower: ReplicaFollower,
        context: RequestContext,
        requested: Recorded | undefined,
    ): AsyncGenerator<sync.Page> {
        // open the audience
        const { scope, after, previous } = subscription;
        const reader = this.#reader(follower, scope);
        const audience = await ObjectAudience.of(this.store, scope, reader);

        // compile a request's queries over the scope's chain now
        const compile = (parameters: JsonObject) =>
            ObjectType.queries(
                this.store.durable,
                this.#served(QueriesParameters.parse(parameters).queries),
                audience.chain,
            );
        const follow = (feed: sync.Feed) =>
            this.#replicating(feed, {
                subscription,
                caller: Caller.principal(reader),
                audience,
                compile,
                after,
                previous,
                requested,
                drain: context.closing,
                signal: context.request.signal,
            });

        // audit watching each audited object type
        const tables = new Set(
            Object.values(compile(subscription.parameters)).flatMap((query) =>
                sync.Node.tables(query),
            ),
        );
        const audited = this.store.objects.filter(
            (object) =>
                (object.isReadAudited || this.store.isAccessAudited) && tables.has(object.table),
        );
        const pages = audited.reduce(
            (source, object) => () => {
                const { action, values } = object.auditCall("watch", {}, scope);

                return this.store
                    .audit(scope, context.access(scope), {
                        package: object.package,
                        requestId: context.requestId,
                    })
                    .stream(action, values, source);
            },
            () => follow(this.store.feed),
        );
        yield* pages();
    }

    /** Follow queries over their scope's chain and the replicas a caller sees below it. */
    async *#replicating(
        feed: sync.Feed,
        followed: {
            readonly subscription: sync.Subscription;
            readonly caller: Subject | undefined;
            readonly audience: ObjectAudience;
            readonly compile: (parameters: JsonObject) => Record<string, sync.Query>;
            readonly after: LogPosition | undefined;
            readonly previous: JsonObject | undefined;
            readonly requested: Recorded | undefined;
            readonly drain: AbortSignal;
            readonly signal: AbortSignal;
        },
    ): AsyncGenerator<sync.Page> {
        // read the replicas the reader holds now, and compile parameters over them
        const { subscription, caller, audience, compile, signal } = followed;
        const { database } = this.store;
        const { scope } = subscription;
        const { generation } = database.state.commits;
        let held = await sync.Replica.tables(database, scope, caller);
        let parameters = subscription.parameters;
        const queries = (requested: JsonObject) =>
            ObjectSource.#withReplicas(compile(requested), held);

        // reshape the stream once the replicas the reader holds change, or the scope's chain grows or moves
        let stream: sync.Stream | undefined;
        const names = new Set(
            [sync.replica, sync.replicaSubscription].map((table) => table[TABLE].sqlName),
        );
        const scopes = Scope.table[TABLE].sqlName;
        const watching = new AbortController();
        const failed = new AbortController();
        const watched = database.log
            .until(
                async (tables) => {
                    // reshape over the replicas the reader holds anew
                    const current = canonicalize(held);
                    const chain = canonicalize(audience.chain);
                    if ([...tables].some((table) => names.has(table))) {
                        held = await sync.Replica.tables(database, scope, caller);
                    }
                    if (tables.has(scopes)) {
                        await audience.refresh();
                    }
                    const isChanged =
                        canonicalize(held) !== current || canonicalize(audience.chain) !== chain;
                    if (isChanged) {
                        stream?.reshape(queries(parameters));
                    }

                    return false;
                },
                AbortSignal.any([signal, watching.signal]),
                generation,
            )
            .catch((error: unknown) => {
                if (!watching.signal.aborted && !signal.aborted) {
                    failed.abort(error);
                }
            });

        // follow the queries, moving to a request's changed parameters in place, until the watch fails
        const { previous } = followed;
        const request: sync.StreamRequest | undefined =
            followed.requested === undefined
                ? undefined
                : {
                      ...followed.requested,
                      feed: this.store.feed,
                      parameters,
                      queries: async (requested) => {
                          parameters = requested;

                          return queries(requested);
                      },
                  };
        const streaming = AbortSignal.any([signal, failed.signal]);
        const subscribed = feed.subscribe(queries(parameters), followed.after, streaming, {
            audience,
            scopes: [...new Set([...audience.chain, ...held.map((each) => each.scope)])],
            drain: followed.drain,
            ...(request === undefined ? {} : { request }),
            ...(previous === undefined ? {} : { previous: queries(previous) }),
            ...(subscription.refresh === undefined
                ? {}
                : { every: Duration.milliseconds(subscription.refresh.every) }),
            opened: (opened) => {
                stream = opened;
            },
        });
        try {
            for await (const page of subscribed) {
                const read = [...new Set([...audience.chain, ...held.map((each) => each.scope)])];
                yield ObjectSource.#marked({ ...page, scopes: read });
            }
        } finally {
            watching.abort();
            await watched;
        }

        // end the stream with the watch's failure
        failed.signal.throwIfAborted();
    }

    /** Follow a scope's ephemeral objects for a writer. */
    async *#ephemeral(
        subscription: sync.Subscription,
        follower: ReplicaFollower,
        context: RequestContext,
        requested: Recorded | undefined,
    ): AsyncGenerator<sync.Page> {
        // track the writer's rows
        const { scope, after, previous } = subscription;
        const { queries, writerId } = EphemeralParameters.parse(subscription.parameters);
        const store = this.store.requireEphemeral();
        await store.ready();
        const release = store.track(EphemeralStorage.writerKey(context.access(scope), writerId));
        const compiled = ObjectType.queries(store.objects, queries, [scope]);
        const compile = (parameters: JsonObject) =>
            ObjectType.queries(store.objects, EphemeralParameters.parse(parameters).queries, [
                scope,
            ]);

        // collect the scope's events for the whole follow, kept across its restarts
        const events: ScopeEvents = { received: [], arrival: Promise.withResolvers<void>() };
        const stop = store.tracker.listen(ObjectSource.#eventTopic(scope), ({ event }) => {
            events.received.push(event);
            events.arrival.resolve();
        });
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
                    const options = this.#feedOptions(
                        subscription,
                        audience,
                        audience.chain,
                        context,
                        {
                            requested,
                            compile,
                            previous: isFirst ? reflected : undefined,
                        },
                    );
                    const pages = store.feed.subscribe(
                        compiled,
                        isFirst ? after : undefined,
                        signal,
                        options,
                    );

                    return this.#interleave(pages, events, audience, signal);
                },
            );
        } finally {
            stop();
            release();
        }
    }

    /** Follow a scope's external objects for the caller, decided in memory until access changes. */
    async *#external(
        subscription: sync.Subscription,
        follower: ReplicaFollower,
        context: RequestContext,
        requested: Recorded | undefined,
    ): AsyncGenerator<sync.Page> {
        // open the scope's files
        const { scope, after, previous } = subscription;
        const { queries } = QueriesParameters.parse(subscription.parameters);
        const external = ExternalStorage.of(
            this.store.external,
            Object.values(queries ?? {}).map((query) => query.object),
        );
        const feed = new sync.Feed(
            await external.open(scope),
            external.objects.map((object) => object.table),
            {},
        );
        const compiled = ObjectType.queries(external.objects, queries, [scope]);
        const compile = (parameters: JsonObject) =>
            ObjectType.queries(external.objects, QueriesParameters.parse(parameters).queries, [
                scope,
            ]);
        const earlier =
            previous === undefined ? undefined : QueriesParameters.parse(previous).queries;
        const reflected = earlier && ObjectType.queries(external.objects, earlier, [scope]);

        // follow until access changes, or to a completed page once the caller lapses
        yield* this.#decided(scope, follower, context, "external", (audience, signal, isFirst) => {
            const options = this.#feedOptions(subscription, audience, audience.chain, context, {
                requested,
                compile,
                previous: isFirst ? reflected : undefined,
            });

            return feed.subscribe(compiled, isFirst ? after : undefined, signal, options);
        });
    }

    /** Follow the access rows the caller's checks read in a scope's chain. */
    async *#access(
        subscription: sync.Subscription,
        follower: ReplicaFollower,
        context: RequestContext,
        requested: Recorded | undefined,
    ): AsyncGenerator<sync.Page> {
        // follow until access changes
        const { scope, after } = subscription;
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
            const options = this.#feedOptions(subscription, sync.EVERYONE, chain, context, {
                requested,
                compile: () => queries,
                previous: undefined,
            });
            const pages = this.store.feed.subscribe(
                queries,
                isFirst ? after : undefined,
                signal,
                options,
            );

            return ObjectSource.#marking(pages, {
                scopes: [...chain],
                subjects: [...subjects],
            });
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
        while (!context.closing.aborted) {
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
            const watching = this.#watchAccess(audience, since, revised, signal);

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

    /** Abort a follower's stream at the first access change deciding its audience's rows. */
    async #watchAccess(
        audience: ObjectAudience,
        since: number,
        revised: AbortController,
        signal: AbortSignal,
    ): Promise<void> {
        const { chain } = audience;
        const tables = this.store.authorizer.watch(chain).map((watch) => watch.table);
        try {
            // abort the stream on the first page with a change the audience decides by
            const pages = this.store.database.log.follow(
                { tables, scopes: chain, after: since },
                signal,
            );
            for await (const page of pages) {
                const decisions = await Promise.all(
                    page.changes.map((change) => audience.decides(change)),
                );
                if (decisions.includes(true)) {
                    revised.abort();
                    break;
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
            throw ObjectError.of(
                "BROADCAST_TOO_LARGE",
                `a broadcast cannot exceed ${BROADCAST_BYTES} bytes`,
            );
        }
        await this.store.enter(context, scope);
        const reference = { ...target, scope };
        const authorization = await this.store.authorizeHere(
            Transaction.reading(this.store.database),
            scope,
            context.access(scope),
        );
        if (!(await this.#lists(authorization, reference))) {
            throw CallError.http("NOT_FOUND", {
                message: `${target.type} ${target.id} not found`,
            });
        }

        // send it to the scope's streams
        store.tracker.broadcast(ObjectSource.#eventTopic(scope), { ...reference, event });
    }

    /** The publications this store serves and follows, by name. */
    get publications(): ReadonlyMap<string, sync.Publication> {
        return new Map(
            [
                this.chainPublication,
                this.queriesPublication,
                this.ephemeralPublication,
                this.externalPublication,
                this.accessPublication,
                ...this.store.publications,
            ].map((publication) => [publication.name, publication]),
        );
    }

    /** Build the replica a request of a served publication asks for. */
    buildReplica(request: sync.SubscriptionRecord): sync.Replica {
        return this.publication(request).replica(request);
    }

    /** Find the served publication a request asks for, refusing any other. */
    publication(request: Pick<sync.Subscription, "publication">): sync.Publication {
        const publication = this.publications.get(request.publication);
        if (publication === undefined) {
            throw ObjectError.of(
                "PUBLICATION_NOT_FOUND",
                `publication ${request.publication} not found`,
            );
        }

        return publication;
    }

    /** Record a caller's request here, which the stream serving it follows on any instance. */
    async request(subscription: sync.Subscription, caller = this.#inProcess): Promise<void> {
        await sync.Subscription.request(this.store.database, subscription, caller);
    }

    /** Record an authenticated caller's request of its own stream. */
    async requestFrom(subscription: sync.Subscription, context: RequestContext): Promise<void> {
        // require a known publication and an authenticated caller, whose key the request carries
        this.publication(subscription);
        const caller = ObjectSource.#caller(context);
        if (caller === undefined) {
            throw ObjectError.of("CREDENTIAL_MISSING", "missing caller credential");
        }
        await this.request(subscription, caller);
    }

    /** Read the key of an authenticated caller, absent for an anonymous one. */
    static #caller(context: RequestContext): string | undefined {
        const subject = context.authentication?.caller.subject;

        return subject === undefined ? undefined : Subject.key(subject);
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

        // stream the replica, following its follower's request
        const requested = await this.#record(subscription, this.#inProcess);
        const request = this.#streamRequest(
            requested,
            subscription.parameters,
            (parameters) => this.buildReplica({ ...subscription, parameters }).queries,
        );
        yield* this.store.feed.subscribe(
            replica.queries,
            after,
            signal,
            request === undefined ? {} : { request },
        );
    }

    /** Stream a replica's pages to a database below in this process. */
    async *replicate(
        request: sync.Subscription,
        after: LogPosition | undefined,
        follower: ReplicaFollower,
        signal: AbortSignal,
    ): AsyncGenerator<sync.Page> {
        const requested = await this.#record(request, this.#inProcess);
        yield* this.#replicate(request, after, follower, signal, { requested });
    }

    /** Stream a replica's pages to a database below, serving its follower's request. */
    async *#replicate(
        request: sync.Subscription,
        after: LogPosition | undefined,
        follower: ReplicaFollower,
        signal: AbortSignal,
        serving: { readonly requested: Recorded | undefined; readonly drain?: AbortSignal },
    ): AsyncGenerator<sync.Page> {
        // require the replicated scope to contain the follower's for a chain
        const publication = this.publication(request);
        if (publication.audience === "contained") {
            await this.#requireContaining(request, follower);
        }

        // allow the follower to a chain by containment and to the universe's rows as their reader
        const audience = await this.#replicaAudience(publication, request, follower);

        // relay a replicated scope once its replica has a position, to followers other than its source
        const replica = publication.replica(request);
        const { previous, origin } = request;
        await sync.Replica.requireRelayable(this.store.database, request.name, request.scope);
        if (origin !== undefined) {
            await replica.requireAcyclic(this.store.database, origin);
        }

        // stream the replica from the parameters it reflects, publishing past commits in its follower's chain
        const reflected =
            previous === undefined
                ? undefined
                : publication.replica({ ...request, parameters: previous }).queries;
        const streamRequest = this.#streamRequest(
            serving.requested,
            request.parameters,
            (parameters) => publication.replica({ ...request, parameters }).queries,
        );
        let stream: sync.Stream | undefined;
        const chain = await Scope.chain(Snapshot.live(this.store.database), request.target);
        const targeted = chain.map((link) => link.object.id);
        const pages = this.store.feed.subscribe(replica.queries, after, signal, {
            audience,
            scopes: [...new Set([replica.scope, ...replica.reads, ...targeted])],
            ...(serving.drain === undefined ? {} : { drain: serving.drain }),
            ...(streamRequest === undefined ? {} : { request: streamRequest }),
            ...(reflected === undefined ? {} : { previous: reflected }),
            opened: (opened) => {
                stream = opened;
            },
        });

        // send a relayed replica's home position with each completed page at this database's head
        const isRelayed =
            (await sync.Replica.origin(this.store.database, request.name, request.scope)) !==
            undefined;
        yield* isRelayed ? this.#originating(pages, request, () => stream?.beat()) : pages;
    }

    /** Send a relayed replica's home position with each completed page and each move of it. */
    async *#originating(
        pages: AsyncIterable<sync.Page>,
        request: Pick<sync.Subscription, "name" | "scope">,
        beat: () => void,
    ): AsyncGenerator<sync.Page> {
        // read the pages one at a time, closing them once the follower stops
        const { database } = this.store;
        const { lag } = this.store.authorizer;
        const iterator = pages[Symbol.asyncIterator]();
        let next: Promise<IteratorResult<sync.Page>> | undefined;
        let caught: sync.Page | undefined;
        try {
            for (;;) {
                // wait for the pending page or a move of the relayed replica's mark
                const moved = Promise.withResolvers<undefined>();
                const unwatch = sync.Replica.watchMarked(database, request, () =>
                    moved.resolve(undefined),
                );
                next ??= iterator.next();
                const result = await Promise.race([next, moved.promise]).finally(unwatch);
                next = result === undefined ? next : undefined;
                const isLive = sync.Replica.isLive(database, request.name, request.scope, lag);

                // forward a moved home position, or beat a stream behind this log's head
                if (result === undefined) {
                    const forwarded = await this.#forwarded(request, caught, isLive, beat);
                    caught = forwarded ?? caught;
                    if (forwarded !== undefined) {
                        yield forwarded;
                    }
                }
                // end with the stream
                else if (result.done === true) {
                    return;
                }
                // forward a bare page while live, which keeps the follower live
                else if (ObjectSource.#isBare(result.value)) {
                    if (isLive) {
                        const origin = await this.#originOf(request, result.value.position);
                        caught = origin === undefined ? result.value : { ...result.value, origin };
                        yield caught;
                    }
                }
                // attach the home position to a completed page a follower records
                else {
                    const page = result.value;
                    const origin = page.complete
                        ? await sync.Replica.originAt(database, request, page.position)
                        : undefined;
                    const sent = origin === undefined ? page : { ...page, origin };
                    caught = page.complete ? sent : undefined;
                    yield sent;
                }
            }
        } finally {
            // close the pages behind any pending read, which the stream's signal ends
            void iterator.return?.().catch(() => {});
        }
    }

    /** Answer the bare page forwarding a relayed replica's moved mark, or beat a stream behind. */
    async #forwarded(
        request: Pick<sync.Subscription, "name" | "scope">,
        caught: sync.Page | undefined,
        isLive: boolean,
        beat: () => void,
    ): Promise<sync.Page | undefined> {
        // forward nothing before a completed page, while stale, or for a mark that did not move
        const { database } = this.store;
        const marked = sync.Replica.marked(database, request.name, request.scope)?.position;
        const origin = caught?.origin;
        const isMoved =
            marked !== undefined &&
            (origin === undefined ||
                origin.log !== marked.log ||
                LogPosition.isAfter(marked, origin));
        if (caught === undefined || marked === undefined || !isLive || !isMoved) {
            return undefined;
        }

        // beat a stream behind this log's head, which then publishes the mark itself
        const head = await database.log.position();
        if (LogPosition.isAfter(head, caught.position)) {
            beat();

            return undefined;
        }

        return {
            reset: false,
            complete: true,
            changes: [],
            position: caught.position,
            origin: marked,
        };
    }

    /** Decide whether a page only moves a position: no rows, results, reset or reshape. */
    static #isBare(page: sync.Page): boolean {
        return (
            page.complete &&
            !page.reset &&
            page.changes.length === 0 &&
            (page.results ?? []).length === 0 &&
            page.parameters === undefined &&
            page.scopes === undefined &&
            page.subjects === undefined
        );
    }

    /** Read the home position a relayed replica's rows reflect at a position of this log. */
    async #originOf(
        request: Pick<sync.Subscription, "name" | "scope">,
        position: LogPosition,
    ): Promise<LogPosition | undefined> {
        // take the mark once the log has nothing past the position, else what the rows recorded
        const { database } = this.store;
        const head = await database.log.position();
        const marked = sync.Replica.marked(database, request.name, request.scope);
        const isCaughtUp = !LogPosition.isAfter(head, position);

        return marked !== undefined && isCaughtUp
            ? marked.position
            : await sync.Replica.originAt(database, request, position);
    }

    /** Require a chain replica's scope and claimed chain to contain the follower's scope. */
    async #requireContaining(request: sync.Subscription, follower: ReplicaFollower): Promise<void> {
        // read the scopes an outside follower replicates via, else those containing it
        const chain = ObjectSource.#chainParameters(request);
        const via = chain?.via ?? [];
        const scopes =
            via.length > 0
                ? await this.#viaScopes(request.scope, via, chain?.between ?? [])
                : await this.#containing(request, follower);

        // require a chain's scopes and the scopes it replicates to be the follower's scopes
        const claimed = [
            ...(chain?.scopes ?? []),
            ...(chain?.replicated ?? []).map((replicated) => replicated.scope),
        ];
        const foreign = claimed.find((scope) => !scopes.has(scope));
        if (foreign !== undefined) {
            throw ObjectError.of(
                "SCOPE_NOT_CONTAINED",
                `${foreign} does not contain ${request.target}`,
            );
        }
    }

    /** Read the scopes containing a follower's, requiring the replicated scope among them. */
    async #containing(request: sync.Subscription, follower: ReplicaFollower): Promise<Set<string>> {
        // read the scopes containing the follower's
        const parent = "subject" in follower ? follower.parent : undefined;
        const chain = await Scope.chain(
            Snapshot.live(this.store.database),
            parent ?? request.target,
        );
        const scopes = new Set([
            request.target,
            Scope.universe.id,
            ...chain.map((link) => link.object.id),
        ]);

        // require the replicated scope to contain the follower's
        if (!scopes.has(request.scope)) {
            throw ObjectError.of(
                "SCOPE_NOT_CONTAINED",
                `${request.scope} does not contain ${request.target}`,
            );
        }

        return scopes;
    }

    /** Read the via scopes' chains, each with the replicated scope and those between. */
    async #viaScopes(
        scope: string,
        via: readonly string[],
        between: readonly string[],
    ): Promise<Set<string>> {
        // require each replicated scope's chain to contain the replicated scope
        const chains = await Scope.chains(Snapshot.live(this.store.database), via);
        const scopes = new Set<string>();
        for (const [relay, links] of chains) {
            const ids = links.map((link) => link.object.id);
            if (!ids.includes(scope)) {
                throw ObjectError.of("SCOPE_NOT_CONTAINED", `${scope} does not contain ${relay}`);
            }
            for (const id of ids) {
                scopes.add(id);
            }
        }

        // require each scope between to contain a replicated scope
        const outside = between.find((id) => !scopes.has(id));
        if (outside !== undefined) {
            throw ObjectError.of(
                "SCOPE_NOT_CONTAINED",
                `${outside} contains none of the replicated scopes`,
            );
        }

        return new Set([...scopes, Scope.universe.id]);
    }

    /** Require a reader to have `replicate` on the scope it replicates a chain via. */
    async requireReplicateVia(via: string, reader: AccessContext): Promise<void> {
        // require a scope type with replicas of its chain
        const { database } = this.store;
        const scope = await Scope.object(Snapshot.live(database), via);
        const policy = this.store.authorizer.policy(scope);
        if (!Object.hasOwn(policy.definition.permissions, REPLICATE)) {
            throw ObjectError.of(
                "CHAIN_REPLICAS_MISSING",
                `a ${scope.type} has no replicas of its chain`,
            );
        }

        // decide for the follower inside the scopes its row places it, wherever it calls from
        const authorization = await this.store.authorize(
            Transaction.reading(database),
            via,
            reader,
        );
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
        const via = isContained ? (ObjectSource.#chainParameters(request)?.via ?? []) : [];
        const replicating = via.length === 0 ? {} : { via };

        // decide a principal's chain by containment in the universe as a principal inside its parent's chain
        if ("subject" in follower && isContained) {
            return ObjectAudience.of(
                this.store,
                Scope.universe.id,
                AccessContext.of(Caller.of(follower.subject), this.store.clock.now()),
                {
                    isContained,
                    within: await this.#within(follower.parent ?? request.target),
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
                request.target,
                follower.context.access(request.target),
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
            const scope = replicated.every(Boolean) ? request.target : request.scope;

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

    /** Interleave a stream's pages with the readable objects' events, once it yielded a page. */
    async *#interleave(
        pages: AsyncGenerator<sync.Page>,
        events: ScopeEvents,
        audience: Pick<ObjectAudience, "authorization">,
        signal: AbortSignal,
    ): AsyncGenerator<sync.Page> {
        // decide each object's events once per access the audience has
        const readable = new Map<string, boolean>();
        let decider = audience.authorization;
        let last: sync.Page | undefined;
        let next = pages.next();
        try {
            while (!signal.aborted) {
                // wait for the next page, or for events once a page gave their position
                const arrived =
                    last !== undefined && events.received.length > 0
                        ? { kind: "events" as const }
                        : await Promise.race([
                              next.then((page) => ({ kind: "page" as const, page })),
                              ...(last === undefined
                                  ? []
                                  : [
                                        events.arrival.promise.then(() => ({
                                            kind: "events" as const,
                                        })),
                                    ]),
                          ]);

                // yield the next page
                if (arrived.kind === "page") {
                    if (arrived.page.done === true) {
                        return;
                    }
                    last = arrived.page.value;
                    next = pages.next();
                    yield arrived.page.value;
                    continue;
                }
                events.arrival = Promise.withResolvers<void>();

                // yield the events readable under the audience's current access, deciding again once it changed
                const { authorization } = audience;
                if (authorization !== decider) {
                    readable.clear();
                    decider = authorization;
                }
                const broadcasts = await this.#readable(
                    events.received.splice(0),
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

    /** Build a caller's replica feed options, following its request across its chain. */
    #feedOptions(
        subscription: sync.Subscription,
        audience: sync.Audience,
        scopes: readonly string[],
        context: RequestContext,
        serving: {
            readonly requested: Recorded | undefined;
            readonly compile: (parameters: JsonObject) => Record<string, sync.Query>;
            readonly previous: Readonly<Record<string, sync.Query>> | undefined;
        },
    ): sync.FeedOptions {
        // follow the request, draining with the request's close
        const { refresh } = subscription;
        const { requested, compile, previous } = serving;
        const request = this.#streamRequest(requested, subscription.parameters, compile);

        return {
            audience,
            scopes,
            drain: context.closing,
            ...(request === undefined ? {} : { request }),
            ...(previous === undefined ? {} : { previous }),
            ...(refresh === undefined ? {} : { every: Duration.milliseconds(refresh.every) }),
        };
    }

    /** Build the request a stream follows, absent without one. */
    #streamRequest(
        requested: Recorded | undefined,
        parameters: JsonObject,
        compile: (parameters: JsonObject) => Record<string, sync.Query>,
    ): sync.StreamRequest | undefined {
        return requested === undefined
            ? undefined
            : {
                  ...requested,
                  feed: this.store.feed,
                  parameters,
                  queries: async (asked) => compile(asked),
              };
    }

    /** Record a caller's request here, with when its session lapses. */
    async #record(
        subscription: sync.Subscription,
        caller: string,
        lapsesAt?: number,
    ): Promise<Recorded> {
        // record the request, keyed by the caller
        await sync.Subscription.request(this.store.database, subscription, caller);
        const { name, scope, target } = subscription;
        const key = { name, scope, target, caller };

        return lapsesAt === undefined ? { key } : { key, lapsesAt };
    }

    /** The caller of in-process followers: this store's own principal. */
    get #inProcess(): string {
        return Subject.key(this.store.principal);
    }

    /** Mark each page with its scopes and subjects, and each completed one with its event. */
    static async *#marking(
        pages: AsyncIterable<sync.Page>,
        marks: Pick<sync.Page, "scopes" | "subjects">,
    ): AsyncGenerator<sync.Page> {
        for await (const page of pages) {
            yield ObjectSource.#marked({ ...page, ...marks });
        }
    }

    /** Read the tracker topic of a scope's object events. */
    static #eventTopic(scope: string): string {
        return `event/${scope}`;
    }

    /** Mark a completed page with its position as event identifier, for resumed streams. */
    static #marked(page: sync.Page): sync.Page {
        return page.complete
            ? withEventMeta(page, {
                  id: `${page.position.log}/${page.position.epoch}/${page.position.sequence}`,
              })
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

/** A follower's request as this source records it. */
type Recorded = Pick<sync.StreamRequest, "key" | "lapsesAt">;

/** Who a relayed replica is decided for: a principal where its rows live, or the caller. */
export type ReplicaFollower =
    | {
          /** The principal the replica is decided for. */
          readonly subject: Subject;
          /** The scope containing the follower's scope, when this database has no replica of it. */
          readonly parent?: string;
      }
    | { readonly context: RequestContext };
