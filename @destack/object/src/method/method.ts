import type { Version, schema } from "@destack/schema";
import type { Expression, Table } from "@destack/db";
import type * as sync from "@destack/sync";
import type { Call, HandlerOf, Lifecycle } from "./call.ts";
import type { ObjectType } from "../object/object.ts";
import type { Empty, MethodProcedure, ObjectSchema } from "./procedure.ts";
import {
    create,
    mutation,
    query,
    get,
    list,
    remove,
    update,
    updateMany,
    type BatchPayload,
    type CreateOptions,
    type CustomDefinition,
    type RemoveOptions,
    type UpdateManyOptions,
    type UpdateOptions,
} from "../trait/record.ts";
import type { MethodKind } from "./kind.ts";
import type { Step } from "./step.ts";
import type { BranchCall } from "../branch/branch.ts";
import type { SettlementCall } from "./settlement.ts";

export { METHOD_KINDS, type MethodKind } from "./kind.ts";

/** A method's signature: its kind, permission, schemas and table, which type its calls and handlers. */
export interface MethodConfiguration {
    /** The method's kind. */
    readonly kind: MethodKind;
    /** The permission the caller needs on the target, or null for none. */
    readonly permission: string | null;
    /** The caller-supplied values beyond columns. */
    readonly input: schema.Schema;
    /** The result of a custom method. */
    readonly output: schema.Schema;
    /** The external work the method prepares before its transaction. */
    readonly prepared: schema.Schema;
    /** Whether the method changes state. */
    readonly mutates: boolean;
    /** The columns a creation or update writes. */
    readonly fields: string;
    /** The table of the method's object type. */
    readonly table: Table;
    /** Whether only the system calls the method. */
    readonly system: boolean;
}

/** One type a method declares: its own, or the default when it declares none. */
export type OptionOf<
    Configuration extends Partial<MethodConfiguration>,
    Key extends keyof MethodConfiguration,
    Absent,
> = Configuration extends { readonly [Name in Key]: infer Value } ? Value : Absent;

/** A named operation on an object type, typed by the input, output and prepared work it declares. */
export interface Method<Configuration extends Partial<MethodConfiguration> = MethodConfiguration> {
    /** The method's kind. */
    readonly kind: OptionOf<Configuration, "kind", MethodKind>;
    /** The permission the caller needs on the target, or null for none. */
    readonly permission: OptionOf<Configuration, "permission", string | null>;
    /** Whether the method changes state. */
    readonly mutates: OptionOf<Configuration, "mutates", boolean>;
    /** Whether each reading call is audited. */
    readonly audited?: true;
    /** The columns a creation or update writes, every written field when absent. */
    readonly fields?: readonly OptionOf<Configuration, "fields", string>[];
    /** The caller-supplied values beyond columns. */
    readonly input?: OptionOf<Configuration, "input", never>;
    /** The input fields each release computes from an earlier call's input, by the release introducing them. */
    readonly convert?: Readonly<Record<Version, Readonly<Record<string, Expression>>>>;
    /** The result of a custom method. */
    readonly output?: OptionOf<Configuration, "output", never>;
    /** The external work the method prepares before its transaction, kept until it settles. */
    readonly prepared?: OptionOf<Configuration, "prepared", never>;
    /** The state change a transition makes. */
    readonly transition?: {
        readonly field: string;
        readonly from: readonly string[];
        readonly to: string;
    };
    /** Whether the permitted target loads before the method runs. */
    readonly target: boolean;
    /** What the method returns. */
    readonly result: "object" | "page" | "value";
    /** Whether clients predict the method. */
    readonly isPredicted: boolean;
    /** The fields of the result an audited read's event names. */
    readonly audit?: { readonly details: schema.Object<Record<string, schema.Schema>> };
    /** Derive the method's procedure under a name. */
    procedure(name: string, schema: ObjectSchema): MethodProcedure;
    /** Change rows for a call. */
    handler(call: Call): Promise<unknown>;
    /** Execute a call around the handler. */
    execute(call: Call): Promise<unknown>;
    /** Authorize a call before its external work. */
    authorize?(call: Call): Promise<void>;
    /** List the calls a mutation runs before this one, each as its caller, read before the transaction. */
    expand?(call: Call): Promise<readonly BranchCall[]>;
    /** Do external work before the transaction, on the server only. */
    prepare?(call: Call): Promise<unknown>;
    /** Confirm the prepared work once the transaction committed. */
    commit?(call: SettlementCall, prepared: unknown): Promise<void>;
    /** Undo the prepared work once the transaction failed, without it when it was never recorded. */
    rollback?(call: SettlementCall, prepared: unknown): Promise<void>;
    /** Derive the idempotency key of a call's external work. */
    idempotencyKey?(call: Call): string;
    /** Whether only the system calls the method. */
    readonly isSystem?: OptionOf<Configuration, "system", boolean>;
    /** Refuse an object type the method cannot serve. */
    validate?(object: ObjectType): void;
    /** Derive the calls undoing a step. */
    inverse?(step: Step): readonly sync.Call[] | undefined;
    /** Wrap the method in an object's own handler or phases. */
    handle(handler: HandlerOf<Configuration>): Method<Configuration>;
}

/** The method declarations of one object type, typed by its table. */
export interface MethodBuilder<Definition extends Table = Table, Written extends string = string> {
    /** Declare a method reading one object, as tRPC's `query`. */
    query<
        const Permission extends string | null,
        Input extends schema.JsonObject = never,
        Output extends schema.Schema = never,
        Prepared extends schema.Schema = never,
        const System extends boolean = false,
    >(
        definition: CustomDefinition<Permission, Input, Output, Prepared, System>,
    ): Method<{
        kind: "custom";
        permission: Permission;
        input: NoInfer<Input>;
        output: NoInfer<Output>;
        prepared: NoInfer<Prepared>;
        mutates: false;
        table: Definition;
        system: NoInfer<System>;
    }>;
    /** Declare a method changing one object, as tRPC's `mutation`. */
    mutation<
        const Permission extends string | null,
        Input extends schema.JsonObject = never,
        Output extends schema.Schema = never,
        Prepared extends schema.Schema = never,
        const System extends boolean = false,
    >(
        definition: CustomDefinition<Permission, Input, Output, Prepared, System>,
    ): Method<{
        kind: "custom";
        permission: Permission;
        input: NoInfer<Input>;
        output: NoInfer<Output>;
        prepared: NoInfer<Prepared>;
        mutates: true;
        table: Definition;
        system: NoInfer<System>;
    }>;
    /** Read one object. */
    get<const Permission extends string>(
        permission: Permission,
    ): Method<{ kind: "get"; permission: Permission; mutates: false; table: Definition }>;
    /** List the objects a query selects. */
    list<const Permission extends string>(
        permission: Permission,
    ): Method<{ kind: "list"; permission: Permission; mutates: false; table: Definition }>;
    /** Create an object from the fields the caller writes. */
    create<
        const Permission extends string | null,
        const Fields extends Written = Written,
        Input extends schema.JsonObject = never,
        Prepared extends schema.Schema = never,
        const System extends boolean = false,
    >(
        permission: Permission,
        options?: CreateOptions<Fields, Input, Prepared, System, Definition>,
    ): Method<{
        kind: "create";
        permission: Permission;
        input: NoInfer<Input>;
        prepared: NoInfer<Prepared>;
        mutates: true;
        fields: NoInfer<Fields>;
        table: Definition;
        system: NoInfer<System>;
    }>;
    /** Update an object's written fields. */
    update<
        const Permission extends string | null,
        const Fields extends Written = Written,
        Input extends schema.JsonObject = never,
        Prepared extends schema.Schema = never,
    >(
        permission: Permission,
        options?: UpdateOptions<Fields, Input, Prepared>,
    ): Method<{
        kind: "update";
        permission: Permission;
        input: NoInfer<Input>;
        prepared: NoInfer<Prepared>;
        mutates: true;
        fields: NoInfer<Fields>;
        table: Definition;
    }>;
    /** Update every matching object the caller may change, returning the count. */
    updateMany<
        const Permission extends string,
        const Fields extends Written = Written,
        const Match extends string = string,
    >(
        permission: Permission,
        options: UpdateManyOptions<Fields, Match>,
    ): Method<{
        kind: "updateMany";
        permission: Permission;
        output: typeof BatchPayload;
        mutates: true;
        fields: NoInfer<Fields>;
        table: Definition;
    }>;
    /** Delete an object. */
    delete<const Permission extends string | null, Prepared extends schema.Schema = never>(
        permission: Permission,
        options?: RemoveOptions<Prepared>,
    ): Method<{
        kind: "delete";
        permission: Permission;
        output: typeof Empty;
        prepared: NoInfer<Prepared>;
        mutates: true;
        table: Definition;
    }>;
}

/** A method as a trait declares it. */
export type MethodDefinition<Configuration extends Partial<MethodConfiguration>> = Omit<
    Method<Configuration>,
    "execute" | "handle" | "isPredicted"
> & {
    /** Whether clients predict the method, true by default. */
    readonly isPredicted?: boolean;
    /** Execute a call around `this.handler`. */
    readonly execute?: {
        /** Execute the call through the method it belongs to. */
        run(this: Method, call: Call): Promise<unknown>;
    }["run"];
};

/** What every method can report about itself. */
export const Method = {
    /** Decide whether a method settles its prepared work: it commits or rolls it back. */
    settles(declared: Method): boolean {
        return declared.commit !== undefined || declared.rollback !== undefined;
    },
};

/** Declare methods with `method.query(...)`, `method.mutation(...)`, `method.get(...)` and their siblings. */
export const method: MethodBuilder = {
    query,
    mutation,
    get,
    list,
    create,
    update,
    updateMany,
    delete: remove,
};

/** Complete a method a trait declares. */
export function defineMethod<Configuration extends Partial<MethodConfiguration>>(
    definition: MethodDefinition<Configuration>,
): Method<Configuration> {
    const declared: Method<Configuration> = {
        ...definition,
        isPredicted: definition.isPredicted ?? true,
        execute:
            definition.execute ??
            function (this: Method, call: Call) {
                return this.handler(call);
            },
        handle: (handler) => handled(declared, handler),
    };

    return declared;
}

/** Wrap a method in an object's own handler or phases. */
function handled<Configuration extends Partial<MethodConfiguration>>(
    declared: Method<Configuration>,
    handler: HandlerOf<Configuration>,
): Method<Configuration> {
    // normalize the handler
    const handling: Lifecycle = typeof handler === "function" ? { handler } : handler;
    if (
        (handling.commit !== undefined || handling.rollback !== undefined) &&
        handling.prepare === undefined
    ) {
        throw new TypeError("a method commits or rolls back only the work its prepare phase does");
    } else if (handling.prepare !== undefined && declared.prepared === undefined) {
        throw new TypeError("a method prepares only the work it declares");
    }

    // wrap the current handler, and run the declared authorization before the handler's
    const wrapped = handling.handler;
    const authorize = handling.authorize;
    const wrapping: Method<Configuration> = {
        ...declared,
        ...(wrapped === undefined
            ? {}
            : {
                  handler: (call: Call) =>
                      wrapped(call, (changed = call) => declared.handler(changed)),
                  // predict a handled custom method
                  ...(declared.kind === "custom" ? { isPredicted: true } : {}),
              }),
        ...(handling.prepare === undefined
            ? {}
            : { prepare: handling.prepare, isPredicted: false }),
        ...(handling.commit === undefined ? {} : { commit: handling.commit }),
        ...(handling.rollback === undefined ? {} : { rollback: handling.rollback }),
        ...(handling.expand === undefined ? {} : { expand: handling.expand }),
        ...(handling.idempotencyKey === undefined
            ? {}
            : { idempotencyKey: handling.idempotencyKey }),
        ...(authorize === undefined
            ? {}
            : {
                  authorize: async (call: Call) => {
                      await declared.authorize?.(call);
                      await authorize(call);
                  },
              }),
        handle: (next) => handled(wrapping, next),
    };

    return wrapping;
}
