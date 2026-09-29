import { ObjectReference } from "@destack/sync";
import { type GrantReader, type Subject } from "@destack/access";
import {
    and,
    eq,
    type Column,
    type DatabaseConnection,
    type Insert,
    type Select,
    type Table,
} from "@destack/db";
import { schema } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import type * as sync from "@destack/sync";
import type { ObjectType } from "../object/object.ts";
import type { Authorization } from "../server/authorization.ts";
import type { Method } from "./method.ts";
import { ParentReference } from "../trait/nested.ts";

/** One call of a method inside its transaction, served or predicted. */
export class Call<Definition extends Table = Table> {
    /** The object type. */
    readonly object: ObjectType;
    /** The method's name. */
    readonly name: string;
    /** The method. */
    readonly method: Method;
    /** The scope containing the object. */
    readonly scope: string;
    /** The scope and the scopes containing it, nearest first. */
    readonly chain: readonly string[];
    /** The method's own input fields. */
    readonly input: Readonly<Record<string, unknown>>;
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
    /** Whether a client predicts the call. */
    readonly isPredicted: boolean;
    /** The server's authorization, absent in a prediction. */
    readonly authorization?: Authorization;
    /** The object types served together. */
    readonly objects: readonly ObjectType[];
    /** The value the prepare phase produced. */
    readonly prepared?: unknown;
    /** The idempotency key of the call's external work. */
    readonly key?: string;
    /** The client writing an ephemeral object. */
    readonly client?: string;
    /** Run another method in the call's transaction as its caller. */
    readonly run?: Run;

    /** Hold one call's fields. */
    constructor(fields: CallFields<Definition>) {
        // take the required fields
        this.object = fields.object;
        this.name = fields.name;
        this.method = fields.method;
        this.scope = fields.scope;
        this.chain = fields.chain;
        this.input = fields.input;
        this.database = fields.database;
        this.now = fields.now;
        this.objects = fields.objects;
        this.isPredicted = fields.isPredicted;

        // require an authorization for served calls
        if (!fields.isPredicted && fields.authorization === undefined) {
            throw new TypeError(`a served call of ${fields.name} needs an authorization`);
        }

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
        if (fields.key !== undefined) {
            this.key = fields.key;
        }
        if (fields.client !== undefined) {
            this.client = fields.client;
        }
        if (fields.run !== undefined) {
            this.run = fields.run;
        }
    }

    /** Record a call of an object's method. */
    static record(
        object: ObjectType,
        name: string,
        input: Readonly<Record<string, unknown>>,
    ): sync.Call {
        return {
            method: `${object.name}.${name}`,
            input: schema.record(schema.string(), schema.json()).parse(input),
            ...(object.version > 1 ? { version: object.version } : {}),
        };
    }

    /** Upcast a recorded call's input to the object type's current version. */
    static upcast(object: ObjectType, name: string, call: sync.Call): Record<string, unknown> {
        const from = call.version ?? 1;
        if (from > object.version) {
            throw new ServiceError("BAD_REQUEST", {
                message: `${call.method} was made against version ${from} of ${object.name}, which is at version ${object.version}`,
            });
        }

        return from === object.version ? call.input : object.upcast!(name, call.input, from);
    }

    /** Read the identifier of the object a method returned. */
    static resultId(result: unknown): string | undefined {
        const id = (result as { id?: unknown } | null)?.id;

        return typeof id === "string" ? id : undefined;
    }

    /** Call another object's method in this call's scope and transaction. */
    async invoke(
        object: ObjectType,
        name: string,
        input: Readonly<Record<string, unknown>>,
    ): Promise<unknown> {
        // require a method without external work in the same storage
        const method = (object.methods as Readonly<Record<string, Method>>)[name];
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

        return this.run(object, name, input);
    }

    /** Copy the call with some fields changed. */
    with(changes: Partial<CallFields<Definition>>): Call<Definition> {
        return new Call({ ...this, ...changes });
    }

    /** Read the server's authorization and refuse a prediction. */
    served(): Authorization {
        if (this.isPredicted) {
            throw new ServiceError("FORBIDDEN", {
                message: `${this.object.name} changes on the server`,
            });
        }

        return this.authorization!;
    }

    /** Reference the object the call acts on, in the call's scope. */
    reference(): ObjectReference {
        return this.object.reference(this.scope, this.id!);
    }

    /** Read the parent the call's input names. */
    parent(): ObjectReference | undefined {
        // read the named parent
        const declared = this.object.parent!;
        const named = declared.object === "any" ? this.input.parent : this.input.parentId;
        if (named === undefined || named === null) {
            return undefined;
        }

        // place the parent in the object's scope
        return declared.object === "any"
            ? ObjectReference.parse({ ...ParentReference.parse(named), scope: this.scope })
            : declared.object.reference(this.scope, schema.string().parse(named));
    }

    /** Read the parent columns the call's input writes. */
    parentColumns(): Record<string, unknown> {
        // write the typed parent's identifier as named
        if (this.object.parent!.object !== "any") {
            return { parentId: this.input.parentId };
        }

        // write no parent, or the reference's parts
        const parent = this.parent();

        return parent === undefined
            ? { parentPackageId: null, parentType: null, parentId: null }
            : { parentPackageId: parent.packageId, parentType: parent.type, parentId: parent.id };
    }

    /** Require the receive permission on a parent. */
    async requireReceiving(parent: ObjectReference, reader?: GrantReader): Promise<void> {
        // require an attachment host type
        const declared = this.object.parent!;
        const authorizer = this.authorization!.authorizer;
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
            const table = mapping.table as Table & Record<string, Column>;
            const [held] = await this.authorization!.database.select({ id: table[mapping.id]! })
                .from(table)
                .where(
                    and(
                        eq(table[mapping.id]!, parent.id),
                        mapping.scope === undefined
                            ? undefined
                            : eq(table[mapping.scope]!, parent.scope),
                    ),
                );
            if (held === undefined) {
                throw new ServiceError("NOT_FOUND", { message: `no ${parent.type} ${parent.id}` });
            }
        }

        // accept a receiving parent
        const receive = authorizer.policy(parent).permission(declared.receive);
        if ((await this.authorization!.check(receive, parent, reader)).isAllowed) {
            return;
        }

        // hide an unreadable parent, else require the permission
        const host =
            declared.object === "any"
                ? this.objects.find((object) => object.policy.is(parent))
                : declared.object;
        const reading = host?.reading;
        if (
            reading === undefined ||
            !(await this.authorization!.check(reading, parent)).isAllowed
        ) {
            throw new ServiceError("NOT_FOUND", { message: `no ${parent.type} ${parent.id}` });
        }
        await this.authorization!.require(receive, parent);
    }

    /** Update the target's desired state at the loaded revision. */
    async revise(changes: Readonly<Record<string, unknown>>): Promise<Record<string, unknown>> {
        // advance the generation of a controlled target
        const target = this.target as Record<string, unknown>;
        const generation = this.object.isControlled
            ? { generation: (target.generation as number) + 1 }
            : {};

        return this.#write({ ...changes, ...generation });
    }

    /** Update the target's observed state at the loaded revision and generation. */
    async observe(changes: Readonly<Record<string, unknown>>): Promise<Record<string, unknown>> {
        return this.#write(changes);
    }

    /** Delete the target at the loaded revision. */
    async remove(): Promise<void> {
        // delete only the revision the call loaded
        const table = this.object.table as Table & Record<string, never>;
        const target = this.target as Record<string, unknown>;
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
    async #write(changes: Readonly<Record<string, unknown>>): Promise<Record<string, unknown>> {
        // update at the loaded revision
        const table = this.object.table as Table & Record<string, never>;
        const target = this.target as Record<string, unknown>;
        const [row] = (await this.database
            .update(table)
            .set({
                ...changes,
                revision: (target.revision as number) + 1,
                updatedAt: this.now,
            } as Partial<Insert<Table>>)
            .where(and(eq(table.id, target.id), eq(table.revision, target.revision)))
            .returning()) as Record<string, unknown>[];
        if (!row) {
            throw new ServiceError("CONFLICT", {
                message: `${this.object.name} revision has changed`,
            });
        }

        return row;
    }
}

/** The fields of a call. */
export type CallFields<Definition extends Table = Table> = Pick<
    Call<Definition>,
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
    | "isPredicted"
    | "authorization"
    | "objects"
    | "prepared"
    | "key"
    | "client"
    | "run"
>;

/** Run a method of an object in a call's scope and transaction, as its caller. */
export type Run = (
    object: ObjectType,
    name: string,
    input: Readonly<Record<string, unknown>>,
) => Promise<unknown>;

/** An object's own behaviour for a method, wrapping next. */
export type Handler<Definition extends Table = Table> = (
    call: Call<Definition>,
    next: (call?: Call<Definition>) => Promise<unknown>,
) => Promise<unknown>;

/** An object's own behaviour for a method with external side effects. */
export interface Phases<Definition extends Table = Table> {
    /** Authorize the call before its external work. */
    readonly authorize?: (call: Call<Definition>) => Promise<void>;
    /** Do external work before the transaction, returning `call.prepared`. */
    readonly prepare?: (call: Call<Definition>) => Promise<unknown>;
    /** Derive the idempotency key of the call's external work. */
    readonly key?: (call: Call<Definition>) => string;
    /** Change rows inside the transaction, wrapping next. */
    readonly effect?: Handler<Definition>;
    /** Confirm or cancel the prepared work, at least once. */
    readonly settle?: (
        call: Call<Definition>,
        prepared: unknown,
        isCommitted: boolean,
    ) => Promise<void>;
}
