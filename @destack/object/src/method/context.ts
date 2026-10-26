import type { Environment } from "@destack/environment";
import { ObjectReference, type ObjectTypeReference, Subject } from "@destack/sync";
import { ObjectError } from "../error/error.ts";
import {
    AccessContext,
    type Authorizer,
    Caller,
    type Creation,
    type Decision,
    type Manager,
    type Permission,
    type PermissionReference,
    principal,
    Grant,
} from "@destack/access";
import {
    and,
    eq,
    type DatabaseConnection,
    type Insert,
    type InsertValue,
    Key,
    type JsonOf,
    Row,
    type Select,
    TABLE,
    type LogPosition,
    type RelationalQueryBuilder,
    type Transaction,
    Table,
} from "@destack/db";
import type { EventKeyFields, EventKind } from "@destack/event";
import type { BranchEntry } from "../branch/entry.ts";
import {
    found,
    Identifier,
    present,
    schema,
    type JsonObject,
    type JsonValue,
} from "@destack/schema";
import type { PackageId } from "@destack/package";

import * as sync from "@destack/sync";
import type { ObjectOf, ObjectType } from "../object/object.ts";
import type { ManagedTable } from "../trait/record.ts";
import { Observation, type ObservationReport, type ObservedRecord } from "../object/condition.ts";
import type { OptionOf, Method, MethodConfiguration } from "./method.ts";
import { CallSite } from "./site.ts";
import type { MethodKind } from "./kind.ts";
import type { InstalledServerContext } from "../workload/workload.ts";
import type { Settled, SettlementContext } from "./settlement.ts";
import { type IsSum, RecordMethod } from "./record.ts";
import type { Listing } from "../query/listing.ts";
import { ParentReference } from "../trait/nested.ts";
import type { Session } from "./reader.ts";
import type { Follow, FollowKey } from "../replica/shape.ts";
import { EventView } from "../query/event.ts";
import { Execution } from "./execution.ts";
import { BranchWrite } from "../branch/write.ts";
import { Replay } from "../audit/replay.ts";

import type {} from "@destack/package/import-meta";

/** The values of a draw: the integers a double represents exactly, 2^53. */
const RANDOM_RANGE = 2 ** 53;

/** The bits a draw drops from a 64-bit word leaving 53. */
const RANDOM_SHIFT = 11n;

/** The identifier and revision of a row the record writes change at its revision. */
export const RecordedRow = Object.assign(
    schema.looseObject({
        id: schema.string().min(1),
        revision: schema.number().int().positive(),
    }),
    {
        /** Match a table's row at its identifier and revision. */
        at(table: Table, row: { readonly id: string; readonly revision: number }) {
            const definition = table[TABLE];

            return and(
                eq(definition.column("id"), row.id),
                eq(definition.column("revision"), row.revision),
            );
        },
    },
);

/** A controlled target's generation, stored by its trait beside the record columns. */
const GENERATION = schema.number().int();

/** The context of one method call, served or predicted, typed by its method's declaration. */
export class MethodContext<
    Configuration extends MethodConfiguration = MethodConfiguration,
    Prepared = PreparedOf<Configuration>,
    Id extends string | undefined = IdOf<Configuration>,
    Target extends Select<TableOf<Configuration>> | undefined = TargetOf<Configuration>,
> {
    /** The object type. */
    readonly object: ObjectOf<{ table: TableOf<Configuration> }>;
    /** The method's name. */
    readonly name: string;
    /** The method. */
    readonly method: Method;
    /** The scope containing the object. */
    readonly scope: string;
    /** The scope and the scopes containing it, nearest first. */
    readonly chain: readonly string[];
    /** The method's input, as its schema reads it. */
    readonly input: InputValueOf<Configuration>;
    /** The target's or created object's identifier, absent for a method on its scope. */
    readonly id: Id;
    /** The target at the start of the call, loaded for targeted methods. */
    readonly target: Target;
    /** The caller: who the call is for, who acts, and what it presented, at the call's time. */
    readonly caller: AccessContext;
    /** Where the call runs: predicted on a client replica, or served by an authorized store. */
    readonly #location: CallLocation;
    /** The object types served together. */
    readonly objects: readonly ObjectType[];
    /** The object types queried here over rows replicated from their home. */
    readonly replicas: readonly ObjectType[];
    /** The value the prepare phase produced. */
    readonly prepared: Prepared;
    /** The idempotency key of the call's external work. */
    readonly idempotencyKey?: string;
    /** The client instance writing an ephemeral object. */
    readonly writerId?: string;
    /** The request a durable mutation replays under. */
    readonly requestId?: string;
    /** Run another method in the call's transaction as its caller. */
    readonly run?: Run;
    /** The calls the method's expansion ran before it in its mutation. */
    readonly expansion?: readonly BranchEntry[];
    /** The fields the call was made of, which replicates change. */
    readonly #fields: MethodContextOptions<Configuration, Prepared, Id, Target>;

    /** Store one call's fields. */
    constructor(fields: MethodContextOptions<Configuration, Prepared, Id, Target>) {
        // refuse a mutation by a caller proving no identity
        if (fields.method.mutates && Caller.isAnonymous(fields.caller)) {
            throw ObjectError.of("UNAUTHENTICATED", "a mutation needs an authenticated caller");
        }

        // take the required fields
        this.#fields = fields;
        this.object = fields.object;
        this.name = fields.name;
        this.method = fields.method;
        this.scope = fields.scope;
        this.chain = fields.chain;
        this.input = fields.input;
        this.id = fields.id;
        this.target = fields.target;
        this.caller = fields.caller;
        this.prepared = fields.prepared;
        this.objects = fields.objects;
        this.replicas = fields.replicas;
        this.#location = fields.location;

        // take the present optional fields
        if (fields.idempotencyKey !== undefined) {
            this.idempotencyKey = fields.idempotencyKey;
        }
        if (fields.writerId !== undefined) {
            this.writerId = fields.writerId;
        }
        if (fields.requestId !== undefined) {
            this.requestId = fields.requestId;
        }
        if (fields.run !== undefined) {
            this.run = fields.run;
        }
        if (fields.expansion !== undefined) {
            this.expansion = fields.expansion;
        }
    }

    /** Run calls in this call's transaction as a caller, returning each result in order. */
    async mutate<const Calls extends readonly sync.Call[]>(
        caller: AccessContext,
        calls: Calls,
    ): Promise<ResultsOf<Calls>> {
        // run each call as the code of the package implementing the method
        const site = this.#site();
        const results: unknown[] = [];
        for (const entry of calls) {
            results.push(await this.#run(caller, entry, true, site));
        }

        return MethodContext.results([...this.objects, ...this.replicas], calls, results);
    }

    /** The environment the call runs in: its store's, its client's or its installation's. */
    get environment(): Environment {
        const location = this.#location;

        return location.kind === "sandbox"
            ? location.server.environment
            : location.kind === "store"
              ? location.store.environment
              : location.environment;
    }

    /** Generate a new identifier from the call's request, stable across attempts in order. */
    async generateId(object: ObjectType): Promise<Identifier<string>> {
        return object.deriveId(this.#requireRequestId(), await this.takePosition());
    }

    /** Draw a number in [0, 1) from the call's request, stable across attempts in order. */
    async random(): Promise<number> {
        // read 53 bits of the digest of the request and the transaction's next position
        const key = `${this.#requireRequestId()}/${await this.takePosition()}`;
        const digest = await this.environment.crypto.digest(
            "SHA-256",
            new TextEncoder().encode(key),
        );
        const bits = new DataView(digest).getBigUint64(0) >> RANDOM_SHIFT;

        return Number(bits) / RANDOM_RANGE;
    }

    /** Take the position of the transaction's next derived identifier, refusing a prediction. */
    takePosition(): Promise<number> {
        // take it from the store's transaction, else through the hook's session
        const location = this.#location;
        if (location.kind === "store") {
            return Promise.resolve(location.transaction.takePosition());
        } else if (location.kind === "sandbox") {
            return location.session.takePosition();
        }

        throw new TypeError(
            `${this.object.name}.${this.name} can only derive identifiers in its store`,
        );
    }

    /** Read the request the call runs in, which every server call has. */
    #requireRequestId(): string {
        return present(this.requestId, `the request of ${this.object.name}.${this.name}`);
    }

    /** Record an object's conditions and observed fields at its generation as the package, once. */
    async observe(
        object: ObjectType,
        row: ObservedRecord & Readonly<Record<string, unknown>>,
        observation: ObservationReport,
    ): Promise<unknown> {
        // skip an observation the row has
        if (Observation.isRecorded(row, observation)) {
            return row;
        }

        // record the generation, the conditions and the fields
        const [written] = await this.mutate(this.self, [
            Observation.call(object, row, observation),
        ]);

        return written;
    }

    /** Read through a method in this call's transaction as a caller. */
    async query<Result>(caller: AccessContext, call: sync.Call<Result>): Promise<Result> {
        const [result] = MethodContext.results(
            [...this.objects, ...this.replicas],
            [call],
            [await this.#run(caller, call, false, this.#site())],
        );

        return result;
    }

    /** Read the site of the call's code: the package implementing its type. */
    #site(): CallSite {
        const location = this.#location;

        return CallSite.package(
            location.kind === "store"
                ? location.store.implementer(this.object)
                : this.object.package.id,
        );
    }

    /** Run a call a package's sandboxed hook sent in this call's transaction. */
    runSandboxed(
        caller: AccessContext,
        entry: sync.Call,
        mutates: boolean,
        packageId: PackageId,
    ): Promise<unknown> {
        return this.#run(caller, entry, mutates, CallSite.package(packageId));
    }

    /** Run one platform call in this call's transaction and scope, refusing it without a runner. */
    async #run(
        caller: AccessContext,
        entry: sync.Call,
        mutates: boolean,
        site: CallSite,
    ): Promise<unknown> {
        if (this.run === undefined) {
            throw new TypeError(`${this.object.name}.${this.name} cannot run nested calls`);
        }

        return this.run(entry, caller, mutates, site);
    }

    /** Read the answers of calls as their methods' results, parsed by their output schemas. */
    static results<const Calls extends readonly sync.Call[]>(
        objects: readonly ObjectType[],
        calls: Calls,
        answers: readonly unknown[],
    ): ResultsOf<Calls>;
    /**
     * Read the answers of calls, each parsed by its method's output schema.
     *
     * NOTE #Architecture: the one stated type of call results, as an oRPC client reads its output.
     */
    static results(
        objects: readonly ObjectType[],
        calls: readonly sync.Call[],
        answers: readonly unknown[],
    ): unknown[] {
        return calls.map((entry, index) => {
            // parse each answer by its type's method
            const type = present(
                objects.find((each) => each.isTypeOf(entry)),
                sync.Call.label(entry),
            );

            return type.result(entry.method, answers[index]);
        });
    }

    /** Copy the call with some fields changed, with its types. */
    with(
        changes: Partial<MethodContextOptions<Configuration, Prepared, Id, Target>>,
    ): MethodContext<Configuration, Prepared, Id, Target> {
        return new MethodContext({ ...this.#fields, ...changes });
    }

    /** Read the context as creating or acting on the object under an identifier. */
    identify(id: string): MethodContext<Configuration, Prepared, string, Target> {
        return new MethodContext({ ...this.#fields, id });
    }

    /** Read the context as acting on one target row, loaded. */
    at(
        row: Select<TableOf<Configuration>>,
    ): MethodContext<Configuration, Prepared, string, Select<TableOf<Configuration>>> {
        return new MethodContext({ ...this.#fields, id: RecordedRow.parse(row).id, target: row });
    }

    /** The client replica a prediction reads and writes, refusing a store's or a sandbox's call. */
    get #replica(): DatabaseConnection {
        if (this.#location.kind !== "client") {
            throw new TypeError(`${this.object.name}.${this.name} must read through context.read`);
        }

        return this.#location.database;
    }

    /** Whether a client predicts the call, which it runs without the server's authorization. */
    get isPredicted(): boolean {
        return this.#location.kind === "client";
    }

    /** Read a table's or object type's rows in the call's view. */
    read<Read extends Table>(
        source: Read | { readonly table: Read },
    ): RelationalQueryBuilder<Read> {
        // read through the hook's session, else the store's view or the client's replica
        const table = TABLE in source ? source : source.table;
        const location = this.#location;

        return location.kind === "sandbox"
            ? location.session.read(table)
            : Execution.snapshot(location).read(table);
    }

    /** Read the whole rows of a type of the call's package, sensitive columns included. */
    reveal<Read extends Table>(type: {
        readonly table: Read;
        readonly name: string;
        readonly package: { readonly id: string };
    }): RelationalQueryBuilder<Read, Select<Read>> {
        // refuse another package's type, by the packages declaring them
        if (type.package.id !== this.object.package.id) {
            throw ObjectError.of(
                "REVEAL_REFUSED",
                `${this.object.package.name} cannot reveal ${type.name}: only the declaring package can`,
            );
        }

        // read through the store's transaction, else through the hook's session
        const location = this.#location;
        if (location.kind === "store") {
            return location.transaction.read(type.table);
        } else if (location.kind === "sandbox") {
            return location.session.reveal(type.table);
        }

        // refuse a prediction, whose replica has no sensitive column
        throw new TypeError(`${this.object.name}.${this.name} can only reveal rows in its store`);
    }

    /** Read the events of an event kind of the call's package, refusing any other kind. */
    events<Fields extends EventKeyFields, Data extends JsonValue>(
        kind: EventKind<Fields, Data>,
    ): EventView<Fields, Data> {
        // refuse another package's kind
        if (kind.package.id !== this.object.package.id) {
            throw ObjectError.of(
                "EVENTS_REFUSED",
                `${this.object.package.name} cannot read ${kind.name} events: only the declaring package can`,
            );
        }

        // read through the store's event log, else through the hook's session
        const location = this.#location;
        if (location.kind === "store") {
            return new EventView(kind, location.store.events);
        } else if (location.kind === "sandbox") {
            return new EventView(kind, location.session);
        }

        // refuse a prediction, whose replica has no event log
        throw new TypeError(`${this.object.name}.${this.name} can only read events in its store`);
    }

    /** The authorizer of the store serving the call, refusing outside a store. */
    get authorizer(): Authorizer {
        // read the store's authorizer
        const location = this.#location;
        if (location.kind !== "store") {
            throw new TypeError(
                `${this.object.name}.${this.name} can only read the authorizer in its store`,
            );
        }

        return location.authorization.authorizer;
    }

    /** The declaration managing the call's writes, null for a call no stack declaration makes. */
    get manager(): Manager | null {
        // read the manager of the store's authorization
        const location = this.#location;
        if (location.kind !== "store") {
            throw new TypeError(
                `${this.object.name}.${this.name} can only read its manager in its store`,
            );
        }

        return location.authorization.manager;
    }

    /** List a page of the subjects of a type with a permission on an object. */
    lookup(
        permission: PermissionReference,
        target: ObjectReference,
        options: {
            readonly type: ObjectTypeReference;
            readonly page: { readonly after?: string; readonly limit: number };
        },
    ): Promise<readonly Subject[]> {
        // list through the store's authorizer, else through the hook's session
        const location = this.#location;
        if (location.kind === "store") {
            const { authorizer } = location.authorization;

            return authorizer.subjects(
                Execution.snapshot(location),
                permission,
                target,
                options.type,
                this.now,
                options.page,
            );
        } else if (location.kind === "sandbox") {
            return location.session.lookup({
                permission,
                object: target,
                type: options.type,
                page: {
                    ...(options.page.after === undefined ? {} : { after: options.page.after }),
                    limit: options.page.limit,
                },
            });
        }

        throw this.#unauthorized();
    }

    /** Run calls as their authors over some rows and undo them, returning the rows they change. */
    async rehearse(
        over: readonly BranchWrite[],
        calls: readonly BranchEntry[],
    ): Promise<readonly BranchWrite[]> {
        // rehearse through the hook's session, and refuse a prediction
        const location = this.#location;
        if (location.kind === "sandbox") {
            return location.session.rehearse(over, calls);
        } else if (location.kind === "client") {
            throw new TypeError(
                `${this.object.name}.${this.name} can only rehearse calls in its store`,
            );
        }

        // rehearse over the store's transaction, dropping what the rehearsal writes
        const { transaction } = location;
        const tables = new Map(
            this.objects
                .filter((object) => object.storage === "durable")
                .map((object) => [object.table[TABLE].sqlName, object.table]),
        );
        const { changes } = await transaction.rehearse(async () => {
            // write the rows over the main line's
            for (const entry of over) {
                const table = found(tables, entry.table);
                const key = Table.decode(table, entry.key);
                const main = await transaction
                    .read(table)
                    .findFirst({ where: Key.where(table, key) });
                const after = entry.after === null ? null : Table.decode(table, entry.after);
                await transaction.write({ table, before: main ?? null, after });
            }

            // run each call apart as its author's remote call, skipping a refusal for good
            for (const { author, ...entry } of calls) {
                await MethodContext.#attempt(transaction, () =>
                    this.#run(
                        AccessContext.of(Caller.of(author), this.now),
                        entry,
                        true,
                        CallSite.client,
                    ),
                );
            }
        });

        return BranchWrite.capture(changes);
    }

    /** Run a rehearsed call in a new savepoint, skipping a refusal the store makes for good. */
    static async #attempt(transaction: Transaction, run: () => Promise<unknown>): Promise<void> {
        try {
            await transaction.savepoint(run);
        } catch (error) {
            if (Replay.failure(error) === undefined) {
                throw error;
            }
        }
    }

    /** Decide whether the caller has a permission on an object, refusing a prediction. */
    check(permission: PermissionReference, target: ObjectReference): Promise<Decision> {
        // decide through the store's authorization, else through the hook's session
        const location = this.#location;
        if (location.kind === "store") {
            return location.authorization.check(permission, target);
        } else if (location.kind === "sandbox") {
            return location.session.check(permission, target);
        }

        throw this.#unauthorized();
    }

    /** Decide whether another subject has a permission on an object now, as Zanzibar checks. */
    async permits(
        subject: Subject,
        permission: PermissionReference,
        target: ObjectReference,
    ): Promise<boolean> {
        // decide through the store's authorizer for the subject alone
        const location = this.#location;
        if (location.kind !== "store") {
            throw new TypeError(
                `${this.object.name}.${this.name} decides for another subject only in its store`,
            );
        }
        const { authorizer, snapshot } = location.authorization;
        const context = AccessContext.of(Caller.of(subject), this.now);
        const access = await authorizer.resolve(
            snapshot,
            authorizer.governingScope(target),
            context,
        );

        return (await authorizer.check(snapshot, permission, target, access)).isAllowed;
    }

    /** Require the caller to have a permission on an object, refusing a prediction. */
    require(permission: PermissionReference, target: ObjectReference): Promise<void> {
        // decide through the store's authorization, else through the hook's session
        const location = this.#location;
        if (location.kind === "store") {
            return location.authorization.require(permission, target);
        } else if (location.kind === "sandbox") {
            return location.session.require(permission, target);
        }

        throw this.#unauthorized();
    }

    /** Refuse a decision outside the store, which alone authorizes. */
    #unauthorized(): Error {
        return ObjectError.of(
            "STORE_ONLY",
            `${this.object.name} can only be authorized in its store`,
        );
    }

    /** Read the permission the method requires of its caller. */
    permission(): Permission {
        return this.object.permission(this.method.permission);
    }

    /** Reference the object the call acts on, in the call's scope. */
    reference(this: MethodContext<Configuration, Prepared, string, Target>): ObjectReference {
        return this.object.reference(this.scope, this.id);
    }

    /** Read the parent the call's input refers to. */
    parent(): ObjectReference | undefined {
        // read the given parent of a nested object
        const declared = this.object.parent;
        const input: JsonObject = this.input;
        const named = declared?.object === "any" ? input["parent"] : input["parentId"];
        if (declared === undefined || named === undefined || named === null) {
            return undefined;
        }

        // place the parent in the object's scope
        return declared.object === "any"
            ? ObjectReference.parse({ ...ParentReference.parse(named), scope: this.scope })
            : declared.object.reference(this.scope, schema.string().parse(named));
    }

    /** Read the parent the call's input refers to, refusing a call without one. */
    requireParent(): ObjectReference {
        const parent = this.parent();
        if (parent === undefined) {
            throw ObjectError.of("PARENT_MISSING", `${this.object.name} needs a parent`);
        }

        return parent;
    }

    /** Read the parent columns the call's input writes. */
    parentColumns(): Row {
        // write nothing for an object without a parent, and the typed parent's identifier as given
        const declared = this.object.parent;
        if (declared === undefined) {
            return {};
        } else if (declared.object !== "any") {
            const input: JsonObject = this.input;

            return input["parentId"] === undefined
                ? {}
                : Table.decode(this.object.table, { parentId: input["parentId"] });
        }

        // write no parent, or the reference's fields
        const parent = this.parent();

        return parent === undefined
            ? { parentPackageId: null, parentType: null, parentId: null }
            : { parentPackageId: parent.packageId, parentType: parent.type, parentId: parent.id };
    }

    /** Follow a shape's key on demand in a prepare, unavailable until its rows are here. */
    async acquire<Parameters extends object>(
        follow: Extract<Follow<Parameters>, { readonly onDemand: unknown }>,
        at: FollowKey<Parameters>,
    ): Promise<void> {
        // acquire through the executing store, refusing a sandbox's hook and a prediction
        const location = this.#location;
        if (location.kind !== "store") {
            throw new TypeError(
                `${this.object.name}.${this.name} can only acquire replicas in its store`,
            );
        }

        await location.store.acquire(follow, at);
    }

    /** Insert an object of the package's own type as its create method writes it. */
    async insert<Inserted extends Table>(
        type: ObjectType & { readonly table: Inserted },
        values: Readonly<Partial<Insert<Inserted>>>,
    ): Promise<Select<Inserted>> {
        // refuse another package's type, and a call outside the store
        const location = this.#location;
        if (type.package.id !== this.object.package.id) {
            throw ObjectError.of(
                "INSERT_REFUSED",
                `${this.object.package.name} cannot insert ${type.name}: only the declaring package can`,
            );
        } else if (location.kind !== "store") {
            throw new TypeError(`${this.object.name}.${this.name} can only insert in its store`);
        }

        // create as the type's create method, its parent given by the parent columns
        const input = type.table[TABLE].encode(Row.parse(values));
        const parent =
            type.parent?.object === "any" && input["parentId"] !== undefined
                ? {
                      parent: {
                          packageId: input["parentPackageId"],
                          type: input["parentType"],
                          id: input["parentId"],
                      },
                  }
                : {};
        const creating = new MethodContext<MethodConfiguration, unknown, undefined, undefined>({
            ...this.#fields,
            object: type,
            name: "create",
            method: type.method("create"),
            input: { ...input, ...parent },
            id: undefined,
            target: undefined,
        });

        // require the receiving parent and the creation, then write the row and record its access
        await RecordMethod.authorizeCreate(creating, location);
        const { row } = await RecordMethod.insertAuthorized(creating, location, (created) =>
            RecordMethod.createObject(created, location),
        );

        return Table.complete(type.table, Row.parse(row));
    }

    /** Update the target's desired state at its revision, advancing a controlled generation. */
    async update<Recorded extends Table>(
        this: MethodContext<{ readonly table: Recorded }, unknown>,
        changes: Readonly<Partial<InsertValue<Recorded>>>,
    ): Promise<Select<Recorded>> {
        return Table.complete(this.object.table, await this.#write(this.revised(changes)));
    }

    /** Stamp changes with the next revision, time and author, advancing a controlled generation. */
    revised(
        this: MethodContext<{ readonly table: Table }, unknown>,
        changes: Readonly<Record<string, unknown>>,
    ): Row {
        // advance the generation of a controlled target
        const target: Row = this.target;
        const generation = this.object.isControlled
            ? { generation: GENERATION.parse(target["generation"]) + 1 }
            : {};

        return this.#stamped({ ...changes, ...generation });
    }

    /** Stamp changes with the target's next revision, the call's time and its author. */
    #stamped(
        this: MethodContext<{ readonly table: Table }, unknown>,
        changes: Readonly<Record<string, unknown>>,
    ): Row {
        // stamp the next revision, and the time and author where the table records them
        const target = RecordedRow.parse(this.target);
        const columns = this.object.table[TABLE].columns;

        return {
            ...changes,
            revision: target.revision + 1,
            ...(Object.hasOwn(columns, "updatedAt") ? { updatedAt: this.now } : {}),
            ...(Object.hasOwn(columns, "updatedById")
                ? ObjectReference.columns("updatedBy", this.caller.subject)
                : {}),
        };
    }

    /** Update the target's observed state at the loaded revision and generation. */
    async updateStatus<Recorded extends Table>(
        this: MethodContext<{ readonly table: Recorded }, unknown>,
        changes: Readonly<Partial<InsertValue<Recorded>>>,
    ): Promise<Select<Recorded>> {
        return Table.complete(this.object.table, await this.#write(this.#stamped(changes)));
    }

    /** Delete the target at the loaded revision. */
    async remove(this: MethodContext<{ readonly table: Table }, unknown>): Promise<void> {
        // remove the loaded target through the mutation, whose commit refuses a target changed since
        const table: Table = this.object.table;
        const target = RecordedRow.parse(this.target);
        const location = this.#location;
        if (location.kind === "store") {
            await Execution.write(location, this, {
                table,
                before: await this.#current(location.transaction),
                after: null,
            });

            return;
        }

        // delete only the revision a prediction loaded
        const deleted = await this.#replica
            .delete(table)
            .where(RecordedRow.at(table, target))
            .returning();
        if (deleted.length === 0) {
            throw ObjectError.of("REVISION_STALE", `${this.object.name} revision has changed`);
        }
    }

    /** Read the target's row after the call's writes, refusing a target removed since. */
    async #current(
        this: MethodContext<{ readonly table: Table }, unknown>,
        transaction: Transaction,
    ): Promise<Row> {
        // refuse a target the transaction removed
        const { id } = RecordedRow.parse(this.target);
        const table: Table = this.object.table;
        const row = await transaction.read(table).findFirst({ where: { id } });
        if (row === undefined) {
            throw ObjectError.of("REMOVED", `${this.object.name} ${id} is removed`);
        }

        return row;
    }

    /** Write stamped changes to the target at the loaded revision. */
    async #write(
        this: MethodContext<{ readonly table: Table }, unknown>,
        stamped: Row,
    ): Promise<Row> {
        // write the stamped changes over the target
        const table: Table = this.object.table;
        const target = RecordedRow.parse(this.target);

        // write over the target as the transaction has it, whose commit refuses a target changed since
        if (this.#location.kind === "store") {
            const transaction = this.#location.transaction;
            const current = await this.#current(transaction);
            await transaction.write({
                table,
                before: current,
                after: Table.values(table, Row.parse({ ...current, ...stamped })),
            });

            return present(
                await transaction.read(table).findFirst({ where: { id: target.id } }),
                `the written ${this.object.name} ${target.id}`,
            );
        }

        // update at the revision a prediction loaded
        const [row] = await this.#replica
            .update(table)
            .set(Table.values(table, stamped))
            .where(RecordedRow.at(table, target))
            .returning();
        if (!row) {
            throw ObjectError.of("REVISION_STALE", `${this.object.name} revision has changed`);
        }

        return row;
    }

    /** The time of the call in UTC epoch milliseconds: its caller's. */
    get now(): number {
        return this.caller.now;
    }

    /** The principal the call's code runs as: the installation serving its type. */
    get self(): AccessContext {
        // act as the installation serving the type in the store
        const location = this.#location;
        if (location.kind === "store") {
            return location.store.as(this.object, this.now);
        }
        // act as the sandbox's installation
        else if (location.kind === "sandbox") {
            const { scope, installation } = location.server;
            const subject = principal.installation.reference(scope, installation);

            return AccessContext.of(Caller.of(subject), this.now);
        }

        throw new TypeError(`${this.object.name}.${this.name} can only act as itself in its store`);
    }

    /** Report whether stacks can declare the call's records, typing its target. */
    isDeclarable(): this is MethodContext<
        MethodConfiguration & { readonly table: ManagedTable },
        Prepared,
        Id,
        Select<ManagedTable> | undefined
    > {
        return this.object.isDeclarable();
    }

    /** Report whether the installation serving the call's type has a capability of its package. */
    enabled(capability: string): boolean {
        // read the capabilities of the installation serving the type, in its store or its sandbox
        const location = this.#location;
        const server =
            location.kind === "store"
                ? location.store.readServer(this.object)
                : location.kind === "sandbox"
                  ? location.server
                  : undefined;
        if (server === undefined) {
            throw new TypeError(
                `${this.object.name}.${this.name} does not run in an installation with capabilities`,
            );
        }

        return server.capabilities.includes(capability);
    }

    /** Act with the delegation the call's partition grants the installation running the call. */
    async delegated(): Promise<AccessContext> {
        // find the principal of the partition the installation lives in
        const installation = this.self.subject;
        const delegators = [principal.space, principal.machine, principal.device];
        const kind = delegators.find(
            (each) => schema.identifier(each.name).safeParse(installation.scope).success,
        );
        if (kind === undefined) {
            throw new TypeError(
                `${this.object.name}.${this.name} runs in ${installation.scope}, which delegates to no installation`,
            );
        }

        // act as the partition with the installation acting for it, within the partition's live grant
        const delegator = kind.reference(sync.Scope.universe.id, installation.scope);
        const reader = { read: <Read extends Table>(table: Read) => this.read(table) };
        const parties = { delegator, delegate: installation };
        const caller = await Grant.act(reader, parties, this.now);

        return AccessContext.of(caller, this.now);
    }

    /** The log position the call's mutation started at, refusing a prediction and a query. */
    get base(): LogPosition {
        return present(this.started, `the start position of ${this.object.name}.${this.name}`);
    }

    /** The log position the call's mutation started at, absent for a prediction and a query. */
    get started(): LogPosition | undefined {
        // read the transaction's start, else the start the hook's call record has
        const location = this.#location;
        if (location.kind === "store") {
            return location.transaction.isReading ? undefined : location.transaction.base;
        } else if (location.kind === "sandbox") {
            return location.base;
        }

        return undefined;
    }
}

/** The fields a method context is made of. */
export type MethodContextOptions<
    Configuration extends MethodConfiguration = MethodConfiguration,
    Prepared = PreparedOf<Configuration>,
    Id extends string | undefined = IdOf<Configuration>,
    Target extends Select<TableOf<Configuration>> | undefined = TargetOf<Configuration>,
> = Pick<
    MethodContext<Configuration, Prepared, Id, Target>,
    | "object"
    | "name"
    | "method"
    | "scope"
    | "chain"
    | "input"
    | "id"
    | "target"
    | "caller"
    | "objects"
    | "replicas"
    | "prepared"
    | "idempotencyKey"
    | "writerId"
    | "requestId"
    | "run"
    | "expansion"
> & {
    /** Where the call runs: executed by a store or a client, or hooked in a sandbox. */
    readonly location: CallLocation;
};

/** Where a call runs: executed by a store or a client, or hooked in a sandbox. */
export type CallLocation =
    | Execution
    | {
          /** An installation's sandbox running the call's hook for its store. */
          readonly kind: "sandbox";
          /** The hook's session with the store in its mutation. */
          readonly session: Session;
          /** The server context of the installation the sandbox runs as. */
          readonly server: InstalledServerContext;
          /** The log position the call's mutation started at, absent for a query and a prepare. */
          readonly base?: LogPosition;
      };

/** The results of calls in order. */
export type ResultsOf<Calls extends readonly sync.Call[]> = {
    -readonly [Index in keyof Calls]: Calls[Index] extends sync.Call<infer Result> ? Result : never;
};

/** Run a call in the calling method's transaction and scope. */
export type Run = (
    entry: sync.Call,
    caller: AccessContext,
    mutates: boolean,
    site: CallSite,
) => Promise<unknown>;

/** The input a call reads: the output of its method's input schema, a JSON object without one. */
export type InputOf<Input extends schema.Schema> = [Input] extends [never]
    ? JsonObject
    : schema.Schema extends Input
      ? JsonObject
      : schema.Output<Input> & JsonObject;

/** A method's object identifier: present for a creation and for a method acting on one object. */
type IdOf<Configuration extends MethodConfiguration> =
    OptionOf<Configuration, "target", "object"> extends "object" | "id"
        ? string
        : OptionOf<Configuration, "kind", MethodKind> extends "create"
          ? string
          : string | undefined;

/** The table of a method's object type. */
type TableOf<Configuration extends MethodConfiguration> = OptionOf<Configuration, "table", Table>;

/** A method's input: its fields, the written columns, and a creation's parent and variant. */
type InputValueOf<Configuration extends MethodConfiguration> = InputOf<
    OptionOf<Configuration, "input", never>
> &
    (OptionOf<Configuration, "kind", MethodKind> extends "create" | "update"
        ? WrittenInput<
              TableOf<Configuration>,
              OptionOf<Configuration, "fields", string>,
              OptionOf<Configuration, "kind", MethodKind> extends "update" ? true : false,
              OptionOf<Configuration, "filled", never>
          >
        : {}) &
    (OptionOf<Configuration, "kind", MethodKind> extends "create"
        ? ParentInput<TableOf<Configuration>> &
              VariantInput<TableOf<Configuration>, OptionOf<Configuration, "variants", never>>
        : {}) &
    BulkInput<Configuration>;

/** A bulk method's input: the matched columns, else each listed object at its revision. */
type BulkInput<Configuration extends MethodConfiguration> =
    OptionOf<Configuration, "kind", MethodKind> extends "updateMany"
        ? OptionOf<Configuration, "items", false> extends true
            ? {
                  readonly items: readonly ({
                      readonly id: string;
                      readonly revision: number;
                  } & WrittenInput<
                      TableOf<Configuration>,
                      OptionOf<Configuration, "fields", string>,
                      true
                  >)[];
              }
            : MatchInput<Configuration> &
                  WrittenInput<
                      TableOf<Configuration>,
                      OptionOf<Configuration, "fields", string>,
                      true
                  >
        : OptionOf<Configuration, "kind", MethodKind> extends "deleteMany"
          ? MatchInput<Configuration>
          : {};

/** A bulk method's matched columns, under `where`. */
type MatchInput<Configuration extends MethodConfiguration> = {
    readonly where: WrittenInput<
        TableOf<Configuration>,
        OptionOf<Configuration, "match", string>,
        true
    >;
};

/** A method's target: the loaded row for a method loading its object, else absent. */
type TargetOf<Configuration extends MethodConfiguration> =
    OptionOf<Configuration, "target", "object"> extends "object"
        ? Select<TableOf<Configuration>>
        : Select<TableOf<Configuration>> | undefined;

/** The external work a method prepares, as its schema reads it, absent without one. */
export type PreparedOf<Configuration extends MethodConfiguration> = [
    OptionOf<Configuration, "prepared", never>,
] extends [never]
    ? undefined
    : schema.Schema extends OptionOf<Configuration, "prepared", never>
      ? unknown
      : schema.Output<OptionOf<Configuration, "prepared", never>>;

/** What a method returns in process: its output, a listing's page, else the changed row. */
export type MethodResult<Configuration extends MethodConfiguration> = [
    OptionOf<Configuration, "output", never>,
] extends [never]
    ? OptionOf<Configuration, "kind", MethodKind> extends "list"
        ? Awaited<ReturnType<Listing["list"]>>
        : Select<TableOf<Configuration>>
    : schema.Output<OptionOf<Configuration, "output", never>>;

/** A creation's parent: a reference for several types, else an identifier, if any. */
type ParentInput<Definition extends Table> =
    Select<Definition> extends { readonly parentId: infer Parent }
        ? Select<Definition> extends { readonly parentType: unknown }
            ? null extends Parent
                ? { readonly parent?: ParentReference }
                : { readonly parent: ParentReference }
            : null extends Parent
              ? { readonly parentId?: NonNullable<Parent> }
              : { readonly parentId: Parent }
        : {};

/** The written fields of a creation or update in JSON column form, optional for updates. */
type WrittenInput<
    Definition extends Table,
    Fields extends string,
    IsPartial extends boolean,
    Filled extends string = never,
    Written = Pick<Insert<Definition>, Extract<Fields, keyof Insert<Definition>>>,
> = IsPartial extends true
    ? { readonly [Name in keyof Written]?: JsonOf<Exclude<Written[Name], undefined>> }
    : {
          readonly [Name in keyof Written as Name extends Filled ? never : Name]: JsonOf<
              Written[Name]
          >;
      } & {
          readonly [Name in keyof Written as Name extends Filled ? Name : never]?: JsonOf<
              Exclude<Written[Name], undefined>
          >;
      };

/** A creation of a sum type: one variant's columns under its discriminating field's value. */
type VariantInput<Definition extends Table, Sum> =
    IsSum<Sum> extends true
        ? {
              [Field in keyof Sum & string]: {
                  [Value in keyof Sum[Field] & string]: {
                      readonly [Name in Field]: Value;
                  } & RequiredInput<
                      Definition,
                      Sum[Field][Value] extends readonly (infer Written extends string)[]
                          ? Written
                          : never
                  >;
              }[keyof Sum[Field] & string];
          }[keyof Sum & string]
        : {};

/** The columns a write gives a value in their JSON form, null refused. */
type RequiredInput<
    Definition extends Table,
    Fields extends string,
    Written = Pick<Insert<Definition>, Extract<Fields, keyof Insert<Definition>>>,
> = { readonly [Name in keyof Written]-?: JsonOf<NonNullable<Written[Name]>> };

/** An object type as its methods' types read it: its table and its methods by name. */
type MethodOwner = { readonly table: Table; readonly methods: Readonly<Record<string, Method>> };

/** The types a declared method has on its object type's table, read from its members. */
export type TypesOf<Declaration extends Method, Definition extends Table> = {
    readonly kind: Declaration["kind"];
    readonly permission: Declaration["permission"];
    readonly input: Exclude<Declaration["input"], undefined>;
    readonly output: Exclude<Declaration["output"], undefined>;
    readonly prepared: Exclude<Declaration["prepared"], undefined>;
    readonly mutates: Declaration["mutates"];
    readonly target: NonNullable<Declaration["target"]>;
    readonly fields: Declaration extends {
        readonly fields?: readonly (infer Fields extends string)[];
    }
        ? Fields
        : string;
    readonly match: Declaration extends {
        readonly match?: readonly (infer Match extends string)[];
    }
        ? Match
        : string;
    readonly items: Declaration extends { readonly isItemized?: infer Itemized extends boolean }
        ? boolean extends Itemized
            ? false
            : Itemized
        : false;
    readonly variants: Exclude<Declaration["variants"], undefined>;
    readonly table: Definition;
};

/** What one named method of an object type returns in process. */
export type ResultOf<Type extends MethodOwner, Name extends keyof Type["methods"]> = MethodResult<
    TypesOf<Type["methods"][Name], Type["table"]>
>;

/** The next behaviour of one named method of an object type, which a handler wraps. */
export type NextOf<Type extends MethodOwner, Name extends keyof Type["methods"]> = (
    context?: MethodContext,
) => Promise<ResultOf<Type, Name>>;

/** The context of one named object type method, as its handle phase sees it by default. */
export type MethodContextOf<
    Type extends MethodOwner,
    Name extends keyof Type["methods"],
    Prepared = PreparedOf<TypesOf<Type["methods"][Name], Type["table"]>>,
> = MethodContext<TypesOf<Type["methods"][Name], Type["table"]>, Prepared>;

/** A method's code: its handler, or its phases, with a prepare phase for prepared work. */
export type Handler<Configuration extends MethodConfiguration = MethodConfiguration> = [
    OptionOf<Configuration, "prepared", never>,
] extends [never]
    ? HandleFunction<Configuration> | MethodHooks<Configuration>
    : schema.Schema extends OptionOf<Configuration, "prepared", never>
      ? HandleFunction<Configuration> | MethodHooks<Configuration>
      : MethodHooks<Configuration> & {
            /** Do external work before the transaction, returning `call.prepared`. */
            readonly prepare: NonNullable<MethodHooks<Configuration>["prepare"]>;
        };

/** A method's handling phase: change rows in the transaction, wrapping next. */
export type HandleFunction<Configuration extends MethodConfiguration = MethodConfiguration> = (
    context: MethodContext<Configuration>,
    next: (context?: MethodContext) => Promise<MethodResult<Configuration>>,
) => Promise<MethodResult<Configuration>>;

/** The hooks a type attaches to one of its methods, by phase. */
export type MethodHooks<Configuration extends MethodConfiguration = MethodConfiguration> = {
    /** Authorize the call before its external work. */
    readonly authorize?: (
        context: MethodContext<Configuration, PreparedOf<Configuration> | undefined>,
    ) => Promise<void>;
    /** Derive a created object's first relationships and a new scope's owner, for creations. */
    readonly creation?: (
        context: MethodContext<Configuration, PreparedOf<Configuration> | undefined>,
        object: ObjectReference,
    ) => Creation | Promise<Creation>;
    /** List the calls a mutation runs first as their callers, read before the transaction. */
    readonly expand?: (
        context: MethodContext<Configuration, PreparedOf<Configuration> | undefined>,
    ) => Promise<readonly BranchEntry[]>;
    /** Do external work before the transaction, returning `call.prepared`. */
    readonly prepare?: (
        context: MethodContext<Configuration, PreparedOf<Configuration> | undefined>,
    ) => Promise<PreparedOf<Configuration>>;
    /** Change rows inside the transaction, wrapping next. */
    readonly handle?: HandleFunction<Configuration>;
    /** Settle the prepared work once the transaction committed or failed, at least once. */
    readonly settle?: (
        context: SettlementContext,
        settled: Settled<PreparedOf<Configuration>>,
    ) => Promise<void>;
};
