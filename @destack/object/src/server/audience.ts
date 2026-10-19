import {
    ObjectReference,
    type Audience,
    Watch,
    replica,
    replicaPosition,
    Scope,
    Subject,
    SyncError,
} from "@destack/sync";
import {
    AccessContext,
    accessRelationship,
    accessRole,
    accessRolePermission,
    decisionTables,
    earliest,
    type Permission,
} from "@destack/access";
import { journal } from "../stack/db.ts";
import {
    sql,
    type Row,
    type SQL,
    type Table,
    Change,
    type LogPosition,
    Snapshot,
    Transaction,
} from "@destack/db";
import { canonicalize } from "@destack/schema";
import { type ObjectStorage, ObjectType } from "../object/object.ts";
import { ObjectTypeIndex } from "../query/listing.ts";
import type { Authorization } from "./authorization.ts";
import type { ObjectStore } from "./store.ts";

/** The columns holding a row's parent. */
const PARENT_COLUMNS = ["parentPackageId", "parentType", "parentId"] as const;

/** The caller's access to one stream of a scope's objects. */
export class ObjectAudience implements Audience {
    /** The store serving the objects. */
    readonly #store: ObjectStore;
    /** The scope the stream follows. */
    readonly #scope: string;
    /** The caller following the stream. */
    readonly #caller: AccessContext;
    /** The types whose personal values the stream showed, journaled once each. */
    readonly #revealed = new Set<ObjectType>();
    /** Authorize the subscriber again, as it resolves access once access changes. */
    readonly #reauthorize: () => Promise<Authorization>;
    /** The object types the store knows, by table. */
    readonly #objects: ObjectTypeIndex;
    /** The caller's access, resolved again once access changes. */
    #authorization: Authorization;
    /** The key of the caller's access inputs. */
    #key: string | undefined;
    /** The earliest time the caller's access changes on its own. */
    #until: number | undefined;
    /** Where the followed objects live. */
    readonly #storage: ObjectStorage;
    /** The durable log position access was decided at, for ephemeral and external objects. */
    #decided: LogPosition | undefined;
    /** Whether the subscriber has every row by containing the scope, deciding guarded fields. */
    readonly #isContained: boolean;
    /** The scopes below the followed one whose access decided rows. */
    readonly #below = new Set<string>();
    /** The scopes an outside follower replicates via, with the rows deciding its replication. */
    readonly #via: readonly Relay[];
    /** The deciding access rows of the chain and lower scopes, none for ephemeral or external. */
    watches: readonly Watch[];

    /** Serve a caller of a scope with resolved access. */
    private constructor(
        store: ObjectStore,
        scope: string,
        caller: AccessContext,
        reauthorize: () => Promise<Authorization>,
        authorization: Authorization,
        storage: ObjectStorage,
        decided: LogPosition | undefined,
        isContained: boolean,
        via: readonly Relay[],
    ) {
        // set the inputs and watch access
        this.#store = store;
        this.#scope = scope;
        this.#caller = caller;
        this.#reauthorize = reauthorize;
        this.#objects = new ObjectTypeIndex(
            store.known.filter((known) => known instanceof ObjectType),
        );
        this.#authorization = authorization;
        this.#until = authorization.access.until;
        this.#storage = storage;
        this.#decided = decided;
        this.#isContained = isContained;
        this.#via = via;
        this.watches = this.#watched();
    }

    /** Resolve a reader's access to a scope's objects: a remote caller or a replica's principal. */
    static async of(
        store: ObjectStore,
        scope: string,
        reader: AccessContext,
        options: {
            readonly storage?: ObjectStorage;
            readonly isContained?: boolean;
            readonly within?: readonly ObjectReference[];
            readonly via?: readonly string[];
        } = {},
    ): Promise<ObjectAudience> {
        // decide ephemeral and external objects at the durable position
        const storage = options.storage ?? "durable";
        const isContained = options.isContained ?? false;
        const decided = storage === "durable" ? undefined : await store.database.log.position();

        // read the scopes a follower replicates chains via
        const via =
            options.via === undefined
                ? []
                : await ObjectAudience.#relays(store, options.via, reader);

        // authorize the reader as a principal, a contained caller or a remote caller this store serves
        const reauthorize = async () => {
            const authorization = await (!AccessContext.isRemote(reader)
                ? store.authorize(Transaction.reading(store.database), scope, reader, {
                      within: options.within ?? [],
                  })
                : isContained
                  ? store.authorize(Transaction.reading(store.database), scope, reader)
                  : store.authorizeHere(Transaction.reading(store.database), scope, reader));

            // require the follower to replicate the scopes it replicates chains via
            await Promise.all(via.map((relay) => relay.require()));

            return authorization;
        };

        return new ObjectAudience(
            store,
            scope,
            reader,
            reauthorize,
            await reauthorize(),
            storage,
            decided,
            isContained,
            via,
        );
    }

    /** Read the rows deciding whether a follower may replicate its via scopes. */
    static async #relays(
        store: ObjectStore,
        via: readonly string[],
        reader: AccessContext,
    ): Promise<Relay[]> {
        // read the scopes' chains at once
        const chains = await Scope.chains(Snapshot.live(store.database), via);

        return via.map((id) => {
            // read the scope's object, refusing an unknown scope
            const links = chains.get(id) ?? [];
            const scope = links[0]?.object;
            if (scope === undefined) {
                throw new SyncError("NOT_FOUND", `unknown scope: ${id}`);
            }

            // watch the scope's own row where this database maps its type, and the access rows of its chain
            const mapping = store.authorizer.mapping(scope);
            const own: Watch[] =
                mapping === undefined
                    ? []
                    : [Watch.of(mapping.table, [scope.scope], { [mapping.id]: id })];
            const chain = links.map((link) => link.object.id);

            return {
                scope: id,
                watches: [...own, ...store.authorizer.watch(chain)],
                require: () => store.source.requireReplicateVia(id, reader),
            };
        });
    }

    /** The scope's own object, absent for a scope the database does not know. */
    get scope(): ObjectReference | undefined {
        return this.#authorization.access.scopes[0];
    }

    /** Match the rows of a table the caller may list, under a query's own permission too. */
    where(table: Table, permission?: string): SQL | "memory" {
        // leave the journal and the replicas' records to their queries
        const listed = this.#listed(table, permission);
        if (listed === "query") {
            return sql`true`;
        }
        // match no rows of a table no type here lists
        else if (listed === "none") {
            return sql`false`;
        }
        // decide in memory the rows at their home in other scopes than the followed one, each in its own chain
        else if (
            listed.object.storage === "durable" &&
            listed.object.inherited === undefined &&
            !this.#store.database.replicates(listed.object.table) &&
            !this.#authorization.isHomeOf(listed.object)
        ) {
            return "memory";
        }

        // match the listed rows the caller holds the query's own permission on
        return this.#authorization.where(listed.object, listed.listing, listed.own);
    }

    /** The name of the caller's access inputs and resolved access, apart from time. */
    get key(): string {
        const access = this.#authorization.access;
        this.#key ??= canonicalize({
            scope: this.#scope,
            context: { ...access.context, request: null, now: null },
            isContained: this.#isContained,
            via: this.#via.map((relay) => relay.scope),
            roles: access.grantedRoles,
            subjects: access.authority.subjects.map((subject) => Subject.key(subject)).toSorted(),
            chain: access.chainKey,
            decided: this.#decided?.sequence ?? null,
        });

        return this.#key;
    }

    /** Decide which rows the caller may list, under a query's own permission too. */
    async visible(
        table: Table,
        rows: readonly Row[],
        position: LogPosition,
        permission?: string,
    ): Promise<ReadonlySet<number>> {
        // show every row left to its query
        const listed = this.#listed(table, permission);
        if (listed === "query") {
            return new Set(rows.keys());
        }
        // show no rows of a table no type here lists
        else if (listed === "none") {
            return new Set();
        }
        // filter the listed rows the caller holds the query's own permission on
        const filtered = await this.#authorization.filter(listed.object, listed.listing, rows, {
            scope: this.#scope,
            reader: this.#reader(position),
            ...(listed.own === undefined ? {} : { required: listed.own }),
        });
        this.#until = earliest([this.#until, filtered.until]);

        // watch the access of the scopes below that decided rows
        const known = this.#below.size;
        for (const scope of filtered.below) {
            this.#below.add(scope);
        }
        if (this.#below.size > known) {
            this.watches = this.#watched();
        }

        // answer the positions of the rows left
        const shown = new Set<Row>(filtered.rows);

        return new Set(rows.flatMap((row, index) => (shown.has(row) ? [index] : [])));
    }

    /** List an object type's guarded fields. */
    concealable(table: Table): readonly string[] {
        return this.#objects.concealable(table);
    }

    /** List the guarded fields the caller may not read on each row. */
    async conceals(
        table: Table,
        rows: readonly Row[],
        position: LogPosition,
    ): Promise<readonly (readonly string[])[]> {
        // conceal nothing of the rows left to their query, or of another table's rows a follower has by containment
        const listed = this.#isContained ? this.#objects.get(table) : this.#listed(table);
        if (listed === "query" || listed === "none" || listed === undefined) {
            return rows.map(() => []);
        }
        const object = "object" in listed ? listed.object : listed;

        // conceal unreadable fields
        const concealed = await this.#authorization.concealed(object, rows, this.#reader(position));
        this.#until = earliest([this.#until, concealed.until]);

        // journal the first personal values a remote caller's stream of the type shows
        const isFirst = !this.#revealed.has(object) && AccessContext.isRemote(this.#caller);
        if (isFirst && object.reveals(rows, concealed.hidden)) {
            this.#revealed.add(object);
            await this.#store.recordUnmask({
                scope: this.#scope,
                caller: this.#caller,
                object,
                operation: "sync",
            });
        }

        return concealed.hidden;
    }

    /** Read the earliest time the caller's access changes on its own. */
    async until(): Promise<number | undefined> {
        return this.#until;
    }

    /** Resolve the caller's access again. */
    async refresh(): Promise<void> {
        await this.#authorize();
    }

    /** Report whether a change may decide the visibility of rows beyond its own. */
    async decides(change: Change): Promise<boolean> {
        // require the follower to replicate its via scopes again after a change their decisions read
        const via = this.#via.filter((relay) => Watch.matches(relay.watches, change));
        if (via.length > 0 && !Watch.matches(this.#deciding(), change)) {
            await Promise.all(via.map((relay) => relay.require()));

            return false;
        }

        // leave roles the caller has no binding of, and another identity's relationship to its own row
        if (
            ((change.table === accessRole || change.table === accessRolePermission) &&
                !this.#hasRole(change)) ||
            (change.table === accessRelationship && !this.#isConcerned(change))
        ) {
            return false;
        }

        // decide again after an access change, or once a row moves under another parent
        const object = this.#objects.get(change.table);

        return (
            decisionTables.includes(change.table) ||
            (object?.parent !== undefined &&
                change.operation === "update" &&
                PARENT_COLUMNS.some((column) => change.before[column] !== change.after[column]))
        );
    }

    /** Decide whether a role or role permission change is of a role the caller has. */
    #hasRole(change: Change): boolean {
        const column = change.table === accessRole ? "id" : "roleId";
        const roles = new Set(
            [Change.before(change), Change.after(change)].map((row) => row?.[column]),
        );

        return this.#authorization.access.bindings.some((binding) => roles.has(binding.roleId));
    }

    /** Decide whether a relationship change may change the caller's access. */
    #isConcerned(change: Change): boolean {
        return [Change.before(change), Change.after(change)].some((row) => {
            // read the subject of each image
            if (row === null) {
                return false;
            }
            const relation = row["subjectRelation"];
            const subject = Subject.parse({
                packageId: row["subjectPackageId"],
                type: row["subjectType"],
                scope: row["subjectScope"],
                id: row["subjectId"],
                ...(relation === null ? {} : { relation }),
            });

            // take every subject set, as the same run may add the caller to it, and the caller's identities
            return (
                subject.relation !== undefined ||
                this.#authorization.access.authority.isMember(subject)
            );
        });
    }

    /** Resolve the caller's access again and reset derived state. */
    async #authorize(): Promise<void> {
        // authorize again and reset
        this.#authorization = await this.#reauthorize();
        this.#key = undefined;
        this.#until = this.#authorization.access.until;
        if (this.#storage !== "durable") {
            this.#decided = await this.#store.database.log.position();
        }
        this.watches = this.#watched();
    }

    /** List the rows whose changes decide again: listing access, and via replicating access. */
    #watched(): Watch[] {
        return [...this.#deciding(), ...this.#via.flatMap((relay) => relay.watches)];
    }

    /** List the access rows deciding what the caller lists. */
    #deciding(): Watch[] {
        return this.#storage === "durable"
            ? this.#store.authorizer.watch(
                  [...this.chain, ...this.#below],
                  this.#authorization.access.authority.subjects,
              )
            : [];
    }

    /** The caller's access, resolved again once access changes. */
    get authorization(): Authorization {
        return this.#authorization;
    }

    /** The scope and the scopes containing it, nearest first. */
    get chain(): string[] {
        return this.#authorization.chain;
    }

    /** Read the listing permission of a table's rows, and a query's own permission on them. */
    #listed(
        table: Table,
        permission?: string,
    ):
        | {
              readonly object: ObjectType;
              readonly listing: Permission;
              readonly own: Permission | undefined;
          }
        | "query"
        | "none" {
        // leave the journal and replica records to their queries, and contained rows to their follower
        if (
            this.#isContained ||
            table === journal ||
            table === replica ||
            table === replicaPosition
        ) {
            return "query";
        }

        // answer no rows of a table no served type lists, as a source has none to decide
        const object = this.#objects.get(table);
        if (object?.listing === undefined) {
            return "none";
        }

        // read the listing permission and the query's own
        const own = permission === undefined ? undefined : object.permission(permission);

        return { object, listing: object.listing, own };
    }

    /** Share one grant reader among the scope's subscribers at a position. */
    #reader(position: LogPosition) {
        return this.#store.reader(this.#authorization.access, this.#decided ?? position);
    }
}

/** A scope an outside follower replicates its chain via, replicated at every decision. */
interface Relay {
    /** The scope. */
    readonly scope: string;
    /** The scope's own row and the access rows of its chain, whose changes decide again. */
    readonly watches: readonly Watch[];
    /** Require the follower to replicate the scope. */
    readonly require: () => Promise<void>;
}
