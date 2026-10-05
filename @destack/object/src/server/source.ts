import {
    accessRelationship,
    accessRole,
    accessRolePermission,
    ChainParameters,
    decisionTables,
    principal,
    Relationship,
    type Standing,
    type UniverseParameters,
} from "@destack/access";
import { ScopeProjector } from "./projected.ts";
import { Scope, ObjectReference, Subject } from "@destack/sync";
import type { JsonValue } from "@destack/schema";
import { Change, eq, LogPosition, Snapshot, TABLE, type Table } from "@destack/db";

import { aligned, Digest, Duration, found, present, schema } from "@destack/schema";

import { isServiceError, ServiceError } from "@destack/service/error";

import { type ServiceContext } from "@destack/service/server";
import { withEventMeta } from "@destack/service";
import * as sync from "@destack/sync";

import {
    ACCESS_SHAPE,
    EPHEMERAL_SHAPE,
    EXTERNAL_SHAPE,
    PROJECTION_SHAPE,
    ProjectionParameters,
    EphemeralParameters,
    QUERIES_SHAPE,
    QueriesParameters,
} from "../replica/replica.ts";
import { ObjectType, REPLICATE, REPRESENT, type ObjectStorage } from "../object/object.ts";
import { projected } from "../trait/projected.ts";

import { Authorization } from "./authorization.ts";

import { EphemeralStorage } from "./ephemeral.ts";
import { ObjectAudience, RecipientAudience } from "./audience.ts";

import type { Controller } from "@destack/service/control";

import { until } from "@destack/service/timer";

import type { ObjectServer, Subscriber, SubscriberEntry } from "./server.ts";

/** The largest broadcast event, in bytes: 16 KiB, far above the tens of bytes a cursor takes. */
const BROADCAST_BYTES = 16 * 1024;

/** An event an object sends, with the object it is about. */
const SENT_EVENT = ObjectReference.extend({ event: schema.json() });

/** The copies a server's objects stream: client feeds of a scope's queries, and replicas of scopes for databases below. */
export class ObjectSource {
    /** The server whose objects the copies keep. */
    readonly server: Omit<ObjectServer, "router">;
    /** The copies streaming now, by the scope or principal below they are for. */
    readonly #streaming = new Map<string, number>();

    /** Stream the copies of a server's objects. */
    constructor(server: Omit<ObjectServer, "router">) {
        this.server = server;
    }

    /** Report whether a copy streams now for a scope or principal below, such as a placed workload following this server. */
    follows(below: string): boolean {
        return this.#streaming.has(below);
    }

    /** Stream a shape's copy to the caller its audience admits. */
    async *relayed(
        subscription: sync.Subscription,
        context: ServiceContext,
    ): AsyncGenerator<sync.Page> {
        // admit the caller by the shape's audience, counting its copy while it streams
        const shape = this.#shape(subscription);
        const follower = await this.#admit(shape, subscription, context);
        const below = subscription.below;
        this.#streaming.set(below, (this.#streaming.get(below) ?? 0) + 1);
        try {
            yield* this.#relay(shape, subscription, follower, context);
        } finally {
            const remaining = found(this.#streaming, below) - 1;
            if (remaining === 0) {
                this.#streaming.delete(below);
            } else {
                this.#streaming.set(below, remaining);
            }
        }
    }

    /** Stream an admitted follower's copy by its shape. */
    async *#relay(
        shape: sync.Shape,
        subscription: sync.Subscription,
        follower: ReplicaFollower,
        context: ServiceContext,
    ): AsyncGenerator<sync.Page> {
        // follow a scope's durable objects
        if (shape === this.queriesShape) {
            yield* this.#durable(subscription, context);
        }
        // follow a scope's ephemeral objects
        else if (shape === this.ephemeralShape) {
            yield* this.#ephemeral(subscription, context);
        }
        // follow a scope's external objects
        else if (shape === this.externalShape) {
            yield* this.#external(subscription, context);
        }
        // follow the access rows of a scope's chain
        else if (shape === this.accessShape) {
            yield* this.#access(subscription, context);
        }
        // project a home's rows
        else if (shape === this.projectionShape) {
            yield* this.project(subscription, context.request.signal, context.signal);
        }
        // relay any other copy to the follower it is for
        else {
            const { after, ...request } = subscription;
            yield* this.replicate(request, after, follower, context.request.signal, context.signal);
        }
    }

    /** Admit a caller to a shape's copy for the scope below by the shape's audience, answering who the copy is for. */
    async #admit(
        shape: sync.Shape,
        subscription: sync.Subscription,
        context: ServiceContext,
    ): Promise<ReplicaFollower> {
        switch (shape.audience) {
            case "caller":
                await this.server.enter(context, subscription.scope);

                return { context };
            case "recipient":
                ObjectSource.#requireHome(subscription, context);

                return { context };
            case "contained":
            case "reader": {
                const { after: _after, ...request } = subscription;

                return this.#follower(request, context);
            }
        }
    }

    /** Require the caller to be the home a projection follows, or one of its installations. */
    static #requireHome(subscription: sync.Subscription, context: ServiceContext): void {
        const { subjects } = context.requireAuthentication().claims;
        const isHome = subjects.some(
            (subject) =>
                subject.id === subscription.below ||
                (principal.installation.is(subject) && subject.scope === subscription.below),
        );
        if (!isHome) {
            throw new ServiceError("FORBIDDEN", {
                message: `only ${subscription.below} follows the rows its residents receive`,
            });
        }
    }

    /** Decide who a relayed copy is for: the principal a served object stands for, or the caller. */
    async #follower(
        request: Omit<sync.Subscription, "after">,
        context: ServiceContext,
    ): Promise<ReplicaFollower> {
        // find the served object standing for the scope
        const standing = await this.#standing(request.below);

        // decide for the principal a served object stands for, which the caller represents
        if (standing !== undefined) {
            const { object, reference, subject, parent } = standing;
            const authorization = await this.server.authorize(
                this.server.database,
                reference.scope,
                context,
            );
            await authorization.require(object.permission(REPRESENT), reference);

            return { subject, ...(parent === undefined ? {} : { parent }) };
        }

        // decide for the caller, which is the scope's principal or lives in the scope
        const { subjects } = this.server.accessContext(context, request.below);
        const kept = subjects.filter(
            (subject) => subject.id === request.below || subject.scope === request.below,
        );
        if (kept.length === 0) {
            throw new ServiceError("FORBIDDEN", {
                message: `the caller keeps no copies for ${request.below}`,
            });
        }
        if (this.#shape(request).audience === "contained") {
            await this.#requireReplicate(request.below, kept, context);
        }

        return { context };
    }

    /** Require the `replicate` permission on a caller's own object, or else on the scope, to copy its chain. */
    async #requireReplicate(
        below: string,
        kept: readonly Subject[],
        context: ServiceContext,
    ): Promise<void> {
        // list the permissions on the caller's own objects
        const authorization = await this.server.authorize(this.server.database, below, context);
        const personal = kept.flatMap((subject) =>
            this.server.durable
                .filter(
                    (object) => object.policy.is(subject) && object.permissions.includes(REPLICATE),
                )
                .map((object) => ({
                    permission: object.permission(REPLICATE),
                    target: object.reference(subject.scope, subject.id),
                })),
        );

        // add the scope's permission, and require the first one
        const scope = await Scope.object(Snapshot.live(this.server.database), below);
        const policy = this.server.authorizer.policy(scope);
        const granting = [...personal];
        if (Object.hasOwn(policy.definition.permissions, REPLICATE)) {
            granting.push({ permission: policy.permission(REPLICATE), target: scope });
        }
        const [first] = granting;
        if (first === undefined) {
            throw new ServiceError("FORBIDDEN", {
                message: `no ${scope.type} keeps copies of its chain`,
            });
        }
        await authorization.require(first.permission, first.target);
    }

    /** Find the object standing for a principal callers act as, with the scopes the principal lives inside, absent when none stands for it here. */
    async standing(subject: Subject): Promise<Standing | undefined> {
        // find the object standing for the principal's identifier, standing for the principal itself
        const standing = await this.#standing(subject.id);
        if (standing === undefined || !Subject.same(standing.subject, subject)) {
            return undefined;
        }

        return {
            permission: standing.object.permission(REPRESENT),
            object: standing.reference,
            subject: standing.subject,
            within: standing.parent === undefined ? [] : await this.#within(standing.parent),
        };
    }

    /** Find the object standing for a scope as a principal among the served types the database is home of and the copied types standing here, with the scope containing it. */
    async #standing(below: string): Promise<
        | {
              readonly object: ObjectType;
              readonly reference: ObjectReference;
              readonly subject: Subject;
              readonly parent: string | undefined;
          }
        | undefined
    > {
        // look among the served types this database is home of and the copied types standing here
        const { database, standing } = this.server;
        const homed = this.server.durable.filter((object) => !database.copies(object.table));
        for (const object of [...homed, ...standing]) {
            // skip unrepresented types and other identifiers
            const table = object.table;
            const identifier = table[TABLE].column("id").definition.schema;
            if (!object.permissions.includes(REPRESENT) || !identifier.safeParse(below).success) {
                continue;
            }

            // read the object, skipping a type without the identity relation
            const [row] = await Snapshot.live(this.server.database).select(
                table,
                ["id"],
                [[below]],
            );
            const identity = Object.entries(object.mapping.relations).find(
                ([, field]) => field.column === object.mapping.id,
            );
            if (row === undefined || identity === undefined) {
                continue;
            }

            // stand for the principal the object's identifier is, as a user's `self` is
            const subject = this.server.authorizer.related(object.mapping, identity[0], row);
            const parent = row["parent"];
            if (subject !== undefined) {
                return {
                    object,
                    reference: object.reference(schema.string().parse(row["scope"]), below),
                    subject,
                    parent: typeof parent === "string" ? parent : undefined,
                };
            }
        }

        return undefined;
    }

    /** The shape of a scope's durable objects a caller follows: its named queries, or every listed type of the scope's level. */
    readonly queriesShape = sync.defineShape({
        name: QUERIES_SHAPE,
        parameters: QueriesParameters,
        audience: "caller",
        replica: ({ name, scope }) =>
            new sync.Replica({
                name,
                scope,
                tables: this.server.durable.map((object) => object.table),
            }),
    });

    /** The shape of a scope's ephemeral objects a client follows, owning the rows it writes while it follows them. */
    readonly ephemeralShape = sync.defineShape({
        name: EPHEMERAL_SHAPE,
        parameters: EphemeralParameters,
        audience: "caller",
        replica: ({ name, scope }) =>
            new sync.Replica({
                name,
                scope,
                tables: this.server.store().objects.map((object) => object.table),
            }),
    });

    /** The shape of a scope's external objects the caller reads, from the database their files open: its named queries. */
    readonly externalShape = sync.defineShape({
        name: EXTERNAL_SHAPE,
        parameters: QueriesParameters,
        audience: "caller",
        replica: ({ name, scope }) =>
            new sync.Replica({
                name,
                scope,
                tables: present(this.server.external, "the external files").objects.map(
                    (object) => object.table,
                ),
            }),
    });

    /** The shape of the access rows the caller's own checks read in a scope's chain: the scopes, their roles, and the relationships the caller's subjects hold. */
    readonly accessShape = sync.defineShape({
        name: ACCESS_SHAPE,
        parameters: schema.object({}),
        audience: "caller",
        replica: ({ name, scope }) => new sync.Replica({ name, scope, tables: decisionTables }),
    });

    /** The shape of the rows a home's residents receive, each decided for its recipient, which the home copies and projects. */
    readonly projectionShape = sync.defineShape({
        name: PROJECTION_SHAPE,
        parameters: ProjectionParameters,
        audience: "recipient",
        replica: ({ name, scope, parameters }) => {
            // project the source's rows into the home the copy is named for, through the served types projecting them
            const projectors = this.server.objects.flatMap((object) => {
                // keep a projecting type's projector for the named source
                const projector =
                    object.projected === undefined
                        ? undefined
                        : new ScopeProjector(object, object.projected, name);
                if (object.projected === undefined || projector === undefined) {
                    return [];
                }
                const source = projected.sourceOf(object.projected);
                const isSource =
                    source.name === parameters.type &&
                    source.policy.definition.packageId === parameters.packageId;

                return isSource ? [projector] : [];
            });
            if (projectors.length === 0) {
                throw new ServiceError("NOT_FOUND", {
                    message: `nothing here projects ${parameters.type}`,
                });
            }

            return new sync.Replica({ name, scope, tables: [], isRelayed: false, projectors });
        },
    });

    /** Stream the rows of a type a home's residents receive from a position, each decided for its recipient. */
    async *project(
        subscription: sync.Subscription,
        signal: AbortSignal,
        drain?: AbortSignal,
    ): AsyncGenerator<sync.Page> {
        // require the rows this installation keeps, and each recipient to live in the following home
        const parameters = ProjectionParameters.parse(subscription.parameters);
        const { installation } = this.server;
        if (installation === undefined || parameters.installation !== installation.id) {
            throw new ServiceError("NOT_FOUND", {
                message: `${parameters.installation} keeps the rows elsewhere`,
            });
        }
        const source = this.#projected(parameters);
        for (const recipient of parameters.recipients) {
            if (!(await installation.directory.isHome(recipient, subscription.below))) {
                throw new ServiceError("FORBIDDEN", {
                    message: `${recipient} lives in no home ${subscription.below}`,
                });
            }
        }

        // restart from a snapshot once the recipients changed
        const { scope, previous, origin } = subscription;
        const after = previous === undefined ? subscription.after : undefined;

        // stream the rows each recipient receives
        const audience = await RecipientAudience.of(
            this.server,
            scope,
            parameters.to,
            parameters.recipients,
        );
        const queries: Record<string, sync.Query> = {
            [source.name]: {
                table: source.table,
                scopes: [scope],
                where: { [parameters.to]: { in: parameters.recipients } },
            },
        };
        yield* marking(
            this.server.feed.subscribe(queries, after, signal, {
                audience,
                ...(drain === undefined ? {} : { drain }),
                ...(origin === undefined ? {} : { origin }),
            }),
            [scope],
        );
    }

    /** Find the served type a projection names, refusing one without its recipient field. */
    #projected(parameters: ProjectionParameters): ObjectType {
        const source = this.server.durable.find(
            (object) =>
                object.name === parameters.type &&
                object.policy.definition.packageId === parameters.packageId,
        );
        if (source?.fields[parameters.to]?.type !== "subject") {
            throw new ServiceError("NOT_FOUND", {
                message: `no ${parameters.type} received through ${parameters.to}`,
            });
        }

        return source;
    }

    /** List the subscriptions a home follows: one per projected type and source installation its residents' addresses name. */
    async projectionSubscriptions(home: string): Promise<sync.Subscription[]> {
        // project nowhere without a projected type
        const projecting = this.server.objects.flatMap((object) =>
            object.projected === undefined ? [] : [object.projected],
        );
        const { residence } = this.server;
        if (residence === undefined || projecting.length === 0) {
            return [];
        }

        // read the addresses of the users living here, grouped by source and installation
        const user = residence.user.table[TABLE];
        const address = residence.address.table[TABLE];
        const rows = await this.server.database
            .select({
                user: user.column("id"),
                source: address.column("source"),
                installation: address.column("installation"),
            })
            .from(residence.address.table)
            .innerJoin(residence.user.table, eq(user.column("id"), address.column("scope")))
            .where(eq(user.column("home"), home))
            .orderBy(address.column("source"), address.column("installation"), user.column("id"));
        const sources = Map.groupBy(
            rows.map((row) => ({
                source: schema.string().parse(row.source),
                installation: schema.identifier("installation").parse(row.installation),
                recipient: Subject.key(
                    principal.user.reference(Scope.universe.id, schema.string().parse(row.user)),
                ),
            })),
            (row) => JSON.stringify([row.source, row.installation]),
        );

        // follow each projected type from each source for its recipients
        return projecting.flatMap((options) => {
            const source = projected.sourceOf(options);

            return [...sources.values()].map((entries) => {
                const { source: scope, installation } = aligned(entries, 0);

                return this.projectionShape.subscription({
                    name: home,
                    scope,
                    below: home,
                    parameters: {
                        installation,
                        packageId: source.policy.definition.packageId,
                        type: source.name,
                        to: options.to,
                        recipients: entries.map((entry) => entry.recipient),
                    },
                });
            });
        });
    }

    /** Build the controller recording in each recipient's user scope the installation keeping rows of a projected type for them, absent without one. */
    addresses(): Controller | undefined {
        // find the served types some type projects, with their recipient fields
        const { installation } = this.server;
        const sources = this.server.durable.flatMap((object) =>
            object.projections.flatMap((projecting) =>
                projecting.projected === undefined
                    ? []
                    : [{ table: object.table, to: projecting.projected.to }],
            ),
        );
        if (installation === undefined || sources.length === 0) {
            return undefined;
        }

        // key each recipient, and record it once
        return {
            name: "address",
            watches: sources.map(({ table }) => table),
            keys: (change) => {
                // read the recipient the changed row names
                const row = Change.image(change);
                const source = sources.find(({ table }) => table === change.table);
                const recipient = source === undefined ? undefined : row[source.to];

                return typeof recipient === "string" ? [recipient] : [];
            },
            list: async () => {
                const keys = await Promise.all(
                    sources.map(async ({ table, to }) => {
                        const rows = await this.server.database
                            .selectDistinct({ recipient: table[TABLE].column(to) })
                            .from(table);

                        return rows.flatMap(({ recipient }) =>
                            typeof recipient === "string" ? [recipient] : [],
                        );
                    }),
                );

                return [...new Set(keys.flat())];
            },
            reconcile: async (recipient) => {
                // settle a recipient without a user, whose key a change of a row naming them enqueues again
                try {
                    await installation.directory.address(recipient);
                } catch (error) {
                    if (!(isServiceError(error) && error.code === "NOT_FOUND")) {
                        throw error;
                    }
                }

                return undefined;
            },
        };
    }

    /** Follow a scope's durable objects for the caller. */
    async *#durable(
        subscription: sync.Subscription,
        context: ServiceContext,
    ): AsyncGenerator<sync.Page> {
        // open the audience
        const { scope, after, previous, refresh, origin } = subscription;
        const { queries } = QueriesParameters.parse(subscription.parameters);
        const audience = await ObjectAudience.of(this.server, scope, context);

        // follow the queries from the queries the copy reflects
        const { chain } = audience;
        const compiled = ObjectType.queries(this.server.durable, queries, chain);
        const earlier =
            previous === undefined ? undefined : QueriesParameters.parse(previous).queries;
        const options = followOptions(audience, context, {
            previous: earlier && ObjectType.queries(this.server.durable, earlier, chain),
            refresh,
            origin,
        });
        const follow = (feed: sync.Feed) =>
            marking(feed.subscribe(compiled, after, context.request.signal, options), chain);

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
        yield* pages();
    }

    /** Follow a scope's ephemeral objects for a client. */
    async *#ephemeral(
        subscription: sync.Subscription,
        context: ServiceContext,
    ): AsyncGenerator<sync.Page> {
        // track the client's rows
        const { scope, after, previous, refresh, origin } = subscription;
        const { queries, client } = EphemeralParameters.parse(subscription.parameters);
        const store = this.server.store();
        const release = store.track(EphemeralStorage.clientKey(context, client));
        const compiled = ObjectType.queries(store.objects, queries, [scope]);
        const earlier =
            previous === undefined ? undefined : EphemeralParameters.parse(previous).queries;
        const reflected = earlier && ObjectType.queries(store.objects, earlier, [scope]);
        try {
            // follow until access changes, or to a completed page once the caller lapses
            yield* this.#decided(scope, context, "ephemeral", (audience, signal, isFirst) => {
                const options = followOptions(audience, context, {
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
            });
        } finally {
            release();
        }
    }

    /** Follow a scope's external objects for the caller, decided in memory until access changes. */
    async *#external(
        subscription: sync.Subscription,
        context: ServiceContext,
    ): AsyncGenerator<sync.Page> {
        // open the scope's files
        const { scope, after, previous, refresh, origin } = subscription;
        const { queries } = QueriesParameters.parse(subscription.parameters);
        const external = present(this.server.external, "the external files");
        const feed = new sync.Feed(
            await external.open(scope),
            external.objects.map((object) => object.table),
        );
        const compiled = ObjectType.queries(external.objects, queries, [scope]);
        const earlier =
            previous === undefined ? undefined : QueriesParameters.parse(previous).queries;
        const reflected = earlier && ObjectType.queries(external.objects, earlier, [scope]);

        // follow until access changes, or to a completed page once the caller lapses
        yield* this.#decided(scope, context, "external", (audience, signal, isFirst) => {
            const options = followOptions(audience, context, {
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
        context: ServiceContext,
    ): AsyncGenerator<sync.Page> {
        // follow until access changes
        const { scope, after, refresh, origin } = subscription;
        yield* this.#decided(scope, context, "durable", (audience, signal, isFirst) => {
            // copy the chain's scopes and roles, and the relationships the caller's subjects hold
            const { chain } = audience;
            const subjects = audience.authorization.access.authorities.flatMap(
                (authority) => authority.subjects,
            );
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
            const options = followOptions(sync.EVERYONE, context, {
                previous: undefined,
                refresh,
                origin,
            });
            const pages = this.server.feed.subscribe(
                queries,
                isFirst ? after : undefined,
                signal,
                options,
            );

            return marking(pages, chain);
        });
    }

    /** Follow pages decided for the caller, from a snapshot again whenever the caller's access along the scope chain changes. */
    async *#decided(
        scope: string,
        context: ServiceContext,
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
            const since = (await this.server.database.log.position()).sequence;
            const audience = await ObjectAudience.of(this.server, scope, context, { storage });
            const watching = this.#watchAccess(audience.chain, since, revised, signal);

            // follow until access changes, and from a snapshot after
            yield* follow(audience, signal, isFirst);
            revised.abort();
            await watching;
            isFirst = false;
        }
    }

    /** Revise a decided stream once the access rows of its chain change, failing with the log. */
    async #watchAccess(
        chain: readonly string[],
        since: number,
        revised: AbortController,
        signal: AbortSignal,
    ): Promise<void> {
        const tables = this.server.authorizer.watch(chain).map((watch) => watch.table);
        try {
            // abort the stream on the first page with changes
            const pages = this.server.database.log.follow(
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
        context: ServiceContext,
    ): Promise<void> {
        // require a small event and a reader
        const store = this.server.store();
        if (new TextEncoder().encode(JSON.stringify(event)).length > BROADCAST_BYTES) {
            throw new ServiceError("BAD_REQUEST", {
                message: `a broadcast has at most ${BROADCAST_BYTES} bytes`,
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

    /** The shapes this server serves and follows, by name. */
    get shapes(): ReadonlyMap<string, sync.Shape> {
        return new Map(
            [
                ...this.server.authorizer.shapes,
                this.queriesShape,
                this.ephemeralShape,
                this.externalShape,
                this.accessShape,
                this.projectionShape,
                ...this.server.shapes,
            ].map((shape) => [shape.name, shape]),
        );
    }

    /** Build the copy a request of a served shape asks for. */
    replicaOf(request: sync.Subscription): sync.Replica {
        return this.#shape(request).replica(request);
    }

    /** Find the served shape a request names, refusing any other. */
    #shape(request: Pick<sync.Subscription, "shape">): sync.Shape {
        const shape = this.shapes.get(request.shape);
        if (shape === undefined) {
            throw new ServiceError("NOT_FOUND", { message: `no shape ${request.shape}` });
        }

        return shape;
    }

    /** Name the copy a subscription keeps by its name and scope, which every subscription of the copy shares. */
    static #copy(subscription: Pick<sync.Subscription, "name" | "scope">): string {
        return `${subscription.name} ${subscription.scope}`;
    }

    /** Build the controller following each requested copy and dropping each kept copy no request names. */
    controller(subscriber: Subscriber): Controller {
        // list the requested copies and revise the listed ones
        const listed = new Map<string, CopyEntry>();
        const list = async (): Promise<readonly string[]> => {
            const copies = await this.#copies(subscriber);
            revise(listed, copies);

            return [...copies.keys()];
        };

        // reconcile each copy under its own key and lease, so no page lands after its drop
        return {
            name: "replica",
            mode: "follow",
            watches: this.#copyWatches(subscriber),
            concurrency: Infinity,
            list,
            reconcile: (key, { signal }) => this.#reconcileCopy(key, listed, list, signal),
        };
    }

    /** List the tables whose changes list the copies again. */
    #copyWatches(subscriber: Subscriber): Table[] {
        // watch the copied scope types and the residents' homes and addresses
        const { residence } = this.server;
        const scopes = this.server.copied
            .filter((object) => object.policy.definition.scope === true)
            .map((object) => object.table);

        return [
            ...new Set([
                Scope.table,
                ...scopes,
                ...(residence === undefined ? [] : [residence.user.table, residence.address.table]),
                ...(subscriber.watches ?? []),
            ]),
        ];
    }

    /** Follow each requested copy from its publisher, and drop each kept copy no request names, by copy key. */
    async #copies(subscriber: Subscriber): Promise<Map<string, CopyIntent & { digest: string }>> {
        // drop the kept copies no request names and follow the requested ones
        const entries = await subscriber.subscriptions();
        const named = new Set(entries.map((entry) => ObjectSource.#copy(entry.subscription)));
        const kept = await sync.Replica.subscriptions(this.server.database);
        const actions: CopyIntent[] = [
            ...kept
                .filter((copy) => !named.has(ObjectSource.#copy(copy)))
                .map((copy) => ({ action: "drop" as const, subscription: copy })),
            ...entries.map(({ subscription, publisher }) => ({
                action: "follow" as const,
                subscription,
                publisher,
            })),
        ];

        // digest each action with its subscription
        return new Map(
            await Promise.all(
                actions.map(async (entry): Promise<[string, CopyIntent & { digest: string }]> => [
                    ObjectSource.#copy(entry.subscription),
                    {
                        ...entry,
                        digest: await Digest.json({
                            action: entry.action,
                            subscription: entry.subscription,
                        }),
                    },
                ]),
            ),
        );
    }

    /** Follow or drop one listed copy until the controller stops it. */
    async #reconcileCopy(
        key: string,
        listed: ReadonlyMap<string, CopyEntry>,
        list: () => Promise<readonly string[]>,
        signal: AbortSignal,
    ): Promise<undefined> {
        while (!signal.aborted) {
            // wait for the loop to stop a copy no longer listed
            const copy = listed.get(key);
            if (copy === undefined) {
                await until(signal);

                return undefined;
            }

            // follow a requested copy from the position it reached until its request changes
            const revised = AbortSignal.any([signal, copy.revised.signal]);
            if (copy.action === "follow") {
                await this.#followCopy(key, copy, listed, list, revised);
            }
            // drop a kept copy no request names and wait for a request naming it again
            else {
                await this.replicaOf(copy.subscription).drop(this.server.database);
                await until(revised);
            }
        }

        return undefined;
    }

    /** Follow a requested copy from its publisher, failing once its source refuses it unless its request changed. */
    async #followCopy(
        key: string,
        copy: CopyEntry & { readonly action: "follow" },
        listed: ReadonlyMap<string, CopyEntry>,
        list: () => Promise<readonly string[]>,
        revised: AbortSignal,
    ): Promise<void> {
        try {
            const { subscription, publisher } = copy;
            await this.replicaOf(subscription).follow(
                this.server.database,
                (from, stream) => publisher.stream({ ...subscription, ...from }, stream),
                revised,
                { subscription },
            );
        } catch (error) {
            // list again once a source refuses a follow, failing it unless its request changed
            await list();
            if (listed.get(key) === copy) {
                throw error;
            }
        }
    }

    /** List the rows of the universe a scope reads: each copied type's rows, nested types within their scopes. */
    universeRows(source?: Pick<ObjectServer, "objects">): UniverseParameters["rows"] {
        // leave inherited rows to the chains, and keep the types of one source in this process, or of none
        const sourceOf = (object: ObjectType) =>
            this.server.sources.find((each) => each.objects.some((type) => type.same(object)));
        const copied = this.server.copied.filter(
            (object) => object.inherited === undefined && sourceOf(object) === source,
        );
        const isUnscoped = (object: ObjectType) =>
            !object.scopes.some((scope) => copied.some((type) => type.same(scope)));

        // copy a type living in no copied scope from every scope its rows live in
        const listed = copied.filter(isUnscoped);
        const rows: UniverseParameters["rows"][number][] = listed.map((object) =>
            object.scope === Scope.universe.id
                ? { type: object.typeReference, where: {} }
                : { type: object.typeReference, where: {}, isEverywhere: true },
        );

        // add the types living in the scopes of listed types, at any depth
        for (let isGrowing = true; isGrowing;) {
            isGrowing = false;
            for (const object of copied.filter((candidate) => !listed.includes(candidate))) {
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

        return rows;
    }

    /** List the subscriptions a placed workload keeps: the universe's rows and one copy of its scopes' chains. */
    async workloadSubscriptions(placement: string): Promise<sync.Subscription[]> {
        // read the copied scopes the served objects live in while the universe's copy includes them
        const living = this.server.copied.filter((scope) =>
            this.server.durable.some((object) => object.scopes.some((type) => type.same(scope))),
        );
        const scopes = await Promise.all(
            living.map(async (scope) => {
                const table = scope.table[TABLE];
                const rows = await this.server.database
                    .select({ id: table.column("id"), scope: table.column("scope") })
                    .from(scope.table)
                    .where(sync.Replica.includes(placement, scope.table));

                return rows.map((row): ObjectReference => ({
                    ...scope.typeReference,
                    scope: schema.string().parse(row.scope),
                    id: schema.string().parse(row.id),
                }));
            }),
        );

        // copy the scopes' chains in one copy, after the rows of the universe
        const universe = this.universeSubscription(placement);
        const chains = await this.server.authorizer.chainVia(
            this.server.database,
            placement,
            scopes.flat(),
        );

        return [
            ...(universe === undefined ? [] : [universe]),
            ...(chains === undefined ? [] : [chains]),
        ];
    }

    /** List the subscriptions of the copies a database keeps for a scope: its chain, and the rows of the universe it reads. */
    async subscriptions(
        below: string,
        options: { readonly isHome: boolean },
    ): Promise<sync.Subscription[]> {
        // copy the chain and the rows of the universe the scope reads
        const chain = await this.server.authorizer.chain(this.server.database, below, options);
        const universe = this.universeSubscription(below);

        return universe === undefined ? chain : [...chain, universe];
    }

    /** Build the subscription of the copy of the universe's rows a scope or cell reads, absent when the server copies no type living there. */
    universeSubscription(below: string): sync.Subscription | undefined {
        // request nothing where no global object type is served
        const rows = this.universeRows();
        if (rows.length === 0) {
            return undefined;
        }

        // copy the universe's rows the scope or cell may read, under its own name
        return this.server.authorizer.universeShape.subscription({
            name: below,
            scope: Scope.universe.id,
            below,
            parameters: { rows },
        });
    }

    /** List the copies a database keeps of the rows its sources in this process keep, one per source. */
    sourceSubscriptions(below: string): SubscriberEntry[] {
        return this.server.sources.flatMap((source) => {
            // request nothing of a source keeping no copied type
            const rows = this.universeRows(source);
            const [first] = rows;
            if (first === undefined) {
                return [];
            }

            // copy the source's rows under a name of its package
            const subscription = this.server.authorizer.universeShape.subscription({
                name: `${below}/${first.type.packageId}`,
                scope: Scope.universe.id,
                below,
                parameters: { rows },
            });

            return [{ subscription, publisher: source.uplink }];
        });
    }

    /** Stream a copy's pages to a database in this process, never back to the source of its rows. */
    async *publish(
        subscription: sync.Subscription,
        signal: AbortSignal,
    ): AsyncGenerator<sync.Page> {
        // refuse a subscriber whose rows the copy would send back
        const copy = this.replicaOf(subscription);
        const { after, origin } = subscription;
        if (origin !== undefined) {
            await copy.requireAcyclic(this.server.database, origin);
        }

        // stream the copy, sending no bare positions of the subscriber's own writes
        yield* this.server.feed.subscribe(
            copy.queries,
            after,
            signal,
            origin === undefined ? {} : { origin },
        );
    }

    /** Stream a copy's pages to a database below. */
    async *replicate(
        request: sync.Subscription,
        after: LogPosition | undefined,
        follower: ReplicaFollower,
        signal: AbortSignal,
        drain?: AbortSignal,
    ): AsyncGenerator<sync.Page> {
        // require the copied scope to contain the follower's for a chain
        const shape = this.#shape(request);
        if (shape.audience === "contained") {
            await this.#requireContaining(request, follower);
        }

        // admit the follower to a chain by containment and to the universe's rows as their reader
        const audience = await this.#replicaAudience(shape, request, follower);

        // relay a copied scope only once its copy has a position, and never back to its source
        const copy = shape.replica(request);
        const { previous, origin } = request;
        await sync.Replica.requireRelayable(this.server.database, request.name, request.scope);
        if (origin !== undefined) {
            await copy.requireAcyclic(this.server.database, origin);
        }

        // stream the copy from the parameters it reflects, ending at a completed page once drained
        const reflected =
            previous === undefined
                ? undefined
                : shape.replica({ ...request, parameters: previous }).queries;
        yield* this.server.feed.subscribe(copy.queries, after, signal, {
            audience,
            ...(reflected === undefined ? {} : { previous: reflected }),
            ...(drain === undefined ? {} : { drain }),
            ...(origin === undefined ? {} : { origin }),
        });
    }

    /** Require a chain copy's scope to contain the follower's scope, through its parent when kept elsewhere. */
    async #requireContaining(request: sync.Subscription, follower: ReplicaFollower): Promise<void> {
        // require a follower outside the chains, such as a placed workload, to copy them via scopes inside them
        const { via, between } = ChainParameters.parse(request.parameters);
        if (via !== undefined) {
            await this.#requireVia(request.scope, via, between ?? []);

            return;
        }

        // read the scopes containing the follower's
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

    /** Require the chains of the scopes a follower copies via to hold the copied scope and each scope between, reading them at once. */
    async #requireVia(
        scope: string,
        via: readonly string[],
        between: readonly string[],
    ): Promise<void> {
        // require each replicated scope's chain to hold the copied scope
        const chains = await Scope.chains(Snapshot.live(this.server.database), via);
        const scopes = new Set<string>();
        for (const [relay, links] of chains) {
            const ids = links.map((link) => link.object.id);
            if (!ids.includes(scope)) {
                throw new ServiceError("NOT_FOUND", {
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
            throw new ServiceError("NOT_FOUND", {
                message: `${outside} contains none of the replicated scopes`,
            });
        }
    }

    /** Require a reader, a principal or a caller, to hold the `replicate` permission on the scope it copies a chain via. */
    async requireReplicateVia(via: string, reader: Subject | ServiceContext): Promise<void> {
        // require a scope type keeping copies of its chain
        const { database } = this.server;
        const scope = await Scope.object(Snapshot.live(database), via);
        const policy = this.server.authorizer.policy(scope);
        if (!Object.hasOwn(policy.definition.permissions, REPLICATE)) {
            throw new ServiceError("FORBIDDEN", {
                message: `no ${scope.type} keeps copies of its chain`,
            });
        }

        // decide for the principal standing for the follower, or for the caller
        const authorization =
            "packageId" in reader
                ? await this.server.authorizeSubject(database, via, reader)
                : await this.server.authorize(database, via, reader);
        await authorization.require(policy.permission(REPLICATE), scope);
    }

    /** Open the audience a relayed copy is decided for. */
    async #replicaAudience(
        shape: sync.Shape,
        request: sync.Subscription,
        follower: ReplicaFollower,
    ): Promise<sync.Audience> {
        // read the scopes a follower outside the chains copies them via
        const isContained = shape.audience === "contained";
        const { via } = isContained ? ChainParameters.parse(request.parameters) : {};
        const copying = via === undefined ? {} : { via };

        // decide a principal's chain by containment in the universe as a principal inside its parent's chain
        if ("subject" in follower && isContained) {
            return ObjectAudience.of(this.server, Scope.universe.id, follower.subject, {
                isContained,
                within: await this.#within(follower.parent ?? request.below),
                ...copying,
            });
        }
        // decide a principal's other rows in the copied scope, as a principal inside its parent's chain
        else if ("subject" in follower) {
            const { parent } = follower;
            const within = parent === undefined ? [] : await this.#within(parent);

            return ObjectAudience.of(this.server, request.scope, follower.subject, { within });
        }
        // decide a caller's chain by containment, with its fields from the scope it acts in
        else if (isContained) {
            return ObjectAudience.of(this.server, request.below, follower.context, {
                isContained,
                ...copying,
            });
        }
        // decide a caller in the scope a relayed copy is kept for, or in the copied scope at its home
        else {
            const isRelayed = await sync.Replica.isCopied(this.server.database, request.scope);
            const scope = isRelayed ? request.below : request.scope;

            return ObjectAudience.of(this.server, scope, follower.context);
        }
    }

    /** List the scopes a principal lives inside: a scope and the scopes enclosing it below the universe. */
    async #within(scope: string): Promise<ObjectReference[]> {
        const links = await Scope.chain(Snapshot.live(this.server.database), scope);

        return links.flatMap((link) => (link.object.id === Scope.universe.id ? [] : [link.object]));
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
                    stop ??= this.server.store().tracker.listen(scope, (event) => {
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

    /** Keep the events of objects the caller may list, remembering each object's decision. */
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

            // keep a readable object's event
            if (isRead) {
                broadcasts.push({ topic, event });
            }
        }

        return broadcasts;
    }
}

/** What the replica controller does with a copy: follow it for its request from its publisher, or drop it once no request names it. */
type CopyIntent =
    | {
          /** A request follows the copy. */
          readonly action: "follow";
          /** The request following the copy. */
          readonly subscription: sync.Subscription;
          /** The publisher streaming the copy's pages. */
          readonly publisher: sync.Publisher;
      }
    | {
          /** The kept copy drops for want of a request. */
          readonly action: "drop";
          /** The subscription the kept copy recorded. */
          readonly subscription: sync.Subscription;
      };

/** A copy the replica controller lists, with the digest each listing compares. */
type CopyEntry = CopyIntent & {
    /** The digest of the action and subscription. */
    readonly digest: string;
    /** Aborts once a listing changes the action or the subscription. */
    readonly revised: AbortController;
};

/** Who a relayed copy is decided for: a principal where its rows live, or the calling principal. */
export type ReplicaFollower =
    | {
          /** The principal the copy is decided for. */
          readonly subject: Subject;
          /** The scope containing the follower's scope, when this database keeps no copy of it. */
          readonly parent?: string;
      }
    | { readonly context: ServiceContext };

/** Revise each listed copy whose action changed or left, and list the new ones. */
function revise(
    listed: Map<string, CopyEntry>,
    copies: ReadonlyMap<string, CopyIntent & { digest: string }>,
): void {
    // abort and forget each listed copy whose digest changed or left
    for (const [key, copy] of listed) {
        if (copies.get(key)?.digest !== copy.digest) {
            copy.revised.abort();
            listed.delete(key);
        }
    }

    // list the new copies
    for (const [key, copy] of copies) {
        if (!listed.has(key)) {
            listed.set(key, { ...copy, revised: new AbortController() });
        }
    }
}

/** Build the feed options of a copy followed for a caller, ending at a completed page once it lapses. */
function followOptions(
    audience: sync.Audience,
    context: ServiceContext,
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

/** List the tables a query reads: its rows', its includes' and those its conditions and computed values follow. */
function tablesOf(query: sync.Query): Table[] {
    return new sync.Node("", query, query.scopes).nodes().map((node) => node.table);
}

/** Mark each page with the scopes it reads and each completed page with its event identifier. */
async function* marking(
    pages: AsyncIterable<sync.Page>,
    chain: readonly string[],
): AsyncGenerator<sync.Page> {
    for await (const page of pages) {
        yield marked({ ...page, scopes: [...chain] });
    }
}

/** Mark a completed page with its position as its event identifier, which a resumed stream continues after. */
function marked(page: sync.Page): sync.Page {
    return page.complete
        ? withEventMeta(page, { id: `${page.position.epoch}/${page.position.sequence}` })
        : page;
}
