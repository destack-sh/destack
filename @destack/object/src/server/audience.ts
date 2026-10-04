import {
    type ObjectReference,
    type RowKey,
    type Audience,
    Watch,
    replica,
    Scope,
    Subject,
    SyncError,
} from "@destack/sync";
import { accessRelationship, earliest, type Permission } from "@destack/access";
import { journal } from "@destack/audit";
import {
    and,
    eq,
    gt,
    or,
    sql,
    TABLE,
    type Row,
    type SQL,
    type Table,
    type Change,
    type LogPosition,
    Snapshot,
} from "@destack/db";
import { schema, canonicalize } from "@destack/schema";
import type { ServiceContext } from "@destack/service/server";
import type { ObjectStorage, ObjectType } from "../object/object.ts";
import { ObjectTypeIndex } from "../query/listing.ts";
import type { Authorization } from "./authorization.ts";
import type { ObjectServer } from "./server.ts";

/** The most identifiers one dependents read lists, well within the parameter budget. */
const DEPENDENT_IDS = 500;

/** The columns naming a row's parent. */
const PARENT_COLUMNS = ["parentPackageId", "parentType", "parentId"] as const;

/** The caller's access to one stream of a scope's objects. */
export class ObjectAudience implements Audience {
    /** The server serving the objects. */
    readonly #server: Omit<ObjectServer, "router">;
    /** The scope the stream follows. */
    readonly #scope: string;
    /** Admit the subscriber again, as it resolves access once access changes. */
    readonly #admit: () => Promise<Authorization>;
    /** The object types by table. */
    readonly #objects: ObjectTypeIndex;
    /** The caller's access, resolved again once access changes. */
    #authorization: Authorization;
    /** The key of the caller's access inputs. */
    #key: string | undefined;
    /** The earliest moment time alone changes the caller's access. */
    #until: number | undefined;
    /** Where the followed objects live. */
    readonly #storage: ObjectStorage;
    /** The durable log position access was decided at, for ephemeral and external objects. */
    #decided: LogPosition | undefined;
    /** Whether the subscriber keeps every row by containing the scope, with only guarded fields decided. */
    readonly #isContained: boolean;
    /** The scopes below the followed one whose access decided rows. */
    readonly #below = new Set<string>();
    /** The scopes a follower outside the chains copies them via, with the rows deciding whether it may replicate each. */
    readonly #via: readonly Relay[];
    /** The access rows of the scope chain and of the scopes below deciding rows, none for ephemeral and external objects. */
    watches: readonly Watch[];

    /** Serve a caller of a scope with resolved access. */
    private constructor(
        server: Omit<ObjectServer, "router">,
        scope: string,
        admit: () => Promise<Authorization>,
        authorization: Authorization,
        storage: ObjectStorage,
        decided: LogPosition | undefined,
        isContained: boolean,
        via: readonly Relay[],
    ) {
        // keep the inputs and watch access
        this.#server = server;
        this.#scope = scope;
        this.#admit = admit;
        this.#objects = new ObjectTypeIndex(server.objects);
        this.#authorization = authorization;
        this.#until = authorization.access.until;
        this.#storage = storage;
        this.#decided = decided;
        this.#isContained = isContained;
        this.#via = via;
        this.watches = this.#watched();
    }

    /** Resolve a reader's access to a scope's objects: an admitted caller, or a principal the kept copies are decided for. */
    static async of(
        server: Omit<ObjectServer, "router">,
        scope: string,
        reader: ServiceContext | Subject,
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
        const decided = storage === "durable" ? undefined : await server.database.log.position();

        // read the scopes a follower copies chains via
        const via =
            options.via === undefined
                ? []
                : await ObjectAudience.#relays(server, options.via, reader);

        // authorize the reader as a principal, a contained caller or an admitted caller
        const admit = async () => {
            const authorization = await ("packageId" in reader
                ? server.authorizeSubject(server.database, scope, reader, options.within)
                : isContained
                  ? server.authorize(server.database, scope, reader)
                  : server.admit(server.database, scope, reader));

            // require the follower to replicate the scopes it copies chains via
            await Promise.all(via.map((relay) => relay.require()));

            return authorization;
        };

        return new ObjectAudience(
            server,
            scope,
            admit,
            await admit(),
            storage,
            decided,
            isContained,
            via,
        );
    }

    /** Read the rows deciding whether a follower may replicate the scopes it copies chains via: each scope's own row and the access rows of its chain. */
    static async #relays(
        server: Omit<ObjectServer, "router">,
        via: readonly string[],
        reader: ServiceContext | Subject,
    ): Promise<Relay[]> {
        // read the scopes' chains at once
        const chains = await Scope.chains(Snapshot.live(server.database), via);

        return via.map((id) => {
            // read the scope's object, refusing an unknown scope
            const links = chains.get(id) ?? [];
            const scope = links[0]?.object;
            if (scope === undefined) {
                throw new SyncError("NOT_FOUND", `unknown scope: ${id}`);
            }

            // watch the scope's own row where this database maps its type, and the access rows of its chain
            const mapping = server.authorizer.mappingOf(scope);
            const own: Watch[] =
                mapping === undefined
                    ? []
                    : [
                          {
                              table: mapping.table,
                              scopes: [scope.scope],
                              where: { [mapping.id]: id },
                          },
                      ];
            const chain = links.map((link) => link.object.id);

            return {
                scope: id,
                watches: [...own, ...server.authorizer.watch(chain)],
                require: () => server.source.requireReplicateVia(id, reader),
            };
        });
    }

    /** The scope's own object, absent for a scope the database does not know. */
    get scope(): ObjectReference | undefined {
        return this.#authorization.access.scopes[0];
    }

    /** Match the rows of a table the caller may list. */
    where(table: Table): SQL | "memory" {
        // leave the journal and the copies' records to their queries
        const listed = this.#listed(table);
        if (listed === "query") {
            return sql`true`;
        }
        // decide in memory the rows at their home in the scopes below the followed one, each in its own chain
        else if (
            listed.object.storage === "durable" &&
            listed.object.inherited === undefined &&
            !this.#server.database.copies(listed.object.table) &&
            !this.#authorization.isScopeOf(listed.object)
        ) {
            return "memory";
        }

        return this.#authorization.listable(listed.object, listed.permission);
    }

    /** The key of the caller's access inputs. */
    get key(): string {
        this.#key ??= canonicalize({
            scope: this.#scope,
            context: { ...this.#authorization.access.context, request: null, now: null },
            isContained: this.#isContained,
            via: this.#via.map((relay) => relay.scope),
        });

        return this.#key;
    }

    /** Decide which rows the caller may list. */
    async admits(
        table: Table,
        rows: readonly Row[],
        position: LogPosition,
    ): Promise<ReadonlySet<number>> {
        // admit every row left to its query
        const listed = this.#listed(table);
        if (listed === "query") {
            return new Set(rows.keys());
        }
        const { permission } = listed;

        // check the listing permission, admitting inherited copies
        const admission = await this.#authorization.admitRows(
            listed.object,
            permission,
            this.#scope,
            rows,
            this.#reader(position),
        );
        this.#until = earliest([this.#until, admission.until]);

        // watch the access of the scopes below that decided rows
        const known = this.#below.size;
        for (const scope of admission.below) {
            this.#below.add(scope);
        }
        if (this.#below.size > known) {
            this.watches = this.#watched();
        }

        return admission.permitted;
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
        // conceal nothing of the rows left to their query, or of another table's rows a follower keeps by containment
        const listed = this.#isContained ? this.#objects.get(table) : this.#listed(table);
        if (listed === "query" || listed === undefined) {
            return rows.map(() => []);
        }
        const object = "object" in listed ? listed.object : listed;

        // conceal unreadable fields
        const concealed = await this.#authorization.concealed(object, rows, this.#reader(position));
        this.#until = earliest([this.#until, concealed.until]);

        return concealed.hidden;
    }

    /** Read the earliest moment time alone changes the caller's access. */
    async until(): Promise<number | undefined> {
        return this.#until;
    }

    /** Resolve the caller's access again. */
    async refresh(): Promise<void> {
        await this.#authorize();
    }

    /** List the rows with access a change decides, or everything. */
    async dependents(change: Change): Promise<readonly RowKey[] | "everything"> {
        // require the follower to replicate the scopes it copies via again after a change only their decisions read
        const via = this.#via.filter((relay) => Watch.matches(relay.watches, change));
        if (via.length > 0 && !Watch.matches(this.#deciding(), change)) {
            await Promise.all(via.map((relay) => relay.require()));

            return [];
        }

        // resolve access after an access change, requiring the follower to copy via its scopes again
        if (this.#objects.get(change.table) === undefined && change.table !== journal) {
            await this.#authorize();
            if (change.table !== accessRelationship) {
                return "everything";
            }
            const row = change.operation === "insert" ? change.after : change.before;
            const object = this.#server.objects.find(
                (entry) =>
                    entry.policy.definition.packageId === row["packageId"] &&
                    entry.policy.definition.name === row["type"],
            );
            if (object === undefined) {
                return "everything";
            }

            return this.#dependentsOf(object, identifier(row["objectId"]));
        }

        // follow a moved row
        const object = this.#objects.get(change.table);
        if (
            object?.parent === undefined ||
            change.operation !== "update" ||
            PARENT_COLUMNS.every((column) => change.before[column] === change.after[column])
        ) {
            return [];
        }

        return this.#dependentsOf(object, identifier(change.key["id"]));
    }

    /** List the rows whose access an object's access decides. */
    async #dependentsOf(object: ObjectType, id: string): Promise<RowKey[]> {
        // walk the types level by level
        const keys: RowKey[] = [];
        let level = new Map<ObjectType, string[]>([[object, [id]]]);
        while (level.size > 0) {
            const next = new Map<ObjectType, string[]>();
            for (const [type, ids] of level) {
                // collect rows and subtrees
                const listed = [...ids, ...(await this.#subtree(type, ids))];
                keys.push(...listed.map((entry) => ({ table: type.table, key: { id: entry } })));

                // continue to durable children and attachments
                for (const child of this.#server.objects.filter(
                    (served) => served.storage === "durable",
                )) {
                    const isAttached = type.attaches(child);
                    if ((type.same(child.parent?.object) && !child.same(type)) || isAttached) {
                        next.set(child, [
                            ...(next.get(child) ?? []),
                            ...(await this.#children(child, type, listed)),
                        ]);
                    }
                }
            }
            level = next;
        }

        return keys;
    }

    /** Read the proper descendants of some rows of a tree type. */
    async #subtree(object: ObjectType, ids: readonly string[]): Promise<string[]> {
        // read descendants in batches
        const path = object.tree?.ancestors;
        const below: string[] = [];
        if (path === undefined) {
            return below;
        }
        for (let start = 0; start < ids.length; start += DEPENDENT_IDS) {
            const batch = ids.slice(start, start + DEPENDENT_IDS);
            const rows = await this.#server.database
                .select({ id: path.descendant })
                .from(path)
                .where(
                    and(
                        eq(path.scope, this.#scope),
                        or(...batch.map((entry) => eq(path.ancestor, entry))),
                        gt(path.depth, 0),
                    ),
                );
            below.push(...rows.map((row) => identifier(row.id)));
        }

        return below;
    }

    /** Read the identifiers of the children of some rows. */
    async #children(
        child: ObjectType,
        parent: ObjectType,
        parents: readonly string[],
    ): Promise<string[]> {
        // read children in batches
        const table = child.table;
        const definition = table[TABLE];
        const hosted =
            child.parent?.object === "any"
                ? [
                      eq(definition.column("parentPackageId"), parent.policy.definition.packageId),
                      eq(definition.column("parentType"), parent.name),
                  ]
                : [];
        const found: string[] = [];
        for (let start = 0; start < parents.length; start += DEPENDENT_IDS) {
            const batch = parents.slice(start, start + DEPENDENT_IDS);
            const rows = await this.#server.database
                .select({ id: definition.column("id") })
                .from(table)
                .where(
                    and(
                        child.inScope(this.#scope),
                        ...hosted,
                        or(...batch.map((entry) => eq(definition.column("parentId"), entry))),
                    ),
                );
            found.push(...rows.map((row) => identifier(row.id)));
        }

        return found;
    }

    /** Resolve the caller's access again and reset derived state. */
    async #authorize(): Promise<void> {
        // admit again and reset
        this.#authorization = await this.#admit();
        this.#key = undefined;
        this.#until = this.#authorization.access.until;
        if (this.#storage !== "durable") {
            this.#decided = await this.#server.database.log.position();
        }
        this.#below.clear();
        this.watches = this.#watched();
    }

    /** List the rows whose changes decide again: the access rows deciding what the caller lists, and those deciding whether it may copy via its scopes. */
    #watched(): Watch[] {
        return [...this.#deciding(), ...this.#via.flatMap((relay) => relay.watches)];
    }

    /** List the access rows deciding what the caller lists: those of the chain and of the scopes below deciding rows, and the relationships of the caller's subjects in every scope. */
    #deciding(): Watch[] {
        return this.#storage === "durable"
            ? this.#server.authorizer.watch(
                  [...this.chain, ...this.#below],
                  this.#authorization.access.authorities.flatMap((authority) => authority.subjects),
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

    /** Read the listing permission deciding a table's rows. */
    #listed(
        table: Table,
    ): { readonly object: ObjectType; readonly permission: Permission } | "query" {
        // leave the journal and the copies' records to their queries, and every row to a follower keeping it by containment
        if (this.#isContained || table === journal || table === replica) {
            return "query";
        }

        // read the listing permission
        return this.#objects.listed(table);
    }

    /** Share one grant reader among the scope's subscribers at a position. */
    #reader(position: LogPosition) {
        return this.#server.reader(this.#authorization.access, this.#decided ?? position);
    }
}

/** A scope a follower outside its chain copies the chain via, which the follower must replicate at every decision. */
interface Relay {
    /** The scope. */
    readonly scope: string;
    /** The scope's own row and the access rows of its chain, whose changes decide again. */
    readonly watches: readonly Watch[];
    /** Require the follower to replicate the scope. */
    require(): Promise<void>;
}

/** Read an identifier of a row. */
function identifier(value: unknown): string {
    return schema.string().parse(value);
}

/** The access of each recipient of a type's rows, deciding every row for the recipient its field names, as a projection's source decides them. */
export class RecipientAudience implements Audience {
    /** The recipient field of the rows. */
    readonly #field: string;
    /** Each recipient's access, by subject key. */
    readonly #members: ReadonlyMap<string, ObjectAudience>;

    /** Hold each recipient's resolved access. */
    private constructor(field: string, members: ReadonlyMap<string, ObjectAudience>) {
        this.#field = field;
        this.#members = members;
    }

    /** Resolve each recipient's access to a scope's objects. */
    static async of(
        server: Omit<ObjectServer, "router">,
        scope: string,
        field: string,
        recipients: readonly string[],
    ): Promise<RecipientAudience> {
        const members = await Promise.all(
            recipients.map(async (recipient): Promise<[string, ObjectAudience]> => [
                recipient,
                await ObjectAudience.of(server, scope, Subject.read(recipient)),
            ]),
        );

        return new RecipientAudience(field, new Map(members));
    }

    /** The access rows each recipient's decisions read. */
    get watches(): readonly Watch[] {
        const watches = [...this.#members.values()].flatMap((member) => member.watches);

        return [
            ...new Map(
                watches.map((watch) => [
                    canonicalize([watch.table[TABLE].sqlName, watch.scopes, watch.where ?? null]),
                    watch,
                ]),
            ).values(),
        ];
    }

    /** Decide every row in memory, by its recipient. */
    where(): "memory" {
        return "memory";
    }

    /** The recipients and their access inputs. */
    get key(): string {
        return canonicalize(
            [...this.#members].map(([recipient, member]) => [recipient, member.key]),
        );
    }

    /** Admit each row its recipient may list. */
    async admits(
        table: Table,
        rows: readonly Row[],
        position: LogPosition,
    ): Promise<ReadonlySet<number>> {
        const admitted = new Set<number>();
        for (const [member, entries] of this.#groups(rows)) {
            const permitted = await member.admits(
                table,
                entries.map(({ row }) => row),
                position,
            );
            for (const [index, { position: at }] of entries.entries()) {
                if (permitted.has(index)) {
                    admitted.add(at);
                }
            }
        }

        return admitted;
    }

    /** List the guarded fields any recipient may not read. */
    concealable(table: Table): readonly string[] {
        return [
            ...new Set([...this.#members.values()].flatMap((member) => member.concealable(table))),
        ];
    }

    /** List the guarded fields each row's recipient may not read. */
    async conceals(
        table: Table,
        rows: readonly Row[],
        position: LogPosition,
    ): Promise<readonly (readonly string[])[]> {
        const concealed: (readonly string[])[] = rows.map(() => []);
        for (const [member, entries] of this.#groups(rows)) {
            const hidden = await member.conceals(
                table,
                entries.map(({ row }) => row),
                position,
            );
            for (const [index, { position: at }] of entries.entries()) {
                concealed[at] = hidden[index] ?? [];
            }
        }

        return concealed;
    }

    /** Read when time alone next changes a recipient's decisions. */
    async until(): Promise<number | undefined> {
        return earliest(
            await Promise.all([...this.#members.values()].map((member) => member.until())),
        );
    }

    /** Decide every recipient's rows as of now. */
    async refresh(): Promise<void> {
        await Promise.all([...this.#members.values()].map((member) => member.refresh()));
    }

    /** List the rows a change decides for any recipient. */
    async dependents(change: Change): Promise<readonly RowKey[] | "everything"> {
        const dependents = await Promise.all(
            [...this.#members.values()].map((member) => member.dependents(change)),
        );

        // decide everything when any member does
        if (dependents.includes("everything")) {
            return "everything";
        }

        return dependents.flatMap((keys) => (keys === "everything" ? [] : keys));
    }

    /** Group rows by the recipient deciding them, leaving out rows of no known recipient. */
    #groups(
        rows: readonly Row[],
    ): Map<ObjectAudience, { readonly row: Row; readonly position: number }[]> {
        const groups = new Map<
            ObjectAudience,
            { readonly row: Row; readonly position: number }[]
        >();
        for (const [position, row] of rows.entries()) {
            const recipient = row[this.#field];
            const member = typeof recipient === "string" ? this.#members.get(recipient) : undefined;
            if (member !== undefined) {
                const entries = groups.get(member) ?? [];
                entries.push({ row, position });
                groups.set(member, entries);
            }
        }

        return groups;
    }
}
