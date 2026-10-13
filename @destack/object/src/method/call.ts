import { ObjectReference, type ObjectTypeReference, Subject } from "@destack/sync";
import { Caller, type Creation, type TupleReader, type Permission } from "@destack/access";
import {
    and,
    eq,
    type DatabaseConnection,
    type Insert,
    type InsertValue,
    type JsonOf,
    Row,
    type Select,
    TABLE,
    type RowChange,
    type Table,
    Snapshot,
    Expression,
    Key,
    type Transaction,
} from "@destack/db";
import type { BranchCall } from "../branch/branch.ts";
import { Identifier, present, schema, Version, type JsonObject } from "@destack/schema";
import type { PackageId } from "@destack/package";
import { isCallError, CallError, TRANSIENT_STATUSES } from "@destack/runtime/error";
import type { Address, Destination } from "@destack/runtime/outbox";
import { RequestId } from "@destack/runtime/request";
import { RunRequest } from "@destack/runtime/trigger";
import { Exchange } from "@destack/runtime/authentication";
import * as sync from "@destack/sync";
import type { ObjectOf, ObjectType } from "../object/object.ts";
import { Observation, type ObservationReport, type ObservedRecord } from "../object/condition.ts";
import type { ObjectTable } from "../object/table.ts";
import type { Authorization } from "../server/authorization.ts";
import type { ObjectStore } from "../server/store.ts";
import type { OptionOf, Method, MethodConfiguration } from "./method.ts";
import type { MethodKind, TargetKind } from "./kind.ts";
import { InputSchema } from "./input.ts";
import type { InstallationContext } from "@destack/runtime/workload";
import type { Settled, SettlementCall } from "./settlement.ts";
import type { IsSum, listObjects } from "./record.ts";
import { ParentReference } from "../trait/nested.ts";
import { ObjectReader, type SessionReader } from "./reader.ts";

/** The values a draw takes: the integers a double holds exactly, 2^53. */
const RANDOM_RANGE = 2 ** 53;

/** The bits a draw drops from a 64-bit word to keep 53. */
const RANDOM_SHIFT = 11n;

/** A call's input as JSON: a record of JSON values. */
const JSON_INPUT = schema.record(schema.string(), schema.json());
/** A call a method sent, as its outbox keeps it until the machine records its run. */
const OutboxMessage = schema.object({
    /** The request recording the run once, however often the outbox delivers it. */
    requestId: RequestId.schema,
    /** The sent call. */
    request: RunRequest,
});

/** The sent calls one delivery sends: 100 small calls at once. */
const SEND_BATCH = 100;

/** The calls methods send through the outbox. */
export const OutboxCall = {
    name: "sends",
    message: OutboxMessage,

    /** Deliver the sent calls in batches, reporting and dropping one refused for good. */
    destination(
        deliver: (sent: OutboxCall, signal: AbortSignal) => Promise<void>,
        report: (error: unknown) => void,
    ): Destination<OutboxCall> {
        return {
            name: OutboxCall.name,
            message: OutboxMessage,
            batch: SEND_BATCH,
            accept: async (sent, { signal }) => {
                // send the whole batch at once
                const delivered = await Promise.allSettled(
                    sent.map((entry) => deliver(entry, signal)),
                );

                // drop a call refused for good, and retry the batch after any other failure
                const failures = delivered.flatMap((result): Error[] =>
                    result.status === "rejected" ? [sendFailure(result.reason)] : [],
                );
                for (const refused of failures.filter(isRefusal)) {
                    report(new Error("a sent call was refused for good", { cause: refused }));
                }
                const failed = failures.find((failure) => !isRefusal(failure));
                if (failed !== undefined) {
                    throw failed;
                }
            },
        };
    },
} satisfies Address<schema.Infer<typeof OutboxMessage>> & { readonly destination: object };
/** A call a method sent. */
export type OutboxCall = schema.Infer<typeof OutboxMessage>;

/** A method result identifying an object by its identifier. */
const IDENTIFIED = schema.looseObject({ id: schema.string() });

/** A controlled target's generation, which the controlled trait keeps beside the record columns. */
const GENERATION = schema.number().int();

/** One call of a method inside its transaction, served or predicted. */
export class Call<Definition extends Table = Table, Input extends schema.Schema = schema.Schema> {
    /** The object type. */
    readonly object: ObjectOf<{ table: Definition }>;
    /** The method's name. */
    readonly name: string;
    /** The method. */
    readonly method: Method;
    /** The scope containing the object. */
    readonly scope: string;
    /** The scope and the scopes containing it, nearest first. */
    readonly chain: readonly string[];
    /** The method's own input fields. */
    readonly input: InputOf<Input>;
    /** The target's or created object's identifier. */
    readonly id?: string;
    /** The target at the start of the call. */
    readonly target?: Select<Definition>;
    /** The caller: who calls and what it presented, anonymous without a subject. */
    readonly caller: Caller;
    /** The time of the call in UTC epoch milliseconds. */
    readonly now: number;
    /** Where the call runs: predicted on a client replica, or served by an authorized store. */
    readonly location: CallLocation;
    /** The object types served together. */
    readonly objects: readonly ObjectType[];
    /** The value the prepare phase produced. */
    readonly prepared?: unknown;
    /** The idempotency key of the call's external work. */
    readonly idempotencyKey?: string;
    /** The client instance writing an ephemeral object. */
    readonly writerId?: string;
    /** The request a durable mutation replays under. */
    readonly requestId?: string;
    /** Run another method in the call's transaction as its caller. */
    readonly run?: Run;
    /** The calls the method's expansion ran before it in its mutation. */
    readonly expansion?: readonly BranchCall[];
    /** The fields the call was made of, which copies change. */
    readonly #fields: CallFields<Definition, Input>;

    /** Keep one call's fields. */
    constructor(fields: CallFields<Definition, Input>) {
        // take the required fields
        this.#fields = fields;
        this.object = fields.object;
        this.name = fields.name;
        this.method = fields.method;
        this.scope = fields.scope;
        this.chain = fields.chain;
        this.input = fields.input;
        this.now = fields.now;
        this.objects = fields.objects;
        this.location = fields.location;

        // take the present optional fields
        if (fields.id !== undefined) {
            this.id = fields.id;
        }
        if (fields.target !== undefined) {
            this.target = fields.target;
        }
        this.caller = fields.caller;
        if (fields.prepared !== undefined) {
            this.prepared = fields.prepared;
        }
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

    /** Resolve a recorded call against the served types, converting input to the served release. */
    static resolve(
        objects: readonly ObjectType[],
        entry: ObjectCall,
        mutates: boolean,
        code: CallCode,
    ): {
        readonly object: ObjectType;
        readonly name: string;
        readonly input: JsonObject;
    } {
        // find the object type and a method the calling code may call
        const object = Call.typeOf(objects, entry);
        const name = entry.method;
        const method = object?.methods?.[name];
        if (
            object === undefined ||
            method === undefined ||
            !CallCode.reaches(code, object, method) ||
            (mutates && !method.mutates)
        ) {
            throw new CallError("BAD_REQUEST", {
                message: `no ${mutates ? "mutating " : ""}method ${sync.Call.label(entry)}`,
            });
        } else if (!mutates && method.mutates) {
            throw new CallError("BAD_REQUEST", {
                message: `${sync.Call.label(entry)} changes objects, so a client pushes it`,
            });
        }

        // convert the input to the served release as JSON, which the method's input schema validates when the call runs
        const fields = InputSchema.fields(object.input(name)).shape;
        const parsed = JSON_INPUT.safeParse(Call.upgrade(object, name, entry, fields));
        if (!parsed.success) {
            throw new CallError("BAD_REQUEST", {
                message: `the input to ${sync.Call.label(entry)} is not JSON`,
            });
        }

        return { object, name, input: parsed.data };
    }

    /** Find the type of a call's method among some types. */
    static typeOf(
        types: readonly ObjectType[],
        call: Pick<ObjectCall, "type">,
    ): ObjectType | undefined {
        return types.find(
            (type) =>
                type.typeReference.packageId === call.type.packageId &&
                type.typeReference.type === call.type.type,
        );
    }

    /** Record a call of an object's method, against the release of the object's package. */
    static record(
        object: ObjectType,
        name: string,
        input: Readonly<Record<string, unknown>>,
    ): sync.Call {
        return Call.wire({
            type: object.typeReference,
            method: name,
            input,
            release: object.package.version,
        });
    }

    /** Encode a call for travel with JSON input, such as a typed call sent to another store. */
    static wire(call: ObjectCall): sync.Call {
        const input = schema.record(schema.string(), schema.json()).parse(call.input);

        return { type: call.type, method: call.method, input, release: call.release };
    }

    /** Convert a recorded call's input to the object's release, dropping undeclared fields. */
    static upgrade(
        object: ObjectType,
        name: string,
        call: ObjectCall,
        shape?: object,
    ): Readonly<Record<string, unknown>> {
        // refuse a call of a later release
        const served = object.package.version;
        if (Version.compare(call.release, served) > 0) {
            throw new CallError("BAD_REQUEST", {
                message: `${sync.Call.label(call)} was made against release ${call.release} of ${object.name}, which is at ${served}`,
            });
        }

        // keep a call of this release as it is
        const conversions = object.conversions(name);
        if (Version.between(conversions, call.release, served).length === 0) {
            return call.input;
        }

        // assign each later release's fields and drop the fields this release no longer declares
        const converted = Expression.upgrade(
            conversions,
            schema.record(schema.string(), schema.json()).parse(call.input),
            call.release,
            served,
        );

        return shape === undefined
            ? converted
            : Object.fromEntries(Object.entries(converted).filter(([field]) => field in shape));
    }

    /** Read the identifier of the object a method returned. */
    static resultId(result: unknown): string | undefined {
        const parsed = IDENTIFIED.safeParse(result);

        return parsed.success ? parsed.data.id : undefined;
    }

    /** Run calls in this call's transaction as a caller, returning each result in order. */
    async mutate<const Calls extends readonly ObjectCall[]>(
        caller: Caller,
        calls: Calls,
    ): Promise<ResultsOf<Calls>> {
        const results: unknown[] = [];
        for (const entry of calls) {
            results.push(await this.#run(caller, entry, true, CallCode.store));
        }

        return Call.#results(calls, results);
    }

    /** Generate a new identifier from the call's request, stable across attempts in order. */
    generateId(object: ObjectType): Identifier<string> {
        return object.deriveId(this.#requireRequestId(), this.requireTransaction().takePosition());
    }

    /** Draw a number in [0, 1) from the call's request, stable across attempts in order. */
    async random(): Promise<number> {
        // read 53 bits of the digest of the request and the transaction's next position
        const key = `${this.#requireRequestId()}/${this.requireTransaction().takePosition()}`;
        const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(key));
        const bits = new DataView(digest).getBigUint64(0) >> RANDOM_SHIFT;

        return Number(bits) / RANDOM_RANGE;
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
            Call.observation(object, row, observation),
        ]);

        return written;
    }

    /** Describe the call recording an observation of a row at its current generation. */
    static observation(
        object: ObjectType,
        row: ObservedRecord & Readonly<Record<string, unknown>>,
        observation: ObservationReport,
    ): ObjectCall {
        return Call.record(object, "observe", {
            ...object.target({ scope: row.scope, id: row.id }),
            observedGeneration: row.generation,
            conditions: observation.conditions,
            ...(observation.fields === undefined ? {} : { fields: observation.fields }),
        });
    }

    /** Read through a method in this call's transaction as a caller. */
    async query<Result>(caller: Caller, call: ObjectCall<Result>): Promise<Result> {
        const [result] = Call.#results(
            [call],
            [await this.#run(caller, call, false, CallCode.store)],
        );

        return result;
    }

    /** Run a call a package's sandboxed hook sent in this call's transaction. */
    runSandboxed(
        caller: Caller,
        entry: ObjectCall,
        mutates: boolean,
        packageId: PackageId,
    ): Promise<unknown> {
        return this.#run(caller, entry, mutates, { kind: "sandbox", packageId });
    }

    /** Run one platform call in this call's transaction and scope, refusing it without a runner. */
    async #run(
        caller: Caller,
        entry: ObjectCall,
        mutates: boolean,
        code: CallCode,
    ): Promise<unknown> {
        if (this.run === undefined) {
            throw new TypeError(`${this.object.name}.${this.name} runs no nested calls`);
        }

        return this.run(entry, caller, mutates, code);
    }

    /** Type the results of calls by the results they answer. */
    static #results<const Calls extends readonly ObjectCall[]>(
        _calls: Calls,
        results: unknown[],
    ): ResultsOf<Calls>;
    /**
     * Type the results of calls.
     *
     * @construct each result is its method's answer, which ObjectCall keeps from type.call.
     */
    static #results(_calls: readonly ObjectCall[], results: unknown[]): unknown[] {
        return results;
    }

    /** Copy the call with some fields changed, as a call of any input once the input changes. */
    with(changes: Partial<Omit<CallFields<Definition, Input>, "input">>): Call<Definition, Input>;
    with(changes: Partial<CallFields<Definition>>): Call<Definition>;
    /**
     * Copy the call with some fields changed.
     *
     * @construct a copy keeping the input keeps its input type.
     */
    with(changes: Partial<CallFields<Definition>>): Call<Definition> {
        return new Call({ ...this.#fields, ...changes });
    }

    /** The client copy a prediction reads and writes, refusing a call a store or a sandbox runs. */
    get #copy(): DatabaseConnection {
        if (this.location.kind !== "client") {
            throw new TypeError(`${this.object.name}.${this.name} reads through call.read`);
        }

        return this.location.database;
    }

    /** Whether a client predicts the call, which it runs without the server's authorization. */
    get isPredicted(): boolean {
        return this.location.kind === "client";
    }

    /** The call's view at a position or over a branch, the live database if absent. */
    get snapshot(): Snapshot | undefined {
        return this.location.kind === "server" ? this.location.snapshot : undefined;
    }

    /** Read a table's or object type's rows in the call's view. */
    read<Read extends Table>(source: Read | { readonly table: Read }): ObjectReader<Read> {
        // read through the transaction on a server without a view of its own, and through the store in a hook
        const table = TABLE in source ? source : source.table;
        const location = this.location;
        if (location.kind === "server" && location.snapshot === undefined) {
            return new ObjectReader(table, location.transaction);
        } else if (location.kind === "sandbox") {
            return new ObjectReader(table, location.session);
        }

        return new ObjectReader(table, this.view());
    }

    /** Read the call's view: as requested, else the server transaction's, else the client copy. */
    view(): Snapshot {
        const location = this.location;
        if (location.kind === "server") {
            return location.snapshot ?? location.transaction.view();
        } else if (location.kind === "sandbox") {
            throw new TypeError(`${this.object.name}.${this.name} reads through call.read`);
        }

        return Snapshot.live(this.#copy);
    }

    /** Write rows of any tables through the call: its server transaction, or the client copy. */
    async write(...changes: readonly RowChange[]): Promise<void> {
        // write through the transaction on a server
        if (this.location.kind === "server") {
            await this.location.transaction.write(...changes);
            return;
        }

        // remove and put each row in the client's copy, refusing a change of no row
        for (const { table, before, after } of changes) {
            if (after !== null) {
                await this.#copy.upsert(table, [after]);
            } else if (before !== null) {
                await this.#copy.delete(table).where(Key.match(table, before));
            } else {
                throw new TypeError(
                    `a write to ${table[TABLE].name} has neither a row before nor after`,
                );
            }
        }
    }

    /** Read the transaction a server call writes through, refusing a prediction and a hook. */
    requireTransaction(): Transaction {
        if (this.location.kind !== "server") {
            throw new TypeError(`${this.object.name}.${this.name} runs outside a transaction`);
        }

        return this.location.transaction;
    }

    /** Read the server's authorization, refusing a prediction and a hook. */
    requireAuthorization(): Authorization {
        if (this.location.kind !== "server") {
            throw new CallError("FORBIDDEN", {
                message: `${this.object.name} is authorized in its store`,
            });
        }

        return this.location.authorization;
    }

    /** Read the subject calling, refusing a call without one. */
    requireCaller(): Subject {
        const subject = this.caller.subject;
        if (subject === undefined) {
            throw new CallError("FORBIDDEN", {
                message: `${this.object.name} needs a calling principal`,
            });
        }

        return subject;
    }

    /** Read the permission the method requires of its caller. */
    permission(): Permission {
        return this.object.permission(this.method.permission);
    }

    /** Reference the object the call acts on, in the call's scope. */
    reference(): ObjectReference {
        return this.object.reference(this.scope, this.requireId());
    }

    /** Read the identifier of the object the call acts on, which an instance call requires. */
    requireId(): string {
        if (this.id === undefined) {
            throw new TypeError(`${this.object.name}.${this.name} acts on no object`);
        }

        return this.id;
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
            throw new CallError("BAD_REQUEST", {
                message: `${this.object.name} needs a parent`,
            });
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
                : this.object.table[TABLE].decode({ parentId: input["parentId"] });
        }

        // write no parent, or the reference's fields
        const parent = this.parent();

        return parent === undefined
            ? { parentPackageId: null, parentType: null, parentId: null }
            : { parentPackageId: parent.packageId, parentType: parent.type, parentId: parent.id };
    }

    /** Require the receive permission on a parent. */
    async requireReceiving(parent: ObjectReference, reader?: TupleReader): Promise<void> {
        // require a nested object, and an attachment host type
        const declared = this.object.parent;
        if (declared === undefined) {
            throw new TypeError(`object ${this.object.name} is not nested in any parent`);
        }
        const authorization = this.requireAuthorization();
        const authorizer = authorization.authorizer;
        if (declared.object === "any") {
            const hosts = authorizer.relation(this.object.policy, "parent").subjects;
            if (
                !hosts.some(
                    (host) => host.packageId === parent.packageId && host.type === parent.type,
                )
            ) {
                throw new CallError("BAD_REQUEST", {
                    message: `${parent.type} takes no ${this.object.plural}`,
                });
            }

            // require the parent to exist
            const mapping = authorizer.mapping(parent);
            const [found] = await authorization.transaction.images(mapping.table, {
                [mapping.id]: parent.id,
                ...(mapping.scope === undefined ? {} : { [mapping.scope]: parent.scope }),
            });
            if (found === undefined) {
                throw new CallError("NOT_FOUND", { message: `no ${parent.type} ${parent.id}` });
            }
        }

        // accept a receiving parent
        const receive = authorizer.policy(parent).permission(declared.receive);
        if ((await authorization.check(receive, parent, reader)).isAllowed) {
            return;
        }

        // hide an unreadable parent, else require the permission
        const host =
            declared.object === "any"
                ? this.objects.find((object) => object.policy.is(parent))
                : declared.object;
        const reading = host?.reading;
        if (reading === undefined || !(await authorization.check(reading, parent)).isAllowed) {
            throw new CallError("NOT_FOUND", { message: `no ${parent.type} ${parent.id}` });
        }
        await authorization.require(receive, parent);
    }

    /** Send a call to run at its rows' home for the caller's subject once this call commits. */
    async send(request: { readonly call: sync.Call; readonly at?: number }): Promise<void> {
        // record the send once in the call's transaction, asking to run it for the caller's subject
        const exchange = Exchange.of(this.requireAuthorization().context());
        await this.requireStore().send(request.call, this.requireTransaction(), {
            ...(request.at === undefined ? {} : { at: request.at }),
            ...(exchange === undefined ? {} : { exchange }),
        });
    }

    /** Update the target's desired state at its revision, advancing a controlled generation. */
    async update<Recorded extends Definition & ObjectTable>(
        this: Call<Recorded>,
        changes: Readonly<Partial<Select<Recorded>>>,
    ): Promise<Select<Recorded>>;
    /**
     * Update the record the call targets.
     *
     * @construct the target is a record of this call's object, whose table the signature reads.
     */
    async update(
        this: Call<ObjectTable>,
        changes: Readonly<Partial<InsertValue<ObjectTable>>>,
    ): Promise<Select<ObjectTable>> {
        return this.#write(this.#revised(changes));
    }

    /** Stamp changes as an update writes them: next revision, time, author and generation. */
    revised<Recorded extends Definition & ObjectTable>(
        this: Call<Recorded>,
        changes: Readonly<Partial<Select<Recorded>>>,
    ): Partial<InsertValue<ObjectTable>>;
    /**
     * Stamp changes to the record the call targets.
     *
     * @construct the target is a record of this call's object, whose table the signature reads.
     */
    revised(
        this: Call<ObjectTable>,
        changes: Readonly<Partial<InsertValue<ObjectTable>>>,
    ): Partial<InsertValue<ObjectTable>> {
        return this.#revised(changes);
    }

    /** Stamp changes with the next revision, time and author, advancing a controlled generation. */
    #revised(
        this: Call<ObjectTable>,
        changes: Readonly<Partial<InsertValue<ObjectTable>>>,
    ): Partial<InsertValue<ObjectTable>> {
        // advance the generation of a controlled target
        const target: Row = this.requireTarget();
        const generation = this.object.isControlled
            ? { generation: GENERATION.parse(target["generation"]) + 1 }
            : {};

        return this.#stamped({ ...changes, ...generation });
    }

    /** Stamp changes with the target's next revision, the call's time and its author. */
    #stamped(
        this: Call<ObjectTable>,
        changes: Readonly<Partial<InsertValue<ObjectTable>>>,
    ): Partial<InsertValue<ObjectTable>> {
        // stamp the next revision, and the time and author where the table records them
        const target: Select<ObjectTable> = this.requireTarget();
        const author = this.caller.subject === undefined ? null : Subject.key(this.caller.subject);
        const columns = this.object.table[TABLE].columns;

        return {
            ...changes,
            revision: target.revision + 1,
            ...(Object.hasOwn(columns, "updatedAt") ? { updatedAt: this.now } : {}),
            ...(Object.hasOwn(columns, "updatedBy") ? { updatedBy: author } : {}),
        };
    }

    /** Update the target's observed state at the loaded revision and generation. */
    async updateStatus<Recorded extends Definition & ObjectTable>(
        this: Call<Recorded>,
        changes: Readonly<Partial<Select<Recorded>>>,
    ): Promise<Select<Recorded>>;
    /**
     * Update the observed state of the record the call targets.
     *
     * @construct the target is a controlled record of this object, whose table the signature reads.
     */
    async updateStatus(
        this: Call<ObjectTable>,
        changes: Readonly<Partial<InsertValue<ObjectTable>>>,
    ): Promise<Select<ObjectTable>> {
        return this.#write(this.#stamped(changes));
    }

    /** Delete the target at the loaded revision. */
    async remove<Recorded extends Definition & ObjectTable>(this: Call<Recorded>): Promise<void> {
        // remove the loaded target through the mutation, whose commit refuses a target changed since
        const table: ObjectTable = this.object.table;
        const target: Select<ObjectTable> = this.requireTarget();
        if (this.location.kind === "server") {
            const transaction = this.location.transaction;
            await transaction.write({
                table,
                before: await this.#current(transaction),
                after: null,
            });

            return;
        }

        // delete only the revision a prediction loaded
        const deleted = await this.#copy
            .delete(table)
            .where(and(eq(table.id, target.id), eq(table.revision, target.revision)))
            .returning({ id: table.id });
        if (deleted.length === 0) {
            throw new CallError("CONFLICT", {
                message: `${this.object.name} revision has changed`,
            });
        }
    }

    /** Read the target's row after the call's writes, refusing a target removed since. */
    async #current(this: Call<ObjectTable>, transaction: Transaction): Promise<Row> {
        // refuse a target the transaction removed
        const id = this.requireTarget().id;
        const row = await transaction.row(this.object.table, { id });
        if (row === null) {
            throw new CallError("CONFLICT", { message: `${this.object.name} ${id} is removed` });
        }

        return row;
    }

    /** Write stamped changes to the target at the loaded revision. */
    async #write(
        this: Call<ObjectTable>,
        stamped: Readonly<Partial<InsertValue<ObjectTable>>>,
    ): Promise<Select<ObjectTable>> {
        // write the stamped changes over the target
        const base: ObjectTable = this.object.table;
        const target: Select<ObjectTable> = this.requireTarget();

        // write over the target as the transaction has it, whose commit refuses a target changed since
        if (this.location.kind === "server") {
            const transaction = this.location.transaction;
            const current = await this.#current(transaction);
            await transaction.write({
                table: base,
                before: current,
                after: base[TABLE].values(Row.parse({ ...current, ...stamped })),
            });

            return present(
                await transaction.row(base, { id: target.id }),
                `the written ${this.object.name} ${target.id}`,
            );
        }

        // update at the revision a prediction loaded
        const [row] = await this.#copy
            .update(base)
            .set(stamped)
            .where(and(eq(base.id, target.id), eq(base.revision, target.revision)))
            .returning();
        if (!row) {
            throw new CallError("CONFLICT", {
                message: `${this.object.name} revision has changed`,
            });
        }

        return row;
    }

    /** The store's principal acting as itself, such as its space or installation. */
    get self(): Caller {
        return Caller.of(this.requireStore().principal, this.now);
    }

    /** Admit a caller in a scope inside the mutation, as its principal's code acts there. */
    admit(caller: Caller, scope: string): Promise<Authorization> {
        return this.requireStore().admit(this.requireTransaction(), scope, caller, {
            now: this.now,
        });
    }

    /** Read the installation the call runs as, refusing a prediction and a call outside a space. */
    requireInstallation(): InstallationContext {
        if (this.location.kind === "client" || this.location.installation === undefined) {
            throw new TypeError(`${this.object.name}.${this.name} runs in no installation`);
        }

        return this.location.installation;
    }

    /** Read the store running the call, refusing a prediction and a hook no store runs. */
    requireStore(): ObjectStore {
        if (this.location.kind !== "server") {
            throw new TypeError(`${this.object.name}.${this.name} runs in no store`);
        }

        return this.location.store;
    }

    /** Read the target an instance call loaded, which a record write requires. */
    requireTarget(): Select<Definition> {
        if (this.target === undefined) {
            throw new TypeError(`${this.object.name}.${this.name} has no target to write`);
        }

        return this.target;
    }
}

/** The fields of a call. */
export type CallFields<
    Definition extends Table = Table,
    Input extends schema.Schema = schema.Schema,
> = Pick<
    Call<Definition, Input>,
    | "object"
    | "name"
    | "method"
    | "scope"
    | "chain"
    | "input"
    | "id"
    | "target"
    | "caller"
    | "now"
    | "location"
    | "objects"
    | "prepared"
    | "idempotencyKey"
    | "writerId"
    | "requestId"
    | "run"
    | "expansion"
>;

/** Where a call runs: predicted on a client, served by a store, or hooked in a sandbox. */
export type CallLocation =
    | {
          /** A client predicting the call. */
          readonly kind: "client";
          /** The client's copy the prediction reads and writes. */
          readonly database: DatabaseConnection;
      }
    | {
          /** A store serving the call. */
          readonly kind: "server";
          /** The server's authorization of the call's caller. */
          readonly authorization: Authorization;
          /** The store running the call, which runs further calls in their own transactions. */
          readonly store: ObjectStore;
          /** The installation the store serves as, absent outside a space. */
          readonly installation?: InstallationContext;
          /** The transaction of the call's reads and writes, committed after its calls ran. */
          readonly transaction: Transaction;
          /** The call's view at a position or over a branch, the mutation's if absent. */
          readonly snapshot?: Snapshot;
      }
    | {
          /** An installation's sandbox running the call's hook for its store. */
          readonly kind: "sandbox";
          /** The reads of the store the hook makes in its mutation's session. */
          readonly session: SessionReader;
          /** The installation the sandbox runs as. */
          readonly installation: InstallationContext;
      };

/** A call of one object type method as a value: method, input, release and result. */
export interface ObjectCall<Result = unknown> {
    /** The package-qualified object type. */
    readonly type: ObjectTypeReference;
    /** The method's name, such as create. */
    readonly method: string;
    /** The method's input with its scope and target, as the method's input schema reads it. */
    readonly input: Readonly<Record<string, unknown>>;
    /** The release of the method's package the call was made against. */
    readonly release: string;
    /** The result the call answers, carried by its type alone. */
    readonly result?: Result;
}

/** The results of calls in order. */
export type ResultsOf<Calls extends readonly ObjectCall[]> = {
    -readonly [Index in keyof Calls]: Calls[Index] extends ObjectCall<infer Result>
        ? Result
        : never;
};

/** Run a call in the calling method's transaction and scope. */
export type Run = (
    entry: ObjectCall,
    caller: Caller,
    mutates: boolean,
    code: CallCode,
) => Promise<unknown>;

/** The code making a call: the store's own, a client's, or a package's sandboxed hooks. */
export type CallCode =
    | { readonly kind: "store" }
    | { readonly kind: "client" }
    | { readonly kind: "sandbox"; readonly packageId: PackageId };

/** The codes making calls, and the methods each reaches. */
export const CallCode = {
    /** The store's own code, which reaches every method. */
    store: { kind: "store" } as const satisfies CallCode,

    /** A client's code, which reaches public methods. */
    client: { kind: "client" } as const satisfies CallCode,

    /** Read the code of a caller: a client's for a remote caller, else the store's. */
    of(caller: Caller): CallCode {
        return Caller.isRemote(caller) ? CallCode.client : CallCode.store;
    },

    /** Decide whether code reaches a method, internal ones the store's and their package's. */
    reaches(code: CallCode, object: ObjectType, method: Method): boolean {
        return (
            method.isInternal !== true ||
            code.kind === "store" ||
            (code.kind === "sandbox" && code.packageId === object.package.id)
        );
    },
};

/** The input a call reads: the output of its method's input schema, a JSON object without one. */
export type InputOf<Input extends schema.Schema> = [Input] extends [never]
    ? JsonObject
    : schema.Schema extends Input
      ? JsonObject
      : schema.Output<Input> & JsonObject;

/** A call as a method handler sees it: typed input, and the target loaded when targeted. */
type MethodCall<Configuration extends Partial<MethodConfiguration>> = Call<
    TableOf<Configuration>,
    OptionOf<Configuration, "input", never>
> &
    (OptionOf<Configuration, "kind", MethodKind> extends "create" | "update"
        ? {
              readonly input: WrittenInput<
                  TableOf<Configuration>,
                  OptionOf<Configuration, "fields", string>,
                  OptionOf<Configuration, "kind", MethodKind> extends "update" ? true : false
              >;
          }
        : {}) &
    (OptionOf<Configuration, "kind", MethodKind> extends "create"
        ? { readonly input: ParentInput<TableOf<Configuration>> } & VariantCall<
              TableOf<Configuration>,
              OptionOf<Configuration, "variants", never>
          >
        : {}) &
    (OptionOf<Configuration, "kind", MethodKind> extends TargetKind
        ? OptionOf<Configuration, "scoped", false> extends true
            ? {}
            : { readonly target: Select<TableOf<Configuration>> }
        : {});

/** A call in its transaction, with any external work its method prepared. */
type PreparedCall<Configuration extends Partial<MethodConfiguration>> = MethodCall<Configuration> &
    ([OptionOf<Configuration, "prepared", never>] extends [never]
        ? {}
        : schema.Schema extends OptionOf<Configuration, "prepared", never>
          ? {}
          : { readonly prepared: PreparedOf<Configuration> });

/** The table of a method's object type. */
type TableOf<Configuration extends Partial<MethodConfiguration>> = OptionOf<
    Configuration,
    "table",
    Table
>;

/** The external work a method prepares, as its schema reads it. */
type PreparedOf<Configuration extends Partial<MethodConfiguration>> = schema.Output<
    OptionOf<Configuration, "prepared", never>
>;

/** What a method returns in process: its output, a listing's page, else the changed row. */
type MethodResult<Configuration extends Partial<MethodConfiguration>> = [
    OptionOf<Configuration, "output", never>,
] extends [never]
    ? OptionOf<Configuration, "kind", MethodKind> extends "list"
        ? Awaited<ReturnType<typeof listObjects>>
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
    Written = Pick<Insert<Definition>, Extract<Fields, keyof Insert<Definition>>>,
> = IsPartial extends true
    ? { readonly [Name in keyof Written]?: JsonOf<Exclude<Written[Name], undefined>> }
    : { readonly [Name in keyof Written]: JsonOf<Written[Name]> };

/** A creation of a sum type: one variant's columns under its discriminating field's value. */
type VariantCall<Definition extends Table, Sum> =
    IsSum<Sum> extends true
        ? {
              readonly input: {
                  [Field in keyof Sum & string]: {
                      [Value in keyof Sum[Field] & string]: {
                          readonly [Key in Field]: Value;
                      } & RequiredInput<
                          Definition,
                          Sum[Field][Value] extends readonly (infer Written extends string)[]
                              ? Written
                              : never
                      >;
                  }[keyof Sum[Field] & string];
              }[keyof Sum & string];
          }
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
    readonly fields: Declaration extends {
        readonly fields?: readonly (infer Fields extends string)[];
    }
        ? Fields
        : string;
    readonly variants: Exclude<Declaration["variants"], undefined>;
    readonly table: Definition;
};

/** What one named method of an object type returns in process. */
export type ResultOf<Type extends MethodOwner, Name extends keyof Type["methods"]> = MethodResult<
    TypesOf<Type["methods"][Name], Type["table"]>
>;

/** The next behaviour of one named method of an object type, which a handler wraps. */
export type NextOf<Type extends MethodOwner, Name extends keyof Type["methods"]> = (
    call?: Call,
) => Promise<ResultOf<Type, Name>>;

/** The call of one named object type method, as its handlers see it before the transaction. */
export type CallOf<Type extends MethodOwner, Name extends keyof Type["methods"]> = MethodCall<
    TypesOf<Type["methods"][Name], Type["table"]>
>;

/** The call of one named object type method in its transaction, with its prepared work. */
export type PreparedCallOf<
    Type extends MethodOwner,
    Name extends keyof Type["methods"],
> = PreparedCall<TypesOf<Type["methods"][Name], Type["table"]>>;

/** A method's code: its handler, or its phases, with a prepare phase for prepared work. */
export type Handler<Configuration extends Partial<MethodConfiguration> = MethodConfiguration> = [
    OptionOf<Configuration, "prepared", never>,
] extends [never]
    ? HandleFunction<Configuration> | PhasedHandler<Configuration>
    : schema.Schema extends OptionOf<Configuration, "prepared", never>
      ? HandleFunction<Configuration> | PhasedHandler<Configuration>
      : PhasedHandler<Configuration>;

/** A method's handling phase: change rows in the transaction, wrapping next. */
export type HandleFunction<
    Configuration extends Partial<MethodConfiguration> = MethodConfiguration,
> = Bivariant<
    (
        call: PreparedCall<Configuration>,
        next: (call?: Call) => Promise<MethodResult<Configuration>>,
    ) => Promise<MethodResult<Configuration>>
>;

/** A method's code by phase, preparing the work its method declares. */
export type PhasedHandler<
    Configuration extends Partial<MethodConfiguration> = MethodConfiguration,
> = {
    /** Authorize the call before its external work. */
    readonly authorize?: Bivariant<(call: MethodCall<Configuration>) => Promise<void>>;
    /** Derive a created object's first relationships and a new scope's owner, for creations. */
    readonly creation?: Bivariant<
        (call: MethodCall<Configuration>, object: ObjectReference) => Creation | Promise<Creation>
    >;
    /** List the calls a mutation runs first as their callers, read before the transaction. */
    readonly expand?: Bivariant<
        (call: MethodCall<Configuration>) => Promise<readonly BranchCall[]>
    >;
    /** Change rows inside the transaction, wrapping next. */
    readonly handle?: HandleFunction<Configuration>;
} & ([OptionOf<Configuration, "prepared", never>] extends [never]
    ? {}
    : schema.Schema extends OptionOf<Configuration, "prepared", never>
      ? {
            /** Do external work before the transaction, returning `call.prepared`. */
            readonly prepare?: Bivariant<(call: MethodCall<Configuration>) => Promise<unknown>>;
            /** Settle the prepared work once the transaction committed or failed, at least once. */
            readonly settle?: Bivariant<(call: SettlementCall, settled: Settled) => Promise<void>>;
        }
      : {
            /** Do external work before the transaction, returning `call.prepared`. */
            readonly prepare: Bivariant<
                (call: MethodCall<Configuration>) => Promise<PreparedOf<Configuration>>
            >;
            /** Settle the prepared work once the transaction committed or failed, at least once. */
            readonly settle?: Bivariant<
                (call: SettlementCall, settled: Settled<PreparedOf<Configuration>>) => Promise<void>
            >;
        });

/** A callback taking any call of its method as a method would, typed by its declared call. */
export type Bivariant<Callback extends (...parameters: never[]) => unknown> = {
    /** Run the callback. */
    run(...parameters: Parameters<Callback>): ReturnType<Callback>;
}["run"];

/** Report whether a failure is the machine's final refusal: a client error, apart from timeouts. */
function isRefusal(error: Error): boolean {
    return (
        isCallError(error) &&
        error.status >= 400 &&
        error.status < 500 &&
        !TRANSIENT_STATUSES.has(error.status)
    );
}

/** Read a failed send's reason as an error. */
function sendFailure(reason: unknown): Error {
    return reason instanceof Error ? reason : new Error("a sent call failed", { cause: reason });
}
