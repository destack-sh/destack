import { ObjectReference, type ObjectTypeReference, Subject } from "@destack/sync";
import {
    Caller,
    type Creation,
    type Decision,
    type Permission,
    type PermissionReference,
    principal,
    type SignedCaller,
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
    Expression,
    type LogPosition,
    type RelationalQueryBuilder,
    type Transaction,
    Table,
} from "@destack/db";
import type { EventKeyShape, EventKind } from "@destack/event";
import type { BranchCall } from "../branch/branch.ts";
import {
    found,
    Identifier,
    present,
    schema,
    Version,
    type JsonObject,
    type JsonValue,
} from "@destack/schema";
import type { PackageId } from "@destack/package";
import { isCallError, CallError, TRANSIENT_STATUSES } from "@destack/runtime/error";
import type { Address, Destination } from "@destack/runtime/outbox";
import { RequestId } from "@destack/runtime/request";
import { RunRequest } from "@destack/runtime/trigger";
import { Exchange } from "@destack/runtime/authentication";
import * as sync from "@destack/sync";
import type { ObjectOf, ObjectType } from "../object/object.ts";
import { Observation, type ObservationReport, type ObservedRecord } from "../object/condition.ts";
import type { TraitTable } from "../object/table.ts";
import type { DeclarableDefinition } from "../trait/declarable.ts";
import type { RecoverableDefinition } from "../trait/recoverable.ts";
import type { OptionOf, Method, MethodConfiguration } from "./method.ts";
import type { MethodKind } from "./kind.ts";
import { InputSchema } from "./input.ts";
import type { ServerContext } from "@destack/runtime/workload";
import type { Settled, SettlementCall } from "./settlement.ts";
import type { IsSum } from "./record.ts";
import type { Listing } from "../query/listing.ts";
import { ParentReference } from "../trait/nested.ts";
import type { Session } from "./reader.ts";
import { EventView } from "../query/event.ts";
import { Execution } from "./execution.ts";
import { BranchWrite } from "../branch/write.ts";
import { Replay } from "../audit/replay.ts";

/** The values of a draw: the integers a double represents exactly, 2^53. */
const RANDOM_RANGE = 2 ** 53;

/** The bits a draw drops from a 64-bit word to keep 53. */
const RANDOM_SHIFT = 11n;

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
                    result.status === "rejected" ? [OutboxCall.sendFailure(result.reason)] : [],
                );
                for (const refused of failures.filter((failure) => OutboxCall.isRefusal(failure))) {
                    report(new Error("a sent call was refused permanently", { cause: refused }));
                }
                const failed = failures.find((failure) => !OutboxCall.isRefusal(failure));
                if (failed !== undefined) {
                    throw failed;
                }
            },
        };
    },

    /** Report whether a failure is the machine's final refusal: a client error but a timeout. */
    isRefusal(error: Error): boolean {
        return (
            isCallError(error) &&
            error.status >= 400 &&
            error.status < 500 &&
            !TRANSIENT_STATUSES.has(error.status)
        );
    },

    /** Read a failed send's reason as an error. */
    sendFailure(reason: unknown): Error {
        return reason instanceof Error
            ? reason
            : new Error("a sent call failed", { cause: reason });
    },
} satisfies Address<schema.Infer<typeof OutboxMessage>> & {
    readonly destination: object;
    readonly isRefusal: object;
    readonly sendFailure: object;
};
/** A call a method sent. */
export type OutboxCall = schema.Infer<typeof OutboxMessage>;

/** A method result identifying an object by its identifier. */
const IDENTIFIED = schema.looseObject({ id: schema.string() });

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

/** A controlled target's generation, which the controlled trait keeps beside the record columns. */
const GENERATION = schema.number().int();

/** One call of a method inside its transaction, served or predicted, typed by its method's kind. */
export class Call<
    Definition extends Table = Table,
    Input extends JsonObject = JsonObject,
    Target extends Select<Definition> | undefined = Select<Definition> | undefined,
    Signer extends Caller = Caller,
    Prepared = unknown,
> {
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
    /** The method's input, as its schema reads it. */
    readonly input: Input;
    /** The target's or created object's identifier. */
    readonly id?: string;
    /** The target at the start of the call, loaded for targeted methods. */
    readonly target: Target;
    /** The caller: who calls and what it presented, a signed one for mutations. */
    readonly caller: Signer;
    /** The time of the call in UTC epoch milliseconds. */
    readonly now: number;
    /** Where the call runs: predicted on a client replica, or served by an authorized store. */
    readonly #location: CallLocation;
    /** The object types served together. */
    readonly objects: readonly ObjectType[];
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
    readonly expansion?: readonly BranchCall[];
    /** The fields the call was made of, which replicates change. */
    readonly #fields: CallFields<Definition, Input, Target, Signer, Prepared>;

    /** Keep one call's fields. */
    constructor(fields: CallFields<Definition, Input, Target, Signer, Prepared>) {
        // refuse a mutation without a represented subject, which its handlers read as signed
        if (fields.method.mutates && !Caller.isSigned(fields.caller)) {
            throw new CallError("UNAUTHORIZED", {
                message: "a mutation needs an authenticated caller",
            });
        }

        // take the required fields
        this.#fields = fields;
        this.object = fields.object;
        this.name = fields.name;
        this.method = fields.method;
        this.scope = fields.scope;
        this.chain = fields.chain;
        this.input = fields.input;
        this.target = fields.target;
        this.caller = fields.caller;
        this.prepared = fields.prepared;
        this.now = fields.now;
        this.objects = fields.objects;
        this.#location = fields.location;

        // take the present optional fields
        if (fields.id !== undefined) {
            this.id = fields.id;
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
        const object = Call.findType(objects, entry);
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
                message: `${sync.Call.label(entry)} changes objects: push it from a client`,
            });
        }

        // convert the input to the served release
        const fields = InputSchema.fields(object.input(name)).shape;

        return { object, name, input: Call.upgrade(object, name, entry, fields) };
    }

    /** Find the type of a call's method among some types. */
    static findType(
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
        return {
            type: object.typeReference,
            method: name,
            input: schema.jsonObject().parse(input),
            release: object.package.version,
        };
    }

    /** Convert a recorded call's input to the object's release, dropping undeclared fields. */
    static upgrade(object: ObjectType, name: string, call: ObjectCall, shape?: object): JsonObject {
        // refuse a call of a later release
        const served = object.package.version;
        if (Version.compare(call.release, served) > 0) {
            throw new CallError("BAD_REQUEST", {
                message: `${sync.Call.label(call)} was made against release ${call.release} of ${object.name}, but ${object.name} is at ${served}`,
            });
        }

        // keep a call of this release as it is
        const conversions = object.conversions(name);
        if (Version.between(conversions, call.release, served).length === 0) {
            return call.input;
        }

        // assign each later release's fields and drop the fields this release no longer declares
        const converted = Expression.upgrade(conversions, call.input, call.release, served);

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

        return Call.results(this.objects, calls, results);
    }

    /** Generate a new identifier from the call's request, stable across attempts in order. */
    async generateId(object: ObjectType): Promise<Identifier<string>> {
        return object.deriveId(this.#requireRequestId(), await this.takePosition());
    }

    /** Draw a number in [0, 1) from the call's request, stable across attempts in order. */
    async random(): Promise<number> {
        // read 53 bits of the digest of the request and the transaction's next position
        const key = `${this.#requireRequestId()}/${await this.takePosition()}`;
        const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(key));
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

        throw new TypeError(`${this.object.name}.${this.name} derives identifiers in its store`);
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
        const [result] = Call.results(
            this.objects,
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
            throw new TypeError(`${this.object.name}.${this.name} cannot run nested calls`);
        }

        return this.run(entry, caller, mutates, code);
    }

    /** Read the answers of calls as their methods' results, parsed by their output schemas. */
    static results<const Calls extends readonly ObjectCall[]>(
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
        calls: readonly ObjectCall[],
        answers: readonly unknown[],
    ): unknown[] {
        return calls.map((entry, index) => {
            // parse the answer by the served method's output schema
            const object = present(Call.findType(objects, entry), sync.Call.label(entry));
            const output = schema.standard().parse(object.procedure(entry.method).output);
            const validated = output["~standard"].validate(answers[index]);
            if (validated instanceof Promise) {
                throw new TypeError(
                    `the output schema of ${sync.Call.label(entry)} is asynchronous`,
                );
            } else if (validated.issues !== undefined) {
                throw new CallError("INTERNAL_SERVER_ERROR", {
                    message: `invalid output of ${sync.Call.label(entry)}`,
                    data: { issues: validated.issues },
                });
            }

            return validated.value;
        });
    }

    /** Copy the call with some fields changed, keeping its types. */
    with(
        changes: Partial<CallFields<Definition, Input, Target, Signer, Prepared>>,
    ): Call<Definition, Input, Target, Signer, Prepared> {
        return new Call({ ...this.#fields, ...changes });
    }

    /** The client replica a prediction reads and writes, refusing a store's or a sandbox's call. */
    get #replica(): DatabaseConnection {
        if (this.#location.kind !== "client") {
            throw new TypeError(`${this.object.name}.${this.name} must read through call.read`);
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

    /** Read the whole rows of a type of the call's own package, sensitive columns included. */
    reveal<Read extends Table>(type: {
        readonly table: Read;
        readonly name: string;
        readonly package: { readonly id: string };
    }): RelationalQueryBuilder<Read, Select<Read>> {
        // refuse another package's type, by the packages declaring them
        if (type.package.id !== this.object.package.id) {
            throw new CallError("FORBIDDEN", {
                message: `${this.object.package.name} reveals ${type.name} only in its own package`,
            });
        }

        // read through the store's transaction, else through the hook's session
        const location = this.#location;
        if (location.kind === "store") {
            return location.transaction.read(type.table);
        } else if (location.kind === "sandbox") {
            return location.session.reveal(type.table);
        }

        // refuse a prediction, whose replica keeps no sensitive column
        throw new TypeError(`${this.object.name}.${this.name} reveals rows in its store`);
    }

    /** Read the events of an own package's event kind, refusing any other kind. */
    events<Shape extends EventKeyShape, Data extends JsonValue>(
        kind: EventKind<Shape, Data>,
    ): EventView<Shape, Data> {
        // refuse another package's kind
        if (kind.package.id !== this.object.package.id) {
            throw new CallError("FORBIDDEN", {
                message: `${this.object.package.name} reads ${kind.name} events only in its own package`,
            });
        }

        // read through the store's event log, else through the hook's session
        const location = this.#location;
        if (location.kind === "store") {
            return new EventView(kind, location.store.events);
        } else if (location.kind === "sandbox") {
            return new EventView(kind, location.session);
        }

        // refuse a prediction, whose replica keeps no event log
        throw new TypeError(`${this.object.name}.${this.name} reads events in its store`);
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
        calls: readonly BranchCall[],
    ): Promise<readonly BranchWrite[]> {
        // rehearse through the hook's session, and refuse a prediction
        const location = this.#location;
        if (location.kind === "sandbox") {
            return location.session.rehearse(over, calls);
        } else if (location.kind === "client") {
            throw new TypeError(`${this.object.name}.${this.name} rehearses calls in its store`);
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
                await Call.#attempt(transaction, () =>
                    this.#run(Caller.of(author, this.now), entry, true, CallCode.client),
                );
            }
        });

        return BranchWrite.capture(changes);
    }

    /** Run a rehearsed call in its own savepoint, skipping a refusal the store makes for good. */
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
        return new CallError("FORBIDDEN", {
            message: `${this.object.name} is authorized in its store`,
        });
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
            throw new TypeError(`${this.object.name}.${this.name} has no object`);
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
                : Table.decode(this.object.table, { parentId: input["parentId"] });
        }

        // write no parent, or the reference's fields
        const parent = this.parent();

        return parent === undefined
            ? { parentPackageId: null, parentType: null, parentId: null }
            : { parentPackageId: parent.packageId, parentType: parent.type, parentId: parent.id };
    }

    /** Send a call to run at its rows' home for the caller's subject once this call commits. */
    async send(request: { readonly call: sync.Call; readonly at?: number }): Promise<void> {
        // send through the hook's session, and refuse a prediction
        const location = this.#location;
        if (location.kind === "sandbox") {
            await location.session.send(request);
            return;
        } else if (location.kind === "client") {
            throw new TypeError(`${this.object.name}.${this.name} sends calls from its store`);
        }

        // record the send once in the call's transaction, asking to run it for the caller's subject
        const exchange = Exchange.of(location.authorization.context());
        await location.store.send(request.call, location.transaction, {
            ...(request.at === undefined ? {} : { at: request.at }),
            ...(exchange === undefined ? {} : { exchange }),
        });
    }

    /** Update the target's desired state at its revision, advancing a controlled generation. */
    async update<Recorded extends Table>(
        this: Call<Recorded>,
        changes: Readonly<Partial<InsertValue<Recorded>>>,
    ): Promise<Select<Recorded>> {
        return Table.complete(this.object.table, await this.#write(this.revised(changes)));
    }

    /** Stamp changes with the next revision, time and author, advancing a controlled generation. */
    revised(changes: Readonly<Record<string, unknown>>): Row {
        // advance the generation of a controlled target
        const target: Row = this.requireTarget();
        const generation = this.object.isControlled
            ? { generation: GENERATION.parse(target["generation"]) + 1 }
            : {};

        return this.#stamped({ ...changes, ...generation });
    }

    /** Stamp changes with the target's next revision, the call's time and its author. */
    #stamped(changes: Readonly<Record<string, unknown>>): Row {
        // stamp the next revision, and the time and author where the table records them
        const target = RecordedRow.parse(this.requireTarget());
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
        this: Call<Recorded>,
        changes: Readonly<Partial<InsertValue<Recorded>>>,
    ): Promise<Select<Recorded>> {
        return Table.complete(this.object.table, await this.#write(this.#stamped(changes)));
    }

    /** Delete the target at the loaded revision. */
    async remove(): Promise<void> {
        // remove the loaded target through the mutation, whose commit refuses a target changed since
        const table: Table = this.object.table;
        const target = RecordedRow.parse(this.requireTarget());
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
            throw new CallError("CONFLICT", {
                message: `${this.object.name} revision has changed`,
            });
        }
    }

    /** Read the target's row after the call's writes, refusing a target removed since. */
    async #current(transaction: Transaction): Promise<Row> {
        // refuse a target the transaction removed
        const { id } = RecordedRow.parse(this.requireTarget());
        const table: Table = this.object.table;
        const row = await transaction.read(table).findFirst({ where: { id } });
        if (row === undefined) {
            throw new CallError("CONFLICT", { message: `${this.object.name} ${id} is removed` });
        }

        return row;
    }

    /** Write stamped changes to the target at the loaded revision. */
    async #write(stamped: Row): Promise<Row> {
        // write the stamped changes over the target
        const table: Table = this.object.table;
        const target = RecordedRow.parse(this.requireTarget());

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
            throw new CallError("CONFLICT", {
                message: `${this.object.name} revision has changed`,
            });
        }

        return row;
    }

    /** The principal the call's code runs as: its store's, or its sandbox's installation. */
    get self(): Caller {
        // act as the store's principal, else as the sandbox's installation
        const location = this.#location;
        if (location.kind === "store") {
            return Caller.of(location.store.principal, this.now);
        } else if (location.kind === "sandbox") {
            const { space, installation } = location.server;

            return Caller.of(principal.installation.reference(space, installation), this.now);
        }

        throw new TypeError(`${this.object.name}.${this.name} acts as itself only in its store`);
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

    /** Decide whether the call's caller represents a subject, as a mutation's does. */
    isSigned(): this is MutationCall<Definition> {
        return Caller.isSigned(this.caller);
    }

    /** Read the call as a mutation's, refusing a caller without a subject. */
    requireSigned(): MutationCall<Definition> {
        if (!this.isSigned()) {
            throw new CallError("UNAUTHORIZED", {
                message: "a mutation needs an authenticated caller",
            });
        }

        return this;
    }

    /** Decide whether the call's object deletes to a trash. */
    isTrashable(): this is Call<TraitTable<{ readonly recoverable: RecoverableDefinition }>> {
        return this.object.isTrashable();
    }

    /** Read the call as one of a type deleting to a trash, refusing another type. */
    requireTrashable(): Call<TraitTable<{ readonly recoverable: RecoverableDefinition }>> {
        if (!this.isTrashable()) {
            throw new TypeError(`${this.object.name} deletes no objects to a trash`);
        }

        return this;
    }

    /** Decide whether the call's object keeps purged records. */
    isKept(): this is Call<
        TraitTable<{ readonly recoverable: RecoverableDefinition & { readonly keep: "record" } }>
    > {
        return this.object.isKept();
    }

    /** Read the call as one of a type keeping purged records, refusing another type. */
    requireKept(): Call<
        TraitTable<{ readonly recoverable: RecoverableDefinition & { readonly keep: "record" } }>
    > {
        if (!this.isKept()) {
            throw new TypeError(`${this.object.name} keeps no purged records`);
        }

        return this;
    }

    /** Decide whether stacks declare the call's object. */
    isManaged(): this is Call<TraitTable<{ readonly declarable: DeclarableDefinition }>> {
        return this.object.isManaged();
    }

    /** Read the call as one of a type stacks declare, refusing another type. */
    requireManaged(): Call<TraitTable<{ readonly declarable: DeclarableDefinition }>> {
        if (!this.isManaged()) {
            throw new TypeError(`${this.object.name} has no stack declarations`);
        }

        return this;
    }

    /** Read the target an instance call loaded, which a record write requires. */
    requireTarget(): Select<Definition> {
        if (this.target === undefined) {
            throw new TypeError(`${this.object.name}.${this.name} has no target to write`);
        }

        return this.target;
    }
}

/** A call of a mutating method, whose caller represents a subject. */
export type MutationCall<
    Definition extends Table = Table,
    Input extends JsonObject = JsonObject,
> = Call<Definition, Input, Select<Definition> | undefined, SignedCaller>;

/** The fields of a call. */
export type CallFields<
    Definition extends Table = Table,
    Input extends JsonObject = JsonObject,
    Target extends Select<Definition> | undefined = Select<Definition> | undefined,
    Signer extends Caller = Caller,
    Prepared = unknown,
> = Pick<
    Call<Definition, Input, Target, Signer, Prepared>,
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
    | "objects"
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
          readonly server: ServerContext;
          /** The log position the call's mutation started at, absent for a query and a prepare. */
          readonly base?: LogPosition;
      };

/** A call of one object type method as data, typed by the result its method answers. */
export type ObjectCall<Result = unknown> = sync.Call & {
    /** The result the call answers, as a type only. */
    readonly result?: Result;
};

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

/** The codes making calls, and the methods each can call. */
export const CallCode = {
    /** The store's own code, which can call every method. */
    store: { kind: "store" } as const satisfies CallCode,

    /** A client's code, which can call public methods. */
    client: { kind: "client" } as const satisfies CallCode,

    /** Read the code of a caller: a client's for a remote caller, else the store's. */
    of(caller: Caller): CallCode {
        return Caller.isRemote(caller) ? CallCode.client : CallCode.store;
    },

    /** Decide whether code can call a method: internal ones only from the store and the package. */
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

/** A call as a method's phases see it: typed input, target and caller, and its prepared work. */
export type MethodCall<
    Configuration extends MethodConfiguration,
    Prepared = PreparedOf<Configuration>,
> = Call<
    TableOf<Configuration>,
    InputValueOf<Configuration>,
    TargetOf<Configuration>,
    SignerOf<Configuration>,
    Prepared
>;

/** The table of a method's object type. */
type TableOf<Configuration extends MethodConfiguration> = OptionOf<Configuration, "table", Table>;

/** A method's input: its own fields, the written columns, and a creation's parent and variant. */
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
        : {});

/** A method's target: the loaded row for a method loading its object, else absent. */
type TargetOf<Configuration extends MethodConfiguration> =
    OptionOf<Configuration, "target", "object"> extends "object"
        ? Select<TableOf<Configuration>>
        : Select<TableOf<Configuration>> | undefined;

/** A method's caller: a signed one for mutations. */
type SignerOf<Configuration extends MethodConfiguration> =
    OptionOf<Configuration, "mutates", boolean> extends true ? SignedCaller : Caller;

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

/** The call of one named object type method, as its handle phase sees it by default. */
export type CallOf<
    Type extends MethodOwner,
    Name extends keyof Type["methods"],
    Prepared = PreparedOf<TypesOf<Type["methods"][Name], Type["table"]>>,
> = MethodCall<TypesOf<Type["methods"][Name], Type["table"]>, Prepared>;

/** A method's code: its handler, or its phases, with a prepare phase for prepared work. */
export type Handler<Configuration extends MethodConfiguration = MethodConfiguration> = [
    OptionOf<Configuration, "prepared", never>,
] extends [never]
    ? HandleFunction<Configuration> | PhasedHandler<Configuration>
    : schema.Schema extends OptionOf<Configuration, "prepared", never>
      ? HandleFunction<Configuration> | PhasedHandler<Configuration>
      : PhasedHandler<Configuration> & {
            /** Do external work before the transaction, returning `call.prepared`. */
            readonly prepare: NonNullable<PhasedHandler<Configuration>["prepare"]>;
        };

/** A method's handling phase: change rows in the transaction, wrapping next. */
export type HandleFunction<Configuration extends MethodConfiguration = MethodConfiguration> = (
    call: MethodCall<Configuration>,
    next: (call?: Call) => Promise<MethodResult<Configuration>>,
) => Promise<MethodResult<Configuration>>;

/** A method's code by phase, preparing the work its method declares. */
export type PhasedHandler<Configuration extends MethodConfiguration = MethodConfiguration> = {
    /** Authorize the call before its external work. */
    readonly authorize?: (
        call: MethodCall<Configuration, PreparedOf<Configuration> | undefined>,
    ) => Promise<void>;
    /** Derive a created object's first relationships and a new scope's owner, for creations. */
    readonly creation?: (
        call: MethodCall<Configuration, PreparedOf<Configuration> | undefined>,
        object: ObjectReference,
    ) => Creation | Promise<Creation>;
    /** List the calls a mutation runs first as their callers, read before the transaction. */
    readonly expand?: (
        call: MethodCall<Configuration, PreparedOf<Configuration> | undefined>,
    ) => Promise<readonly BranchCall[]>;
    /** Do external work before the transaction, returning `call.prepared`. */
    readonly prepare?: (
        call: MethodCall<Configuration, PreparedOf<Configuration> | undefined>,
    ) => Promise<PreparedOf<Configuration>>;
    /** Change rows inside the transaction, wrapping next. */
    readonly handle?: HandleFunction<Configuration>;
    /** Settle the prepared work once the transaction committed or failed, at least once. */
    readonly settle?: (
        call: SettlementCall,
        settled: Settled<PreparedOf<Configuration>>,
    ) => Promise<void>;
};
