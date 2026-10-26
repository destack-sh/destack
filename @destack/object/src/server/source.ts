import { ObjectError } from "../error/error.ts";
import {
    AccessContext,
    accessRelationship,
    accessRole,
    accessRolePermission,
    Caller,
    CHAIN_PUBLICATION,
    type Authentication,
    ChainParameters,
    CONTAINED,
    decisionTables,
    principal,
    Relationship,
} from "@destack/access";
import { Scope, ObjectReference, Subject, SyncError } from "@destack/sync";
import type { JsonValue } from "@destack/schema";
import {
    type Condition,
    type DatabaseConnection,
    LogPosition,
    Snapshot,
    TABLE,
    type Table,
    Transaction,
    DatabaseError,
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
    /** The streams served now, with the subscription and caller each serves. */
    readonly #streams = new Set<SourceStream>();

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
        // allow the caller by the publication's audience
        const publication = this.publication(subscription);
        const follower = await this.#authorizeFollower(publication, subscription, context);
        let isPaged = false;
        for await (const page of this.#relay(publication, subscription, follower, context)) {
            isPaged = true;
            yield page;
        }

        // refuse a stream its caller's credential lapsed before any page, for the follower to resume anew
        const lapse: unknown = context.signal.reason;
        if (!isPaged && SyncError.is(lapse, "CREDENTIAL_LAPSED")) {
            throw lapse;
        }
    }

    /** Stream an authorized follower's replica by its publication. */
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
    ): AsyncGenerator<sync.Page> {
        // open the audience
        const { scope, after, previous } = subscription;
        const { queries } = QueriesParameters.parse(subscription.parameters);
        const reader = this.#reader(follower, scope);
        const audience = await ObjectAudience.of(this.store, scope, reader);

        // follow the replica's queries, with the replicas the reader sees below the scope
        const { chain } = audience;
        const caller = Caller.principal(reader);
        const compile = (named: QueriesParameters["queries"]) =>
            ObjectType.queries(this.store.durable, this.#served(named), chain);
        const earlier =
            previous === undefined ? undefined : QueriesParameters.parse(previous).queries;
        const options = this.#feedOptions(
            subscription,
            audience,
            context,
            earlier && compile(earlier),
        );
        const follow = (feed: sync.Feed) =>
            this.#replicating(
                feed,
                scope,
                caller,
                audience,
                (current) => ObjectType.queries(this.store.durable, this.#served(queries), current),
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

    /** Follow queries over their scopes' chain and the replicas a caller sees below them. */
    async *#replicating(
        feed: sync.Feed,
        scope: string,
        caller: Subject | undefined,
        audience: ObjectAudience,
        compile: (chain: readonly string[]) => Record<string, sync.Query>,
        after: LogPosition | undefined,
        options: sync.FeedOptions,
        signal: AbortSignal,
    ): AsyncGenerator<sync.Page> {
        // read the replicas stored now, following them until the stream ends or drains
        const { database } = this.store;
        let held = await sync.Replica.tables(database, scope, caller);
        let reflected = options.previous && ObjectSource.#withReplicas(options.previous, held);
        let position = after;
        while (!signal.aborted && options.drain?.aborted !== true) {
            // compile the queries over the scope's chain now and the replicas held now
            const { chain } = audience;
            const current = canonicalize(held);
            const compiled = ObjectSource.#withReplicas(compile(chain), held);
            const rekept = new AbortController();
            const following = AbortSignal.any([signal, rekept.signal]);

            // follow again once the replicas change, or once the scope's chain grows or moves
            const names = [sync.replica, sync.replicaSubscription].map(
                (table) => table[TABLE].sqlName,
            );
            const scopes = Scope.table[TABLE].sqlName;
            const changed = database.log.until(async (tables) => {
                // follow again over replicas the reader holds anew
                if (
                    names.some((name) => tables.has(name)) &&
                    canonicalize(await sync.Replica.tables(database, scope, caller)) !== current
                ) {
                    return true;
                }
                // follow again over a chain a scope change grew or moved
                else if (tables.has(scopes)) {
                    await audience.refresh();

                    return canonicalize(audience.chain) !== canonicalize(chain);
                }
                // keep following otherwise
                else {
                    return false;
                }
            }, following);
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
            for await (const page of ObjectSource.#marking(subscribed, { scopes: read })) {
                yield page;
                position = page.complete ? page.position : position;
            }
            rekept.abort();
            await changed;

            // follow again over the replicas stored now
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
        const { scope, after, previous } = subscription;
        const { queries, writerId } = EphemeralParameters.parse(subscription.parameters);
        const store = this.store.requireEphemeral();
        const release = store.track(EphemeralStorage.writerKey(context.access(scope), writerId));
        const compiled = ObjectType.queries(store.objects, queries, [scope]);

        // collect the scope's events for the whole follow, kept across its restarts
        const events: ScopeEvents = { received: [], arrival: Promise.withResolvers<void>() };
        const stop = store.tracker.listen(scope, (event) => {
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
                        context,
                        isFirst ? reflected : undefined,
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
        const earlier =
            previous === undefined ? undefined : QueriesParameters.parse(previous).queries;
        const reflected = earlier && ObjectType.queries(external.objects, earlier, [scope]);

        // follow until access changes, or to a completed page once the caller lapses
        yield* this.#decided(scope, follower, context, "external", (audience, signal, isFirst) => {
            const options = this.#feedOptions(
                subscription,
                audience,
                context,
                isFirst ? reflected : undefined,
            );

            return feed.subscribe(compiled, isFirst ? after : undefined, signal, options);
        });
    }

    /** Follow the access rows the caller's checks read in a scope's chain. */
    async *#access(
        subscription: sync.Subscription,
        follower: ReplicaFollower,
        context: RequestContext,
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
            const options = this.#feedOptions(subscription, sync.EVERYONE, context, undefined);
            const pages = this.store.feed.subscribe(
                queries,
                isFirst ? after : undefined,
                signal,
                options,
            );

            return ObjectSource.#marking(pages, { scopes: [...chain], subjects: [...subjects] });
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
        store.tracker.broadcast(scope, { ...reference, event });
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
    buildReplica(request: sync.Subscription): sync.Replica {
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

    /** Confirm a subscription's streams past a watermark, through upstream for a replica here. */
    async confirm(subscription: sync.Subscription, watermark: sync.Watermark): Promise<void> {
        const history = await this.store.history(watermark);

        // confirm a watermark of the store's log on the subscription's streams
        if (history.kind === "local") {
            const key = ObjectSource.streamKey(subscription);
            for (const served of this.#streams) {
                if (served.key === key) {
                    served.stream.confirm(watermark);
                }
            }
        }
        // confirm upstream for a scope this store's replicas include
        else if (history.kind === "replicated") {
            await this.store.sink.request(watermark, [...history.including, ...history.followed]);
        }
        // refuse a history this store no longer has
        else if (history.kind === "stale") {
            throw DatabaseError.of(
                "EPOCH_STALE",
                `${watermark.scope} no longer has the watermark's history`,
            );
        }
        // refuse a scope with no home or replica here
        else {
            throw ObjectError.of(
                "HOME_OR_REPLICA_MISSING",
                `${watermark.scope} has no home or replica here`,
            );
        }
    }

    /** Confirm a follower's subscription past a watermark, as its publication allows. */
    async confirmRequest(
        subscription: sync.Subscription,
        watermark: sync.Watermark,
        context: RequestContext,
    ): Promise<void> {
        await this.#authorizeFollower(this.publication(subscription), subscription, context);
        await this.confirm(subscription, watermark);
    }

    /** Confirm a caller's streams of a scope at or past a position of this store's log. */
    confirmCaller(
        scope: string,
        authentication: Authentication | undefined,
        position: LogPosition,
    ): void {
        // confirm nothing for an anonymous caller, else each of its streams of the scope
        const subject = authentication?.caller.subject;
        if (subject === undefined) {
            return;
        }
        const caller = Subject.key(subject);
        for (const served of this.#streams) {
            if (served.scope === scope && served.caller === caller) {
                served.stream.confirm(position);
            }
        }
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

        // stream the replica, with the stream for its follower's confirmations
        yield* this.store.feed.subscribe(replica.queries, after, signal, {
            opened: (stream) => this.#track(stream, subscription, undefined),
            closed: (stream) => this.#untrack(stream),
        });
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

        // stream the replica from the parameters it reflects, ending at a completed page once drained
        const reflected =
            previous === undefined
                ? undefined
                : publication.replica({ ...request, parameters: previous }).queries;
        yield* this.store.feed.subscribe(replica.queries, after, signal, {
            audience,
            opened: (stream) => this.#track(stream, request, undefined),
            closed: (stream) => this.#untrack(stream),
            ...(reflected === undefined ? {} : { previous: reflected }),
            ...(drain === undefined ? {} : { drain }),
        });
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

        // require a chain's scopes to be the follower's scopes
        const claimed = chain?.scopes ?? [];
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

    /** Build a caller's replica feed options, ending at a completed page once it lapses. */
    #feedOptions(
        subscription: sync.Subscription,
        audience: sync.Audience,
        context: RequestContext,
        previous: Readonly<Record<string, sync.Query>> | undefined,
    ): sync.FeedOptions {
        const { refresh } = subscription;

        return {
            audience,
            drain: context.signal,
            opened: (stream) => this.#track(stream, subscription, context.authentication),
            closed: (stream) => this.#untrack(stream),
            ...(previous === undefined ? {} : { previous }),
            ...(refresh === undefined ? {} : { every: Duration.milliseconds(refresh.every) }),
        };
    }

    /** Track a stream of a subscription and caller until it ends. */
    #track(
        stream: sync.Stream,
        subscription: Pick<sync.Subscription, "name" | "scope" | "target" | "confirming">,
        authentication: Authentication | undefined,
    ): void {
        // list the stream under its subscription's key and scope, and its caller's key
        const subject = authentication?.caller.subject;
        const served: SourceStream = {
            stream,
            key: ObjectSource.streamKey(subscription),
            scope: subscription.scope,
            caller: subject === undefined ? undefined : Subject.key(subject),
        };
        this.#streams.add(served);

        // ask the new stream for the position its follower waits for
        if (subscription.confirming !== undefined) {
            stream.confirm(subscription.confirming);
        }
    }

    /** Stop tracking a stream that ended. */
    #untrack(stream: sync.Stream): void {
        for (const served of this.#streams) {
            if (served.stream === stream) {
                this.#streams.delete(served);
            }
        }
    }

    /** Key a subscription's streams by its replica and target. */
    static streamKey(subscription: Pick<sync.Subscription, "name" | "scope" | "target">): string {
        return `${subscription.name} ${subscription.scope} ${subscription.target}`;
    }

    /** List the tables a query reads: its rows', its includes' and its followed relations'. */
    static #tablesOf(query: sync.Query): Table[] {
        return new sync.Node("", query, query.scopes).nodes().map((node) => node.table);
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

/** A stream a source serves, with its subscription's key and scope, and its caller. */
interface SourceStream {
    /** The stream. */
    readonly stream: sync.Stream;
    /** The key of its subscription: replica name, scope and target. */
    readonly key: string;
    /** The scope its subscription replicates. */
    readonly scope: string;
    /** The key of the caller whose request it serves, absent in process. */
    readonly caller: string | undefined;
}

/** Who a relayed replica is decided for: a principal where its rows live, or the caller. */
export type ReplicaFollower =
    | {
          /** The principal the replica is decided for. */
          readonly subject: Subject;
          /** The scope containing the follower's scope, when this database has no replica of it. */
          readonly parent?: string;
      }
    | { readonly context: RequestContext };
