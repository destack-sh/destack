import type { Creation } from "@destack/access";
import type { InputSchema } from "./input.ts";
import type { Version, schema } from "@destack/schema";
import type { Expression, Table } from "@destack/db";
import type * as sync from "@destack/sync";
import type { Call, Handler, PhasedHandler } from "./call.ts";
import type { ObjectType } from "../object/object.ts";
import type { Empty, MethodProcedure, ObjectSchema } from "./procedure.ts";
import type {
    BatchPayload,
    CreateOptions,
    CustomDefinition,
    CustomTarget,
    RemoveOptions,
    UpdateManyOptions,
    UpdateOptions,
    Variants,
} from "./record.ts";
import type { MethodKind } from "./kind.ts";
import type { Step } from "./step.ts";
import type { BranchCall } from "../branch/branch.ts";
import type { Settled, SettlementCall } from "./settlement.ts";

export { METHOD_KINDS, type MethodKind } from "./kind.ts";

/** The brand every declared method keeps, through copies too. */
const METHOD: unique symbol = Symbol("method");

/** The phases a method's handler may name. */
const HANDLER_PHASES: ReadonlySet<string> = new Set([
    "authorize",
    "creation",
    "expand",
    "prepare",
    "handle",
    "settle",
]);

/** A method's signature: kind, permission, schemas and table, typing its calls and handlers. */
export interface MethodConfiguration {
    /** The method's kind. */
    readonly kind: MethodKind;
    /** The permission the caller needs on the target. */
    readonly permission: string;
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
    /** The column sets of a sum type creation, by discriminating field and value. */
    readonly variants: object;
    /** The table of the method's object type. */
    readonly table: Table;
    /** Whether the method stays off public routes, run only by the principals' own code. */
    readonly internal: boolean;
    /** Whether the method acts in its scope with no target object. */
    readonly scoped: boolean;
}

/** One type a method declares: its own, or the default when it declares none. */
export type OptionOf<
    Configuration extends Partial<MethodConfiguration>,
    Key extends keyof MethodConfiguration,
    Absent,
> = Configuration extends { readonly [Name in Key]: infer Value } ? Value : Absent;

/** The builder call of a declared method: the builder's name and its arguments as written. */
export type MethodSource =
    | { readonly builder: "get" | "list"; readonly arguments: readonly [permission: string] }
    | {
          readonly builder: "create";
          readonly arguments: readonly [
              permission: string,
              options: CreateOptions<string, schema.JsonObject, schema.Schema, boolean>,
          ];
      }
    | {
          readonly builder: "update";
          readonly arguments: readonly [
              permission: string,
              options: UpdateOptions<string, schema.JsonObject, schema.Schema>,
          ];
      }
    | {
          readonly builder: "updateMany";
          readonly arguments: readonly [
              permission: string,
              options: UpdateManyOptions<string, string>,
          ];
      }
    | {
          readonly builder: "delete";
          readonly arguments: readonly [permission: string, options: RemoveOptions<schema.Schema>];
      }
    | {
          readonly builder: "query" | "mutation";
          readonly arguments: readonly [
              definition: CustomDefinition<
                  string,
                  InputSchema,
                  schema.Schema,
                  schema.Schema,
                  boolean,
                  CustomTarget
              >,
          ];
      };

/** A named operation on an object type, typed by its input, output and prepared work. */
export interface Method<Configuration extends Partial<MethodConfiguration> = MethodConfiguration> {
    /** The brand marking a declared method. */
    readonly [METHOD]: true;
    /** The method's kind. */
    readonly kind: OptionOf<Configuration, "kind", MethodKind>;
    /** The permission the caller needs on the target. */
    readonly permission: OptionOf<Configuration, "permission", string>;
    /** Whether the method changes state. */
    readonly mutates: OptionOf<Configuration, "mutates", boolean>;
    /** Whether each reading call is audited. */
    readonly audited?: true;
    /** The columns a creation or update writes, every written field when absent. */
    readonly fields?: readonly OptionOf<Configuration, "fields", string>[];
    /** The column sets of a sum type a creation writes, one beside the shared fields. */
    readonly variants?: OptionOf<Configuration, "variants", never>;
    /** The caller-supplied values beyond columns. */
    readonly input?: OptionOf<Configuration, "input", never>;
    /** The input fields each release computes from earlier input, by introducing release. */
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
    /** The fields of the result an audited read's event records. */
    readonly audit?: { readonly details: schema.Object<Record<string, schema.Schema>> };
    /** Derive the method's procedure under a name. */
    procedure(name: string, schema: ObjectSchema): MethodProcedure;
    /** Change rows for a call. */
    handler(call: Call): Promise<unknown>;
    /** Execute a call around the handler. */
    execute(call: Call): Promise<unknown>;
    /** Authorize a call before its external work. */
    authorize?(call: Call): Promise<void>;
    /** Derive a created object's first relationships and a new scope's owner, for creations. */
    readonly creation?: (call: Call, object: sync.ObjectReference) => Creation | Promise<Creation>;
    /** List the calls a mutation runs before this one as their callers, before the transaction. */
    expand?(call: Call): Promise<readonly BranchCall[]>;
    /** Do external work before the transaction, on the server only. */
    prepare?(call: Call): Promise<unknown>;
    /** The builder call a declared method came from, absent for a method a trait derives. */
    readonly source?: MethodSource;
    /** Settle the prepared work once the transaction committed or failed, at least once. */
    settle?(call: SettlementCall, settled: Settled): Promise<void>;
    /** Whether the method stays off public routes, run only by the principals' own code. */
    readonly isInternal?: OptionOf<Configuration, "internal", boolean>;
    /** Whether the method acts in its scope with no target object. */
    readonly isScoped?: OptionOf<Configuration, "scoped", boolean>;
    /** Refuse an object type the method cannot serve. */
    validate?(object: ObjectType): void;
    /** Derive the calls undoing a step. */
    inverse?(step: Step): readonly sync.Call[] | undefined;
    /** Wrap the method in an object's own handler or phases, leaving it no builder call source. */
    handle(handler: Handler<Configuration>): Method<Configuration>;
}

/** The method declarations of one object type, typed by its table. */
export interface MethodBuilder<
    Definition extends Table = Table,
    Written extends string = string,
    Granted extends string = string,
> {
    /** Declare a method reading one object, as tRPC's `query`. */
    query<
        const Permission extends Granted,
        Input extends InputSchema = never,
        Output extends schema.Schema = never,
        Prepared extends schema.Schema = never,
        const Internal extends boolean = false,
        const Target extends CustomTarget = true,
    >(
        definition: CustomDefinition<Permission, Input, Output, Prepared, Internal, Target>,
    ): Method<{
        kind: "custom";
        permission: Permission;
        input: NoInfer<Input>;
        output: NoInfer<Output>;
        prepared: NoInfer<Prepared>;
        mutates: false;
        table: Definition;
        internal: NoInfer<Internal>;
        scoped: Target extends "none" ? true : false;
    }>;
    /** Declare a method changing one object, as tRPC's `mutation`. */
    mutation<
        const Permission extends Granted,
        Input extends InputSchema = never,
        Output extends schema.Schema = never,
        Prepared extends schema.Schema = never,
        const Internal extends boolean = false,
        const Target extends CustomTarget = true,
    >(
        definition: CustomDefinition<Permission, Input, Output, Prepared, Internal, Target>,
    ): Method<{
        kind: "custom";
        permission: Permission;
        input: NoInfer<Input>;
        output: NoInfer<Output>;
        prepared: NoInfer<Prepared>;
        mutates: true;
        table: Definition;
        internal: NoInfer<Internal>;
        scoped: Target extends "none" ? true : false;
    }>;
    /** Read one object. */
    get<const Permission extends Granted>(
        permission: Permission,
    ): Method<{ kind: "get"; permission: Permission; mutates: false; table: Definition }>;
    /** List the objects a query selects. */
    list<const Permission extends Granted>(
        permission: Permission,
    ): Method<{ kind: "list"; permission: Permission; mutates: false; table: Definition }>;
    /** Create an object from the fields the caller writes. */
    create<
        const Permission extends Granted,
        const Fields extends Written = Written,
        Input extends schema.JsonObject = never,
        Prepared extends schema.Schema = never,
        const Internal extends boolean = false,
        const Sum extends Variants<Written> = never,
    >(
        permission: Permission,
        options?: CreateOptions<Fields, Input, Prepared, Internal, Sum>,
    ): Method<{
        kind: "create";
        permission: Permission;
        input: NoInfer<Input>;
        prepared: NoInfer<Prepared>;
        mutates: true;
        fields: NoInfer<Fields>;
        variants: NoInfer<Sum>;
        table: Definition;
        internal: NoInfer<Internal>;
    }>;
    /** Update an object's written fields. */
    update<
        const Permission extends Granted,
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
        const Permission extends Granted,
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
    delete<const Permission extends Granted, Prepared extends schema.Schema = never>(
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
    typeof METHOD | "execute" | "handle" | "isPredicted"
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
    /** Decide whether a value is a declared method. */
    is(value: unknown): value is Method {
        return typeof value === "object" && value !== null && METHOD in value;
    },

    /** Decide whether a method settles its prepared work. */
    settles(declared: Method): boolean {
        return declared.settle !== undefined;
    },
};

/** Complete a method a trait declares. */
export function defineMethod<Configuration extends Partial<MethodConfiguration>>(
    definition: MethodDefinition<Configuration>,
): Method<Configuration> {
    const declared: Method<Configuration> = {
        ...definition,
        [METHOD]: true,
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
    handler: Handler<Configuration>,
): Method<Configuration> {
    // normalize the handler, refusing a key that is no phase
    const handling: PhasedHandler = typeof handler === "function" ? { handle: handler } : handler;
    const unknown = Object.keys(handling).find((key) => !HANDLER_PHASES.has(key));
    if (unknown !== undefined) {
        throw new TypeError(`a method's handler has no phase ${unknown}`);
    }
    if (handling.settle !== undefined && handling.prepare === undefined) {
        throw new TypeError("a method settles only the work its prepare phase does");
    } else if (handling.prepare !== undefined && declared.prepared === undefined) {
        throw new TypeError("a method prepares only the work it declares");
    } else if (handling.creation !== undefined && declared.creation !== undefined) {
        throw new TypeError("a creation names a creator relation or a creation, not both");
    }

    // wrap the current handler, and run the declared authorization before the handler's
    const wrapped = handling.handle;
    const authorize = handling.authorize;
    const { source: _source, ...unsourced } = declared;
    const wrapping: Method<Configuration> = {
        ...unsourced,
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
        ...(handling.settle === undefined ? {} : { settle: handling.settle }),
        ...(handling.expand === undefined ? {} : { expand: handling.expand }),
        ...(handling.creation === undefined ? {} : { creation: handling.creation }),
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
