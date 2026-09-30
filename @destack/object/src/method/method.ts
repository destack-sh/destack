import type { Expression } from "@destack/schema/expression";
import type { Version } from "@destack/schema";
import type { schema } from "@destack/schema";
import type * as sync from "@destack/sync";
import type { Call, Handler, Phases } from "./call.ts";
import type { ObjectType } from "../object/object.ts";
import type { MethodProcedure, ObjectSchema } from "./procedure.ts";
import { create, custom, get, list, remove, update, updateMany } from "../trait/record.ts";
import type { MethodKind } from "./kind.ts";
import type { Step } from "./step.ts";

export { METHOD_KINDS, type MethodKind } from "./kind.ts";

/** A named operation on an object type. */
export interface Method<
    Kind extends MethodKind = MethodKind,
    Permission extends string | null = string | null,
    Input extends schema.Schema = schema.Schema,
    Output extends schema.Schema = schema.Schema,
    Mutates extends boolean = boolean,
    Fields extends string = string,
> {
    /** The method's kind. */
    readonly kind: Kind;
    /** The permission the caller needs on the target, or null for none. */
    readonly permission: Permission;
    /** Whether the method changes state. */
    readonly mutates: Mutates;
    /** Whether each reading call is audited. */
    readonly audited?: true;
    /** The columns a creation or update writes, every written field when absent. */
    readonly fields?: readonly Fields[];
    /** The caller-supplied values beyond columns. */
    readonly input?: Input;
    /** The input fields each release computes from an earlier call's input, by the release introducing them. */
    readonly convert?: Readonly<Record<Version, Readonly<Record<string, Expression>>>>;
    /** The result of a custom method. */
    readonly output?: Output;
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
    effect(call: Call): Promise<unknown>;
    /** Execute a call around the effect. */
    execute(call: Call): Promise<unknown>;
    /** Authorize a call before its external work. */
    readonly authorize?: (call: Call) => Promise<void>;
    /** Do external work before the transaction, on the server only. */
    readonly prepare?: (call: Call) => Promise<unknown>;
    /** Confirm or cancel the prepared work. */
    readonly settle?: (call: Call, prepared: unknown, isCommitted: boolean) => Promise<void>;
    /** Derive the idempotency key of a call's external work. */
    readonly key?: (call: Call) => string;
    /** Whether only the system calls the method. */
    readonly isSystem?: true;
    /** Refuse an object type the method cannot serve. */
    validate?(object: ObjectType): void;
    /** Derive the calls undoing a step. */
    inverse?(step: Step): readonly sync.Call[] | undefined;
    /** Wrap the method in an object's own handler or phases. */
    handle(handler: Handler | Phases): this;
}

/** A method as a trait declares it. */
export type MethodDefinition<Declared extends Method> = Omit<
    Declared,
    "execute" | "handle" | "isPredicted"
> & {
    /** Whether clients predict the method, true by default. */
    readonly isPredicted?: boolean;
    /** Execute a call around `this.effect`. */
    execute?(this: Method, call: Call): Promise<unknown>;
};

/** Declare custom methods with `method(...)`, standard ones with `method.get(...)` and siblings. */
export const method = Object.assign(custom, {
    get,
    list,
    create,
    update,
    updateMany,
    delete: remove,
});

/** Complete a method a trait declares. */
export function defineMethod<Declared extends Method>(
    definition: MethodDefinition<Declared>,
): Declared {
    return {
        ...definition,
        isPredicted: definition.isPredicted ?? true,
        execute:
            definition.execute ??
            function (this: Method, call: Call) {
                return this.effect(call);
            },
        handle(this: Method, handler: Handler | Phases) {
            // normalize the handler
            const handling: Phases = typeof handler === "function" ? { effect: handler } : handler;
            if (handling.settle !== undefined && handling.prepare === undefined) {
                throw new TypeError("a method settles only the work its prepare phase does");
            }

            // wrap the current effect
            const effect = this.effect.bind(this);
            const wrapped = handling.effect;

            return {
                ...this,
                ...(wrapped === undefined
                    ? {}
                    : {
                          effect: (call: Call) =>
                              wrapped(call, (changed = call) => effect(changed)),
                          // predict a handled custom method
                          ...(this.kind === "custom" ? { isPredicted: true } : {}),
                      }),
                ...(handling.prepare === undefined
                    ? {}
                    : { prepare: handling.prepare, isPredicted: false }),
                ...(handling.settle === undefined ? {} : { settle: handling.settle }),
                ...(handling.key === undefined ? {} : { key: handling.key }),
                ...(handling.authorize === undefined
                    ? {}
                    : {
                          authorize: async (call: Call) => {
                              await this.authorize?.(call);
                              await handling.authorize!(call);
                          },
                      }),
            };
        },
    } as unknown as Declared;
}
