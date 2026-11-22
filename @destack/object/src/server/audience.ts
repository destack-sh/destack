import {
    type Affected,
    ALL_ROWS,
    NO_ROWS,
    ObjectReference,
    type Audience,
    Watch,
    replica,
    replicaPosition,
    Scope,
    Subject,
    SyncError,
    ObjectTypeReference,
} from "@destack/sync";
import {
    AccessContext,
    accessRelationship,
    accessRole,
    accessRolePermission,
    decisionTables,
    earliest,
    type Permission,
    throughRelations,
} from "@destack/access";
import { journal } from "../stack/db.ts";
import {
    DatabaseError,
    sql,
    type Condition,
    type Row,
    type SQL,
    TABLE,
    type Table,
    Change,
    type LogPosition,
    Snapshot,
    Transaction,
} from "@destack/db";
import { canonicalize } from "@destack/schema";
import { type ObjectStorage, ObjectType } from "../object/object.ts";
import type { Authorization } from "./authorization.ts";
import type { ObjectStore } from "./store.ts";

/** The columns that store a row's parent. */
const PARENT_COLUMNS = ["parentPackageId", "parentType", "parentId"] as const;

/** What a guard conceals from a reader: whole rows from a replica, guarded fields from a call. */
type Concealment = "rows" | "fields";

/** The audience access decides: a reader's rows of a scope's objects. */
export class AccessAudience implements Audience {
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
    /** The earliest time the caller's access changes by itself. */
    #until: number | undefined;
    /** Where the followed objects live. */
    readonly #storage: ObjectStorage;
    /** The durable log position access was decided at, for ephemeral and external objects. */
    #decided: LogPosition | undefined;
    /** Whether the subscriber has every row by containing the scope, deciding guarded fields. */
    readonly #isContained: boolean;
    /** What the guard conceals from the reader: whole rows, or guarded fields. */
    readonly #concealment: Concealment;
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
        concealment: Concealment,
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
        this.#concealment = concealment;
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
    ): Promise<AccessAudience> {
        // decide ephemeral and external objects at the durable position
        const storage = options.storage ?? "durable";
        const isContained = options.isContained ?? false;
        const decided = storage === "durable" ? undefined : await store.database.log.position();

        // read the scopes a follower replicates chains via
        const via =
            options.via === undefined
                ? []
                : await AccessAudience.#relays(store, options.via, reader);

        // authorize the reader now as a principal, a contained caller or a remote caller this store serves
        const reauthorize = async () => {
            // decide at the current time
            const current: AccessContext = { ...reader, now: store.clock.now() };
            const authorization = await (!AccessContext.isRemote(current)
                ? store.authorize(Transaction.reading(store.database), scope, current, {
                      within: options.within ?? [],
                  })
                : isContained
                  ? store.authorize(Transaction.reading(store.database), scope, current)
                  : store.authorizeHere(Transaction.reading(store.database), scope, current));

            // require the follower to replicate the scopes it replicates chains via
            await Promise.all(via.map((relay) => relay.require()));

            return authorization;
        };

        return new AccessAudience(
            store,
            scope,
            reader,
            reauthorize,
            await reauthorize(),
            storage,
            decided,
            isContained,
            via,
            "rows",
        );
    }

    /** Decide rows by a fixed authorization, such as a call's over its own view. */
    static async bound(
        store: ObjectStore,
        scope: string,
        authorization: Authorization,
        storage: ObjectStorage,
    ): Promise<AccessAudience> {
        // decide ephemeral and external objects at the durable position
        const decided = storage === "durable" ? undefined : await store.database.log.position();

        return new AccessAudience(
            store,
            scope,
            authorization.context(),
            async () => authorization,
            authorization,
            storage,
            decided,
            false,
            [],
            "fields",
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
                throw SyncError.of("SCOPE_NOT_FOUND", `scope '${id}' not found`);
            }

            // watch the scope's row where this database maps its type, and the access rows of its chain
            const mapping = store.authorizer.mapping(scope);
            const local: Watch[] =
                mapping === undefined
                    ? []
                    : [Watch.of(mapping.table, [scope.scope], { [mapping.id]: id })];
            const chain = links.map((link) => link.object.id);

            return {
                scope: id,
                watches: [...local, ...store.authorizer.watch(chain)],
                require: () => store.source.requireReplicateVia(id, reader),
            };
        });
    }

    /** The scope's object, absent for a scope the database does not know. */
    get scope(): ObjectReference | undefined {
        return this.#authorization.access.scopes[0];
    }

    /** Match a superset of the rows of a table the caller may list, which `visible` decides. */
    async candidates(table: Table, permission?: string): Promise<SQL> {
        // leave the journal and the replicas' records to their queries, and match no rows of a table no type here lists
        const listed = this.#listed(table, permission);
        if (listed === "query") {
            return sql`true`;
        } else if (listed === "none") {
            return sql`false`;
        }
        // match every row at its home in another scope than the followed one, each decided in its chain
        else if (this.#isElsewhere(listed.object)) {
            return sql`true`;
        }

        // match the candidates of the query's permission
        return this.#authorization.candidates(listed.object, listed.listing, listed.required);
    }

    /** Match exactly the rows of a table the caller may list, for reads of related rows. */
    async readable(table: Table, permission?: string): Promise<SQL> {
        // leave the journal and the replicas' records to their queries, and match no rows of a table no type here lists
        const listed = this.#listed(table, permission);
        if (listed === "query") {
            return sql`true`;
        } else if (listed === "none") {
            return sql`false`;
        }
        // refuse related rows at their home in other scopes, which each scope's chain decides apart
        else if (this.#isElsewhere(listed.object)) {
            throw DatabaseError.of(
                "QUERY_INVALID",
                `table '${table[TABLE].name}' is invalid: relations cannot read its rows, as their scopes decide them apart`,
            );
        }

        // match the rows the caller holds the query's permission on
        return this.#authorization.readable(listed.object, listed.listing, listed.required);
    }

    /** Report whether a type's rows live at their home in other scopes than the followed one. */
    #isElsewhere(object: ObjectType): boolean {
        return (
            object.storage === "durable" &&
            object.propagated === undefined &&
            !this.#store.database.replicates(object.table) &&
            !this.#authorization.isHomeOf(object)
        );
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
            concealment: this.#concealment,
        });

        return this.#key;
    }

    /** Decide which rows the caller may list, under a query's permission too. */
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
        // filter the listed rows the caller has the query's permission on
        const filtered = await this.#authorization.filter(listed.object, listed.listing, rows, {
            scope: this.#scope,
            reader: this.#reader(position),
            ...(listed.required === undefined ? {} : { required: listed.required }),
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

    /** Read the earliest time the caller's access changes by itself. */
    async until(): Promise<number | undefined> {
        return this.#until;
    }

    /** Resolve the caller's access again. */
    async refresh(): Promise<void> {
        await this.#authorize();
    }

    /** Read the rows beyond the changed row whose visibility a change may change. */
    async affects(change: Change): Promise<Affected> {
        // require the follower to replicate its via scopes again after a change their decisions read
        const via = this.#via.filter((relay) => Watch.matches(relay.watches, change));
        if (via.length > 0 && !Watch.matches(this.#deciding(), change)) {
            await Promise.all(via.map((relay) => relay.require()));

            return NO_ROWS;
        }

        // leave roles the caller has no binding of, and another identity's relationship to its row
        if (
            ((change.table === accessRole || change.table === accessRolePermission) &&
                !this.#hasRole(change)) ||
            (change.table === accessRelationship && !this.#isConcerned(change))
        ) {
            return NO_ROWS;
        }

        // affect every row of another storage on an access change, as it decides as of one position
        if (this.#storage !== "durable" && decisionTables.includes(change.table)) {
            return ALL_ROWS;
        }
        // affect a relationship's object and what reads through it
        else if (change.table === accessRelationship) {
            return await this.#relatedRows(change);
        }
        // affect every row on another access change: roles, grants and scopes
        else if (decisionTables.includes(change.table)) {
            return ALL_ROWS;
        }

        // affect a row moved under another parent and what reads through it
        const object = this.#objects.get(change.table);
        const moved = Change.after(change);
        const isMoved =
            this.#storage === "durable" &&
            object?.parent !== undefined &&
            change.operation === "update" &&
            PARENT_COLUMNS.some((column) => change.before[column] !== change.after[column]);

        return isMoved && moved !== null
            ? {
                  kind: "rows",
                  read: (positions) =>
                      this.#throughRows(positions, [
                          {
                              table: change.table,
                              id: AccessAudience.#identifier(moved, "id"),
                          },
                      ]),
              }
            : NO_ROWS;
    }

    /** Read the rows a relationship decides: its object's and what reads through it. */
    async #relatedRows(change: Change): Promise<Affected> {
        // affect every row for a subject set's or a scope's relationship
        const objects: { readonly table: Table; readonly id: string }[] = [];
        for (const row of [Change.before(change), Change.after(change)]) {
            if (row === null) {
                continue;
            }
            const type = { packageId: row["objectPackageId"], type: row["objectType"] };
            const mapping = this.#store.authorizer.mapping(ObjectTypeReference.parse(type));
            if (
                row["subjectRelation"] !== null ||
                mapping === undefined ||
                mapping.policy.definition.scope === true
            ) {
                return ALL_ROWS;
            }
            objects.push({
                table: mapping.table,
                id: AccessAudience.#identifier(row, "objectId"),
            });
        }

        return { kind: "rows", read: (positions) => this.#throughRows(positions, objects) };
    }

    /** Read some objects' rows and every row reading its access through them, at some positions. */
    async #throughRows(
        positions: readonly LogPosition[],
        objects: readonly { readonly table: Table; readonly id: string }[],
    ): Promise<Map<Table, Condition>> {
        // list the durable tables' identifier columns a permission reads through
        const durable = new Set(this.#store.durable.map((object) => object.table));
        const followed = this.#store.authorizer
            .mappings()
            .filter((mapping) => durable.has(mapping.table))
            .flatMap((mapping) => {
                const relations = throughRelations(
                    Object.values(mapping.policy.definition.permissions),
                );

                return Object.entries(mapping.relations).flatMap(([relation, field]) =>
                    field.key === undefined && relations.has(relation)
                        ? [{ table: mapping.table, column: field.column }]
                        : [],
                );
            });

        // walk the rows reading through the found ones until none is new
        const found = new Map<Table, Set<string>>();
        for (const { table, id } of objects) {
            found.set(table, (found.get(table) ?? new Set()).add(id));
        }
        const snapshots = positions.map((position) => this.#store.database.log.at(position));
        let frontier = objects.map((object) => object.id);
        while (frontier.length > 0) {
            const next: string[] = [];
            for (const { table, column } of followed) {
                const tuples = frontier.map((id) => [id]);
                const read = await Promise.all(
                    snapshots.map((snapshot) => snapshot.select(table, [column], tuples)),
                );
                for (const row of read.flat()) {
                    const id = AccessAudience.#identifier(row, "id");
                    if (found.get(table)?.has(id) !== true) {
                        found.set(table, (found.get(table) ?? new Set()).add(id));
                        next.push(id);
                    }
                }
            }
            frontier = next;
        }

        return new Map([...found].map(([table, ids]) => [table, { id: { in: [...ids] } }]));
    }

    /** Read an identifier column of a row, refusing another value. */
    static #identifier(row: Row, column: string): string {
        const value = row[column];
        if (typeof value !== "string") {
            throw new TypeError(`column ${column} holds no identifier`);
        }

        return value;
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

            // take every subject set, the caller's identities and arrows, as one run may add the caller
            const via = row["relation"];
            const type = { packageId: row["objectPackageId"], type: row["objectType"] };

            return (
                subject.relation !== undefined ||
                this.#authorization.access.authority.isMember(subject) ||
                (typeof via === "string" &&
                    this.#authorization.authorizer.isArrow(ObjectTypeReference.parse(type), via))
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

    /** List the rows deciding what the caller lists: the chain's access rows and mapped rows. */
    #deciding(): Watch[] {
        // watch nothing for another storage, which decides as of one position
        if (this.#storage !== "durable") {
            return [];
        }

        // watch the chain's access rows, and the rows of every mapped type anywhere, which policies read
        const { authorizer } = this.#store;
        const access = authorizer.watch(
            [...this.chain, ...this.#below],
            this.#authorization.access.authority.subjects,
        );
        const mapped = authorizer.mappedTables.map((table): Watch => ({ table, scopes: "every" }));

        return [...access, ...mapped];
    }

    /** The caller's access, resolved again once access changes. */
    get authorization(): Authorization {
        return this.#authorization;
    }

    /** The scope and the scopes containing it, nearest first. */
    get chain(): string[] {
        return this.#authorization.chain;
    }

    /** Read the listing permission of a table's rows, and a query's permission on them. */
    #listed(
        table: Table,
        permission?: string,
    ):
        | {
              readonly object: ObjectType;
              readonly listing: Permission;
              readonly required: Permission | undefined;
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

        // read the query's permission
        const required = permission === undefined ? undefined : object.permission(permission);

        // list a guarded type under its guard for a replica, which stores rows whole
        const listing =
            object.guard === undefined || this.#concealment === "fields"
                ? object.listing
                : object.permission(object.guard.read);

        return { object, listing, required };
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
    /** The scope's row and the access rows of its chain, whose changes decide again. */
    readonly watches: readonly Watch[];
    /** Require the follower to replicate the scope. */
    readonly require: () => Promise<void>;
}

/** Object types by their tables. */
export class ObjectTypeIndex {
    /** The object types, by table. */
    readonly #objects: ReadonlyMap<Table, ObjectType>;

    /** Index object types by their tables. */
    constructor(objects: readonly ObjectType[]) {
        this.#objects = new Map(objects.map((object) => [object.table, object]));
    }

    /** Read the object type of a table, absent for another table. */
    get(table: Table): ObjectType | undefined {
        return this.#objects.get(table);
    }

    /** Read a table's listed object type and listing permission. */
    listed(table: Table): { readonly object: ObjectType; readonly permission: Permission } {
        const object = this.#objects.get(table);
        if (object?.listing === undefined) {
            throw new TypeError(`no listed object is stored in '${table[TABLE].name}'`);
        }

        return { object, permission: object.listing };
    }

    /** List an object type's guarded fields. */
    concealable(table: Table): readonly string[] {
        return this.#objects.get(table)?.guarded ?? [];
    }
}
