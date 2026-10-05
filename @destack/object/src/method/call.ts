import { ObjectReference, Subject } from "@destack/sync";
import { type GrantReader, type Permission } from "@destack/access";
import {
    and,
    eq,
    type DatabaseConnection,
    type Insert,
    type InsertValue,
    type JsonOf,
    type Row,
    type Select,
    TABLE,
    type Table,
    type Snapshot,
    Expression,
} from "@destack/db";
import type { BranchCall } from "../branch/branch.ts";
import { schema, Version, type JsonObject } from "@destack/schema";
import { isServiceError, ServiceError, TRANSIENT_STATUSES } from "@destack/service/error";
import { type Address, type Destination, Outbox } from "@destack/service/outbox";
import { RequestId } from "@destack/service/request";
import { RunRequest } from "@destack/service/trigger";
import type * as sync from "@destack/sync";
import type { ObjectOf, ObjectType } from "../object/object.ts";
import type { ObjectTable } from "../object/table.ts";
import type { Authorization } from "../server/authorization.ts";
import type { OptionOf, Method, MethodConfiguration } from "./method.ts";
import type { MethodKind, TargetKind } from "./kind.ts";
import type { CallInput, MethodName } from "./procedure.ts";
import type { InstallationContext } from "@destack/service/workload";
import type { SettlementCall } from "./settlement.ts";
import type { listObjects } from "./record.ts";
import { ParentReference } from "../trait/nested.ts";

/** A call a method sent, as its outbox keeps it until the cell records its run. */
const OutboxMessage = schema.object({
    /** The request recording the run once, however often the outbox delivers it. */
    requestId: RequestId.schema,
    /** The sent call. */
    request: RunRequest,
});

/** The sent calls one delivery sends: 100 small calls at once. */
const SEND_BATCH = 100;

/** The calls methods send through the outbox, delivered to the server keeping the rows they change or to the cell recording their runs. */
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

/** A method result naming an object by its identifier. */
const IDENTIFIED = schema.looseObject({ id: schema.string() });

/** A controlled target's generation, which the controlled trait keeps beside the record columns. */
const GENERATION = schema.number().int();

/** The input schemas of methods as clients call them, by object type, method and whether pushed or read. */
const INPUTS = new WeakMap<ObjectType, Map<string, schema.JsonObject>>();

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
    /** The transaction: the server's, or the client's over its replica. */
    readonly database: DatabaseConnection;
    /** The calling principal. */
    readonly caller?: Subject;
    /** The time of the call in UTC epoch milliseconds. */
    readonly now: number;
    /** The server's authorization, absent exactly when a client predicts the call. */
    readonly authorization?: Authorization;
    /** The object types served together. */
    readonly objects: readonly ObjectType[];
    /** The value the prepare phase produced. */
    readonly prepared?: unknown;
    /** The idempotency key of the call's external work. */
    readonly idempotencyKey?: string;
    /** The client writing an ephemeral object. */
    readonly client?: string;
    /** The request a durable mutation replays under. */
    readonly requestId?: string;
    /** Run another method in the call's transaction as its caller. */
    readonly run?: Run;
    /** The outbox delivering this call's sends, absent where nothing receives them. */
    readonly sends?: Outbox;
    /** The view the call reads, as of a position or over a branch, the live database when absent. */
    readonly snapshot?: Snapshot;
    /** The calls the method's expansion ran before it in its mutation. */
    readonly expansion?: readonly BranchCall[];
    /** The installation the server serves as, absent outside a space and on clients. */
    readonly installation?: InstallationContext;
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
        this.database = fields.database;
        this.now = fields.now;
        this.objects = fields.objects;

        // take the present optional fields
        if (fields.id !== undefined) {
            this.id = fields.id;
        }
        if (fields.target !== undefined) {
            this.target = fields.target;
        }
        if (fields.caller !== undefined) {
            this.caller = fields.caller;
        }
        if (fields.authorization !== undefined) {
            this.authorization = fields.authorization;
        }
        if (fields.prepared !== undefined) {
            this.prepared = fields.prepared;
        }
        if (fields.idempotencyKey !== undefined) {
            this.idempotencyKey = fields.idempotencyKey;
        }
        if (fields.client !== undefined) {
            this.client = fields.client;
        }
        if (fields.requestId !== undefined) {
            this.requestId = fields.requestId;
        }
        if (fields.run !== undefined) {
            this.run = fields.run;
        }
        if (fields.sends !== undefined) {
            this.sends = fields.sends;
        }
        if (fields.snapshot !== undefined) {
            this.snapshot = fields.snapshot;
        }
        if (fields.expansion !== undefined) {
            this.expansion = fields.expansion;
        }
        if (fields.installation !== undefined) {
            this.installation = fields.installation;
        }
    }

    /** Resolve a recorded call against the served types, validating its input. */
    static resolve(
        objects: readonly ObjectType[],
        entry: sync.Call,
        mutates: boolean,
    ): {
        readonly object: ObjectType;
        readonly name: string;
        readonly input: JsonObject;
    } {
        // find the object type and a method clients may call
        const separator = entry.method.lastIndexOf(".");
        const object = objects.find((served) => served.name === entry.method.slice(0, separator));
        const name = entry.method.slice(separator + 1);
        const method = object?.methods?.[name];
        if (
            object === undefined ||
            method === undefined ||
            method.isSystem === true ||
            (mutates && !method.mutates)
        ) {
            throw new ServiceError("BAD_REQUEST", {
                message: `no ${mutates ? "mutating " : ""}method ${entry.method}`,
            });
        } else if (!mutates && method.mutates) {
            throw new ServiceError("BAD_REQUEST", {
                message: `${entry.method} changes objects, so a client pushes it`,
            });
        }

        // convert the input to the served release and validate it
        const input = Call.input(object, name, mutates);
        const parsed = input.safeParse(Call.upgrade(object, name, entry, input.shape));
        if (!parsed.success) {
            throw new ServiceError("BAD_REQUEST", {
                message: `invalid input to ${entry.method}`,
                data: { issues: parsed.error.issues },
            });
        }

        return { object, name, input: parsed.data };
    }

    /** Read a method's input schema as clients call it, without a pushed call's replay field. */
    static input(object: ObjectType, name: string, mutates: boolean): schema.JsonObject {
        // build each method's schema once per object type, which methods of traits share
        const key = `${name} ${String(mutates)}`;
        const built = INPUTS.get(object) ?? new Map<string, schema.JsonObject>();
        INPUTS.set(object, built);
        const kept = built.get(key);
        if (kept !== undefined) {
            return kept;
        }

        // leave out the field naming the request a pushed call replays under
        const procedure = object.method(name).procedure(name, object.schema).input;
        const input = mutates ? Call.#unreplayed(object, procedure) : procedure;
        built.set(key, input);

        return input;
    }

    /** Leave out the field a pushed call of an object type replays under, kept where a written field of its name replaces it. */
    static #unreplayed(object: ObjectType, procedure: schema.JsonObject): schema.JsonObject {
        const replay = object.schema.replay;
        const fields = Object.entries(procedure.shape).filter(
            ([field, value]) => replay[field] !== value,
        );

        return schema.object(Object.fromEntries(fields));
    }

    /** Record a call of an object's method, against the release of the object's package. */
    static record(object: ObjectType, name: string, input: JsonObject): sync.Call {
        return {
            method: `${object.name}.${name}`,
            input: schema.record(schema.string(), schema.json()).parse(input),
            release: object.package.version,
        };
    }

    /** Convert a recorded call's input to the object's release, dropping fields it no longer declares. */
    static upgrade(
        object: ObjectType,
        name: string,
        call: sync.Call,
        shape?: object,
    ): sync.Call["input"] {
        // refuse a call of a later release
        const served = object.package.version;
        if (Version.compare(call.release, served) > 0) {
            throw new ServiceError("BAD_REQUEST", {
                message: `${call.method} was made against release ${call.release} of ${object.name}, which is at ${served}`,
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

    /** Call another object type's methods in this call's scope and transaction, as this call's caller or another principal. */
    invoke<Object extends ObjectType>(
        object: Object,
        options: InvokeOptions = {},
    ): Invoker<Object> {
        return this.invoker(options)<Object>(object);
    }

    /** Call object types' methods through this call as one function, as this call's caller or another principal. */
    invoker(options: InvokeOptions = {}): Invoke {
        return invoking((object, name, input) => this.#invoke(object, name, input, options));
    }

    /** Change another object type's rows as this call's caller, in this call's scope unless the input names another: in this transaction where the database keeps them, else through a call sent toward their home once it commits. */
    async change(object: ObjectType, name: string, input: JsonObject): Promise<void> {
        // send a change of copied rows toward their home
        if (this.database.copies(object.table)) {
            const field = object.route.field;
            const scoped =
                field === undefined || Object.hasOwn(input, field)
                    ? input
                    : { ...input, [field]: this.scope };
            await this.send({ call: Call.record(object, name, scoped) });
        }
        // run any other change in this transaction
        else {
            await this.#invoke(object, name, input, {});
        }
    }

    /** Run another object's method without external work in the same storage. */
    async #invoke(
        object: ObjectType,
        name: string,
        input: JsonObject,
        options: InvokeOptions,
    ): Promise<unknown> {
        // require a method without external work in the same storage
        const method = object.methods[name];
        if (method === undefined) {
            throw new TypeError(`object ${object.name} has no method ${name}`);
        } else if (method.prepare !== undefined) {
            throw new TypeError(
                `${object.name}.${name} does external work, which an invoked call cannot`,
            );
        } else if (object.storage !== this.object.storage) {
            throw new TypeError(
                `${this.object.name}.${this.name} cannot invoke ${object.name}, stored apart`,
            );
        } else if (this.run === undefined) {
            throw new TypeError(`${this.object.name}.${this.name} runs no invoked calls`);
        }

        return this.run(object, name, input, options);
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

    /** Whether a client predicts the call, which it runs without the server's authorization. */
    get isPredicted(): boolean {
        return this.authorization === undefined;
    }

    /** Read the server's authorization, refusing a prediction. */
    requireAuthorization(): Authorization {
        if (this.authorization === undefined) {
            throw new ServiceError("FORBIDDEN", {
                message: `${this.object.name} changes on the server`,
            });
        }

        return this.authorization;
    }

    /** Read the principal calling, refusing a call without one. */
    requireCaller(): Subject {
        if (this.caller === undefined) {
            throw new ServiceError("FORBIDDEN", {
                message: `${this.object.name} needs a calling principal`,
            });
        }

        return this.caller;
    }

    /** Read the permission the method requires of its caller, which a system method has none of. */
    permission(): Permission {
        if (this.method.permission === null) {
            throw new TypeError(
                `${this.object.name}.${this.name} requires no permission of a caller`,
            );
        }

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

    /** Read the parent the call's input names. */
    parent(): ObjectReference | undefined {
        // read the named parent of a nested object
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

    /** Read the parent the call's input names, refusing a call without one. */
    requireParent(): ObjectReference {
        const parent = this.parent();
        if (parent === undefined) {
            throw new ServiceError("BAD_REQUEST", {
                message: `${this.object.name} needs a parent`,
            });
        }

        return parent;
    }

    /** Read the parent columns the call's input writes. */
    parentColumns(): Row {
        // write nothing for an object without a parent, and the typed parent's identifier as named
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
    async requireReceiving(parent: ObjectReference, reader?: GrantReader): Promise<void> {
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
                throw new ServiceError("BAD_REQUEST", {
                    message: `${parent.type} takes no ${this.object.plural}`,
                });
            }

            // require the parent to exist
            const mapping = authorizer.mapping(parent);
            const columns = mapping.table[TABLE];
            const [found] = await authorization.database
                .select({
                    id: columns.column(mapping.id),
                })
                .from(mapping.table)
                .where(
                    and(
                        eq(columns.column(mapping.id), parent.id),
                        mapping.scope === undefined
                            ? undefined
                            : eq(columns.column(mapping.scope), parent.scope),
                    ),
                );
            if (found === undefined) {
                throw new ServiceError("NOT_FOUND", { message: `no ${parent.type} ${parent.id}` });
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
            throw new ServiceError("NOT_FOUND", { message: `no ${parent.type} ${parent.id}` });
        }
        await authorization.require(receive, parent);
    }

    /** Send a call to run later, once this call's transaction commits. */
    async send(request: { readonly call: sync.Call; readonly at?: number }): Promise<void> {
        // require a served call on an object server delivering its sends
        this.requireAuthorization();
        if (this.sends === undefined) {
            throw new TypeError(
                `${this.method.kind} ${this.name} sends calls where nothing receives them`,
            );
        }

        // record the send once in the call's transaction, lending the caller's authority when delegated
        const requestId = RequestId.create();
        const delegation = this.authorization?.delegation;
        const message: OutboxCall = {
            requestId,
            request: { ...request, ...(delegation === undefined ? {} : { delegation }) },
        };
        await this.sends.append(OutboxCall, requestId, message, this.database);
    }

    /** Update the target's desired state at the loaded revision, advancing a controlled target's generation. */
    async update<Recorded extends Definition & ObjectTable>(
        this: Call<Recorded>,
        changes: Readonly<Partial<Select<Recorded>>>,
    ): Promise<Select<Recorded>>;
    /**
     * Update the record the call targets.
     *
     * @construct the target is a record of this call's object, whose table the signature above reads.
     */
    async update(
        this: Call<ObjectTable>,
        changes: Readonly<Partial<InsertValue<ObjectTable>>>,
    ): Promise<Select<ObjectTable>> {
        // advance the generation of a controlled target
        const target: Row = this.requireTarget();
        const generation = this.object.isControlled
            ? { generation: GENERATION.parse(target["generation"]) + 1 }
            : {};

        return this.#write({ ...changes, ...generation });
    }

    /** Update the target's observed state at the loaded revision and generation. */
    async updateStatus<Recorded extends Definition & ObjectTable>(
        this: Call<Recorded>,
        changes: Readonly<Partial<Select<Recorded>>>,
    ): Promise<Select<Recorded>>;
    /**
     * Update the observed state of the record the call targets.
     *
     * @construct the target is a controlled record of this call's object, whose table the signature above reads.
     */
    async updateStatus(
        this: Call<ObjectTable>,
        changes: Readonly<Partial<InsertValue<ObjectTable>>>,
    ): Promise<Select<ObjectTable>> {
        return this.#write(changes);
    }

    /** Delete the target at the loaded revision. */
    async remove<Recorded extends Definition & ObjectTable>(this: Call<Recorded>): Promise<void> {
        // delete only the revision the call loaded
        const table: ObjectTable = this.object.table;
        const target: Select<ObjectTable> = this.requireTarget();
        const deleted = await this.database
            .delete(table)
            .where(and(eq(table.id, target.id), eq(table.revision, target.revision)))
            .returning({ id: table.id });
        if (deleted.length === 0) {
            throw new ServiceError("CONFLICT", {
                message: `${this.object.name} revision has changed`,
            });
        }
    }

    /** Write changes to the target at the loaded revision. */
    async #write(
        this: Call<ObjectTable>,
        changes: Readonly<Partial<InsertValue<ObjectTable>>>,
    ): Promise<Select<ObjectTable>> {
        // stamp the changes with the next revision, which every derived table has
        const base: ObjectTable = this.object.table;
        const target: Select<ObjectTable> = this.requireTarget();
        const stamped: Partial<InsertValue<ObjectTable>> = {
            ...changes,
            revision: target.revision + 1,
            updatedAt: this.now,
            updatedBy: this.caller === undefined ? null : Subject.key(this.caller),
        };

        // update at the loaded revision
        const [row] = await this.database
            .update(base)
            .set(stamped)
            .where(and(eq(base.id, target.id), eq(base.revision, target.revision)))
            .returning();
        if (!row) {
            throw new ServiceError("CONFLICT", {
                message: `${this.object.name} revision has changed`,
            });
        }

        return row;
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
    | "database"
    | "caller"
    | "now"
    | "authorization"
    | "objects"
    | "prepared"
    | "idempotencyKey"
    | "client"
    | "requestId"
    | "run"
    | "sends"
    | "snapshot"
    | "expansion"
    | "installation"
>;

/** The methods of an object type a call invokes, each taking its input and resolving its result. */
export type Invoker<Object extends ObjectType> = {
    readonly [Name in keyof Object["methods"]]: {
        invoke(
            input: CallInput<Object, Name & MethodName<Object>>,
        ): Promise<ResultOf<Object, Name & MethodName<Object>>>;
    }["invoke"];
};

/** Call an object type's methods, as a call, a stack and the system invoke them. */
export type Invoke = <Object extends ObjectType>(object: Object) => Invoker<Object>;

/** Call object types' methods through one runner, as a call, a stack and the system invoke them. */
export function invoking(
    run: (object: ObjectType, name: string, input: JsonObject) => Promise<unknown>,
): Invoke;
/**
 * Call object types' methods through one runner.
 *
 * @construct each object type gets a function per method, each run with its declared input and result, which is how Invoke maps the object.
 */
export function invoking(
    run: (object: ObjectType, name: string, input: JsonObject) => Promise<unknown>,
): (object: ObjectType) => unknown {
    return (object) =>
        Object.fromEntries(
            Object.keys(object.methods).map((name) => [
                name,
                (input: JsonObject) => run(object, name, input),
            ]),
        );
}

/** How a call invokes another object's method. */
export interface InvokeOptions {
    /** The principal the call records as its caller, this call's caller when absent. */
    readonly as?: Subject;
}

/** Run a method of an object in a call's scope and transaction, as its caller or another principal. */
export type Run = (
    object: ObjectType,
    name: string,
    input: JsonObject,
    options: {
        /** The principal the call records as its caller, the invoking call's caller when absent. */
        readonly as?: Subject;
        /** Whose authority decides the call's permissions, the invoking call's when absent and the system's for branch previews. */
        readonly authority?: "caller" | "system";
    },
) => Promise<unknown>;

/** The input a call reads: the output of its method's input schema, a JSON object without one. */
export type InputOf<Input extends schema.Schema> = [Input] extends [never]
    ? JsonObject
    : schema.Schema extends Input
      ? JsonObject
      : schema.Output<Input> & JsonObject;

/** A call as a handler of one method sees it: its input typed with the fields it writes, and its target loaded for targeted kinds. */
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
        ? { readonly input: ParentInput<TableOf<Configuration>> }
        : {}) &
    (OptionOf<Configuration, "kind", MethodKind> extends TargetKind
        ? { readonly target: Select<TableOf<Configuration>> }
        : {});

/** A call inside its transaction, with the external work it prepared when its method declares some. */
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

/** What a method returns in process: its declared output, a page for a listing, else the changed object's stored row. */
type MethodResult<Configuration extends Partial<MethodConfiguration>> = [
    OptionOf<Configuration, "output", never>,
] extends [never]
    ? OptionOf<Configuration, "kind", MethodKind> extends "list"
        ? Awaited<ReturnType<typeof listObjects>>
        : Select<TableOf<Configuration>>
    : schema.Output<OptionOf<Configuration, "output", never>>;

/** The parent a creation names: a reference to a parent of several types, else its identifier, absent for an orphan. */
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

/** The written fields a creation or update passes in their columns' JSON form, each optional to an update. */
type WrittenInput<
    Definition extends Table,
    Fields extends string,
    IsPartial extends boolean,
    Written = Pick<Insert<Definition>, Extract<Fields, keyof Insert<Definition>>>,
> = IsPartial extends true
    ? { readonly [Name in keyof Written]?: JsonOf<Exclude<Written[Name], undefined>> }
    : { readonly [Name in keyof Written]: JsonOf<Written[Name]> };

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

/** The call of one named method of an object type, as its handlers see it before its transaction. */
export type CallOf<Type extends MethodOwner, Name extends keyof Type["methods"]> = MethodCall<
    TypesOf<Type["methods"][Name], Type["table"]>
>;

/** The call of one named method of an object type inside its transaction, with its prepared work. */
export type PreparedCallOf<
    Type extends MethodOwner,
    Name extends keyof Type["methods"],
> = PreparedCall<TypesOf<Type["methods"][Name], Type["table"]>>;

/** The handlers an object type takes for its methods, by method name. */
export type HandlersOf<Type extends MethodOwner> = {
    readonly [Name in keyof Type["methods"]]?: HandlerOf<
        TypesOf<Type["methods"][Name], Type["table"]>
    >;
};

/** The behaviour a method takes: a handler or phases, and phases with a prepare phase when it declares prepared work. */
export type HandlerOf<Configuration extends Partial<MethodConfiguration>> = [
    OptionOf<Configuration, "prepared", never>,
] extends [never]
    ? Handler<Configuration> | Lifecycle<Configuration>
    : schema.Schema extends OptionOf<Configuration, "prepared", never>
      ? Handler<Configuration> | Lifecycle<Configuration>
      : Lifecycle<Configuration>;

/** An object's own behaviour for a method, wrapping next, taking any call of the method as a method would. */
export type Handler<Configuration extends Partial<MethodConfiguration> = MethodConfiguration> =
    Bivariant<
        (
            call: PreparedCall<Configuration>,
            next: (call?: Call) => Promise<MethodResult<Configuration>>,
        ) => Promise<MethodResult<Configuration>>
    >;

/** An object's own behaviour for a method with external side effects, preparing the work its method declares. */
export type Lifecycle<Configuration extends Partial<MethodConfiguration> = MethodConfiguration> = {
    /** Authorize the call before its external work. */
    readonly authorize?: Bivariant<(call: MethodCall<Configuration>) => Promise<void>>;
    /** List the calls a mutation runs before this one, each as its caller, read before the transaction. */
    readonly expand?: Bivariant<
        (call: MethodCall<Configuration>) => Promise<readonly BranchCall[]>
    >;
    /** Derive the idempotency key of the call's external work. */
    readonly idempotencyKey?: Bivariant<(call: MethodCall<Configuration>) => string>;
    /** Change rows inside the transaction, wrapping next. */
    readonly handler?: Bivariant<
        (
            call: PreparedCall<Configuration>,
            next: (call?: Call) => Promise<MethodResult<Configuration>>,
        ) => Promise<MethodResult<Configuration>>
    >;
} & ([OptionOf<Configuration, "prepared", never>] extends [never]
    ? {}
    : schema.Schema extends OptionOf<Configuration, "prepared", never>
      ? {
            /** Do external work before the transaction, returning `call.prepared`, as two-phase commit prepares a participant. */
            readonly prepare?: Bivariant<(call: MethodCall<Configuration>) => Promise<unknown>>;
            /** Confirm the prepared work once the transaction committed, at least once. */
            readonly commit?: Bivariant<(call: SettlementCall, prepared: unknown) => Promise<void>>;
            /** Undo the prepared work once the transaction failed, at least once, without it when it was never recorded. */
            readonly rollback?: Bivariant<
                (call: SettlementCall, prepared: unknown) => Promise<void>
            >;
        }
      : {
            /** Do external work before the transaction, returning `call.prepared`, as two-phase commit prepares a participant. */
            readonly prepare: Bivariant<
                (call: MethodCall<Configuration>) => Promise<PreparedOf<Configuration>>
            >;
            /** Confirm the prepared work once the transaction committed, at least once. */
            readonly commit?: Bivariant<
                (call: SettlementCall, prepared: PreparedOf<Configuration>) => Promise<void>
            >;
            /** Undo the prepared work once the transaction failed, at least once, without it when it was never recorded. */
            readonly rollback?: Bivariant<
                (
                    call: SettlementCall,
                    prepared: PreparedOf<Configuration> | undefined,
                ) => Promise<void>
            >;
        });

/**
 * A callback taking any call of its method as a method would, typed by the call it declares.
 *
 * The server calls a method's callbacks only with calls of that method, and settles prepared work only with the value its `prepared` schema parsed.
 */
export type Bivariant<Callback extends (...parameters: never[]) => unknown> = {
    /** Run the callback. */
    run(...parameters: Parameters<Callback>): ReturnType<Callback>;
}["run"];

/** Report whether a failure is the cell's final refusal: a client error, not a timeout or throttle. */
function isRefusal(error: Error): boolean {
    return (
        isServiceError(error) &&
        error.status >= 400 &&
        error.status < 500 &&
        !TRANSIENT_STATUSES.has(error.status)
    );
}

/** Read a failed send's reason as an error. */
function sendFailure(reason: unknown): Error {
    return reason instanceof Error ? reason : new Error("a sent call failed", { cause: reason });
}
