import { Caller, type Creation } from "@destack/access";
import type { InputObject, InputSchema } from "./input.ts";
import { type Version, schema } from "@destack/schema";
import { type Expression, Row, type Table } from "@destack/db";
import type * as sync from "@destack/sync";
import type {
    HandleFunction,
    Handler,
    MethodResult,
    MethodContext,
    PhasedHandler,
    PreparedOf,
} from "./context.ts";
import type { ObjectType } from "../object/object.ts";
import type { MethodProcedure, ObjectSchema } from "./procedure.ts";
import type {
    CreateManyOptions,
    CreateOptions,
    CustomDefinition,
    MethodTarget,
    DeleteManyOptions,
    RemoveOptions,
    UpdateManyOptions,
    UpdateOptions,
} from "./record.ts";
import type { MethodKind } from "./kind.ts";
import type { Step } from "./step.ts";
import type { BranchEntry } from "../branch/entry.ts";
import type { Settled, SettlementContext } from "./settlement.ts";
import type { Execution, StoreExecution } from "./execution.ts";

export { METHOD_KINDS, type MethodKind } from "./kind.ts";

/** The brand every declared method has, through copies too. */
const METHOD: unique symbol = Symbol("method");

/** A list method's page, as a handler's next reads it. */
const LISTED = schema.looseObject({
    items: schema.array(schema.json()),
    cursor: schema.string().nullable(),
});

/** The phases a method's handler may name, in the order a call runs them. */
export const PHASES = ["authorize", "creation", "expand", "prepare", "handle", "settle"] as const;

/** A phase of a method's handler. */
export type Phase = (typeof PHASES)[number];

/** A method result identifying an object by its identifier. */
const IDENTIFIED = schema.looseObject({ id: schema.string() });

/** A method's signature: kind, permission, schemas and table, each absent option at its default. */
export interface MethodConfiguration {
    /** The method's kind. */
    readonly kind?: MethodKind;
    /** The permission the caller needs on the target. */
    readonly permission?: string;
    /** The caller-supplied values beyond columns. */
    readonly input?: schema.Schema;
    /** The result of a custom method. */
    readonly output?: schema.Schema;
    /** The external work the method prepares before its transaction. */
    readonly prepared?: schema.Schema;
    /** Whether the method changes state. */
    readonly mutates?: boolean;
    /** What the method acts on: its loaded object, its object by identifier, or its scope. */
    readonly target?: MethodTarget;
    /** The columns a creation or update writes. */
    readonly fields?: string;
    /** The written columns a creation's handler fills where the caller leaves them out. */
    readonly filled?: string;
    /** The columns a bulk update or deletion matches objects by. */
    readonly match?: string;
    /** The column sets of a sum type creation, by discriminating field and value. */
    readonly variants?: object;
    /** The table of the method's object type. */
    readonly table?: Table;
    /** Whether the method stays off public routes, run only by the principals' own code. */
    readonly internal?: boolean;
    /** Whether a bulk update takes items, each an object's identifier and revision. */
    readonly items?: boolean;
}

/** One type a method declares: its own, or the default when it declares none. */
export type OptionOf<
    Configuration extends MethodConfiguration,
    Key extends keyof MethodConfiguration,
    Absent,
> = Configuration extends { readonly [Name in Key]: infer Value }
    ? Value
    : Configuration extends { readonly [Name in Key]?: infer Value }
      ? unknown extends Value
          ? Absent
          : Value
      : Absent;

/** The builder call of a declared method: the builder's name and its arguments as written. */
export type MethodSource =
    | { readonly builder: "get" | "list"; readonly arguments: readonly [permission: string] }
    | {
          readonly builder: "create";
          readonly arguments: readonly [
              permission: string,
              options: CreateOptions<string, InputObject, schema.Schema, boolean>,
          ];
      }
    | {
          readonly builder: "createMany";
          readonly arguments: readonly [
              permission: string,
              options: CreateManyOptions<string, boolean>,
          ];
      }
    | {
          readonly builder: "update";
          readonly arguments: readonly [
              permission: string,
              options: UpdateOptions<string, InputObject, schema.Schema>,
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
          readonly builder: "updateItems";
          readonly arguments: readonly [
              permission: string,
              options: Omit<UpdateManyOptions<string, never>, "match">,
          ];
      }
    | {
          readonly builder: "delete";
          readonly arguments: readonly [permission: string, options: RemoveOptions<schema.Schema>];
      }
    | {
          readonly builder: "deleteMany";
          readonly arguments: readonly [permission: string, options: DeleteManyOptions<string>];
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
                  MethodTarget
              >,
          ];
      };

/** A named operation on an object type, typed by its input, output and prepared work. */
export interface Method<Configuration extends MethodConfiguration = MethodConfiguration> {
    /** The brand marking a declared method. */
    readonly [METHOD]: true;
    /** The method's kind. */
    readonly kind: OptionOf<Configuration, "kind", MethodKind>;
    /** The permission the caller needs on the target. */
    readonly permission: OptionOf<Configuration, "permission", string>;
    /** Whether the method changes state. */
    readonly mutates: OptionOf<Configuration, "mutates", boolean>;
    /** Whether each reading call is audited. */
    readonly isAudited?: boolean;
    /** The columns a creation or update writes, every written field when absent. */
    readonly fields?: readonly OptionOf<Configuration, "fields", string>[];
    /** The written columns a creation's handler fills where the caller leaves them out. */
    readonly filled?: readonly OptionOf<Configuration, "filled", string>[];
    /** The columns a bulk update or deletion matches objects by. */
    readonly match?: readonly OptionOf<Configuration, "match", string>[];
    /** The column sets of a sum type a creation writes, one beside the shared fields. */
    readonly variants?: OptionOf<Configuration, "variants", never>;
    /** The caller-supplied values beyond columns. */
    readonly input?: OptionOf<Configuration, "input", never>;
    /** The input fields each release computes from earlier input, by introducing release. */
    readonly convert?: Readonly<Record<Version, Readonly<Record<string, Expression>>>>;
    /** The result of a custom method. */
    readonly output?: OptionOf<Configuration, "output", never>;
    /** The external work the method prepares before its transaction, held until it settles. */
    readonly prepared?: OptionOf<Configuration, "prepared", never>;
    /** The state change a transition makes. */
    readonly transition?: {
        readonly field: string;
        readonly from: readonly string[];
        readonly to: string;
    };
    /** What the method acts on, its loaded target when absent. */
    readonly target?: OptionOf<Configuration, "target", "object">;
    /** What the method returns. */
    readonly result: "object" | "page" | "value";
    /** Whether clients predict the method. */
    readonly isPredicted: boolean;
    /** The fields of the result an audited read's event records. */
    readonly audit?: { readonly details: schema.Object<Record<string, schema.Schema>> };
    /** Derive the method's procedure under a name. */
    readonly procedure: (name: string, schema: ObjectSchema) => MethodProcedure;
    /** Change rows for a call where it executes. */
    readonly handler: (context: MethodContext, execution: Execution) => Promise<unknown>;
    /** Execute a call around the handler where it executes. */
    readonly execute: (
        this: Method,
        context: MethodContext,
        execution: Execution,
    ) => Promise<unknown>;
    /** Authorize a call before its external work, where it executes. */
    readonly authorize?: (context: MethodContext, execution: StoreExecution) => Promise<void>;
    /** Derive a created object's first relationships and a new scope's owner, for creations. */
    readonly creation?: (
        context: MethodContext,
        object: sync.ObjectReference,
    ) => Creation | Promise<Creation>;
    /** List the calls a mutation runs before this one as their callers, before the transaction. */
    readonly expand?: (context: MethodContext) => Promise<readonly BranchEntry[]>;
    /** Do external work before the transaction, on the server only. */
    readonly prepare?: (context: MethodContext) => Promise<unknown>;
    /** The builder call a declared method came from, absent for a method a trait derives. */
    readonly source?: MethodSource;
    /** Settle the prepared work once the transaction committed or failed, at least once. */
    readonly settle?: (context: SettlementContext, settled: Settled) => Promise<void>;
    /** Whether the method stays off public routes, run only by the principals' own code. */
    readonly isInternal?: OptionOf<Configuration, "internal", boolean>;
    /** Whether a bulk update takes items, each an object's identifier and revision. */
    readonly isItemized?: OptionOf<Configuration, "items", boolean>;
    /** Refuse an object type the method cannot serve. */
    readonly validate?: (object: ObjectType) => void;
    /** Derive the calls undoing a step. */
    readonly inverse?: (step: Step) => readonly sync.Call[] | undefined;
    /** The code an implementation attaches by phase, without the declared behaviour it wraps. */
    readonly hooks?: PhasedHandler;
}

/** A method as its builder declares it, which an object's own handler or phases can wrap. */
export interface MethodDeclaration<
    Configuration extends MethodConfiguration = MethodConfiguration,
> extends Method<Configuration> {
    /** Wrap the method in a handler or phases, leaving it no builder call source. */
    handle(handler: Handler<Configuration>): MethodDeclaration<Configuration>;
}

/** A method as a trait declares it. */
export type MethodDefinition<Configuration extends MethodConfiguration> = Omit<
    Method<Configuration>,
    typeof METHOD | "execute" | "isPredicted" | "handler" | "authorize"
> & {
    /** Authorize a call as the method types it before its external work, where it executes. */
    readonly authorize?: (
        context: MethodContext<Configuration, PreparedOf<Configuration> | undefined>,
        execution: StoreExecution,
    ) => Promise<void>;
    /** Change rows for a call as the method types it, where it executes. */
    readonly handler: (
        context: MethodContext<Configuration>,
        execution: Execution,
    ) => Promise<unknown>;
    /** Whether clients predict the method, true by default. */
    readonly isPredicted?: boolean;
    /** Execute a call as the method types it around `this.handler`. */
    readonly execute?: (
        this: Method,
        context: MethodContext<Configuration>,
        execution: Execution,
    ) => Promise<unknown>;
};

/** What every method can report about itself. */
export const Method = {
    /** Read the identifier of the object a method returned. */
    resultId(result: unknown): string | undefined {
        const parsed = IDENTIFIED.safeParse(result);

        return parsed.success ? parsed.data.id : undefined;
    },

    /** Decide whether a value is a declared method. */
    is(value: unknown): value is Method {
        return typeof value === "object" && value !== null && METHOD in value;
    },

    /** Decide whether a method loads its target before it runs. */
    loads(declared: Method): boolean {
        return declared.target === undefined || declared.target === "object";
    },

    /** Decide whether a method settles its prepared work. */
    settles(declared: Method): boolean {
        return declared.settle !== undefined;
    },

    /** List the phases an implementation attaches to a method, in the order a call runs them. */
    phases(declared: Method): Phase[] {
        const { hooks } = declared;

        return hooks === undefined ? [] : PHASES.filter((phase) => hooks[phase] !== undefined);
    },

    /** Mark a trait's method code as declared behaviour, which every store runs itself. */
    declared<Configuration extends MethodConfiguration>(
        method: Method<Configuration>,
    ): MethodDeclaration<Configuration> {
        const { hooks: _hooks, ...declared } = method;

        return Method.declaration(declared);
    },

    /** Let a method take an object's own handler or phases. */
    declaration<Configuration extends MethodConfiguration>(
        method: Method<Configuration>,
    ): MethodDeclaration<Configuration> {
        return { ...method, handle: (handler) => Method.handled(method, handler) };
    },

    /** Decide whether a context is one of a method: its target loaded, its caller identified. */
    isContext<Configuration extends MethodConfiguration>(
        declared: Method<Configuration>,
        context: MethodContext,
    ): context is MethodContext<Configuration, PreparedOf<Configuration> | undefined> {
        // require the target a targeted method loads and the identified caller a mutation needs
        const isTargeted = !Method.loads(declared) || context.target !== undefined;
        const isIdentified = !declared.mutates || !Caller.isAnonymous(context.caller);

        // require prepared work the method's schema reads, absent before it is done
        const prepared = declared.prepared;
        const isReady =
            context.prepared === undefined ||
            (prepared !== undefined && prepared.safeParse(context.prepared).success);

        return isTargeted && isIdentified && isReady;
    },

    /** Decide whether a context is one of a method in its transaction, with its prepared work. */
    isPrepared<Configuration extends MethodConfiguration>(
        declared: Method<Configuration>,
        context: MethodContext,
    ): context is MethodContext<Configuration> {
        return (
            Method.isContext(declared, context) &&
            (declared.prepared === undefined || context.prepared !== undefined)
        );
    },

    /** Decide whether a settlement settles the work a method declares. */
    isSettled<Configuration extends MethodConfiguration>(
        declared: Method<Configuration>,
        settled: Settled,
    ): settled is Settled<PreparedOf<Configuration>> {
        const prepared = declared.prepared;

        return settled.prepared === undefined
            ? settled.status === "rolledBack" || prepared === undefined
            : prepared !== undefined && prepared.safeParse(settled.prepared).success;
    },

    /** Decide whether a value is what a method returns: its output, a page, else a row. */
    isResult<Configuration extends MethodConfiguration>(
        declared: Method<Configuration>,
        value: unknown,
    ): value is MethodResult<Configuration> {
        // read the declared output, a listing's page, else the changed row
        if (declared.output !== undefined) {
            return declared.output.safeParse(value).success;
        } else if (declared.kind === "list") {
            return LISTED.safeParse(value).success;
        } else {
            return Row.safeParse(value).success;
        }
    },

    /** Read a call as a method types it before its prepared work, refusing one it cannot take. */
    requireContext<Configuration extends MethodConfiguration>(
        declared: Method<Configuration>,
        context: MethodContext,
    ): MethodContext<Configuration, PreparedOf<Configuration> | undefined> {
        // refuse a call missing its target or its signer
        if (!Method.isContext(declared, context)) {
            throw new TypeError(`${context.object.name}.${context.name} cannot take this call`);
        }

        return context;
    },

    /** Read a call as a method types it in its transaction, refusing one it cannot take. */
    requirePrepared<Configuration extends MethodConfiguration>(
        declared: Method<Configuration>,
        context: MethodContext,
    ): MethodContext<Configuration> {
        // refuse a call missing its target, its signer or its prepared work
        if (!Method.isPrepared(declared, context)) {
            throw new TypeError(`${context.object.name}.${context.name} cannot take this call`);
        }

        return context;
    },

    /** Read a result as a method types it, refusing one the method does not return. */
    requireResult<Configuration extends MethodConfiguration>(
        declared: Method<Configuration>,
        context: MethodContext,
        value: unknown,
    ): MethodResult<Configuration> {
        // refuse a result off the method's output
        if (!Method.isResult(declared, value)) {
            throw new TypeError(
                `${context.object.name}.${context.name} returned an unexpected result`,
            );
        }

        return value;
    },

    /** Read prepared work as a method types it, refusing work off the method's schema. */
    requireSettled<Configuration extends MethodConfiguration>(
        declared: Method<Configuration>,
        settled: Settled,
    ): Settled<PreparedOf<Configuration>> {
        // refuse prepared work off the method's schema
        if (!Method.isSettled(declared, settled)) {
            throw new TypeError("a method settles only the work it declares");
        }

        return settled;
    },

    /** Wrap a method in an object's own handler or phases. */
    handled<Configuration extends MethodConfiguration>(
        declared: Method<Configuration>,
        handler: HandleFunction<Configuration> | PhasedHandler<Configuration>,
    ): MethodDeclaration<Configuration> {
        // normalize the handler, refusing other keys than phases
        const typed: PhasedHandler<Configuration> =
            typeof handler === "function" ? { handle: handler } : handler;
        const unknown = Object.keys(typed).find((key) => !PHASES.some((phase) => phase === key));
        if (unknown !== undefined) {
            throw new TypeError(`a method's handler has no phase ${unknown}`);
        }
        if (typed.settle !== undefined && typed.prepare === undefined) {
            throw new TypeError("a method settles only the work its prepare phase does");
        } else if (typed.prepare !== undefined && declared.prepared === undefined) {
            throw new TypeError("a method prepares only the work it declares");
        } else if (typed.creation !== undefined && declared.creation !== undefined) {
            throw new TypeError("a creation has a creator relation or a creation, not both");
        }

        // run each phase on the call as the method types it
        const handling = Method.guarded(declared, typed);

        // authorize by the method's own authorization first, then by the handler's
        const wrapped = handling.handle;
        const outer = handling.authorize;
        const authorize =
            declared.authorize === undefined && outer === undefined
                ? undefined
                : async (context: MethodContext, execution: StoreExecution) => {
                      await declared.authorize?.(context, execution);
                      await outer?.(context);
                  };
        const { source, ...unsourced } = declared;
        const wrapping: Method<Configuration> = {
            ...unsourced,
            hooks:
                declared.hooks === undefined ? handling : Method.layered(declared.hooks, handling),
            ...(wrapped === undefined
                ? {}
                : {
                      handler: (context: MethodContext, execution: Execution) =>
                          wrapped(context, (changed = context) =>
                              declared.handler(changed, execution),
                          ),
                      // predict a handled custom method its declaration leaves predicted
                      ...(source?.builder === "query" || source?.builder === "mutation"
                          ? { isPredicted: source.arguments[0].isPredicted !== false }
                          : {}),
                  }),
            ...(handling.prepare === undefined
                ? {}
                : { prepare: handling.prepare, isPredicted: false }),
            ...(handling.settle === undefined ? {} : { settle: handling.settle }),
            ...(handling.expand === undefined ? {} : { expand: handling.expand }),
            ...(handling.creation === undefined ? {} : { creation: handling.creation }),
            ...(authorize === undefined ? {} : { authorize }),
        };

        return Method.declaration(wrapping);
    },

    /** Run a handler's phases on any call, each reading the call as its method types it. */
    guarded<Configuration extends MethodConfiguration>(
        declared: Method<Configuration>,
        handler: PhasedHandler<Configuration>,
    ): PhasedHandler {
        const { authorize, creation, expand, prepare, handle, settle } = handler;
        const before = (context: MethodContext) => Method.requireContext(declared, context);

        return {
            ...(authorize === undefined
                ? {}
                : { authorize: (context) => authorize(before(context)) }),
            ...(creation === undefined
                ? {}
                : { creation: (context, object) => creation(before(context), object) }),
            ...(expand === undefined ? {} : { expand: (context) => expand(before(context)) }),
            ...(prepare === undefined ? {} : { prepare: (context) => prepare(before(context)) }),
            ...(handle === undefined
                ? {}
                : {
                      handle: (context, next) =>
                          handle(Method.requirePrepared(declared, context), async (changed) =>
                              Method.requireResult(declared, context, await next(changed)),
                          ),
                  }),
            ...(settle === undefined
                ? {}
                : {
                      settle: (context, settled) =>
                          settle(context, Method.requireSettled(declared, settled)),
                  }),
        };
    },

    /** Layer wrapping phases over wrapped ones: handle wraps, authorize runs the wrapped first. */
    layered(inner: PhasedHandler, outer: PhasedHandler): PhasedHandler {
        // wrap the wrapped handle and authorize, and let the wrapping phases replace the others
        const handle = outer.handle;
        const authorize = outer.authorize;
        const wrapped = inner.handle;

        return {
            ...inner,
            ...outer,
            ...(handle === undefined || wrapped === undefined
                ? {}
                : {
                      handle: (
                          context: MethodContext,
                          next: (context?: MethodContext) => Promise<unknown>,
                      ) => handle(context, (changed = context) => wrapped(changed, next)),
                  }),
            ...(authorize === undefined || inner.authorize === undefined
                ? {}
                : {
                      authorize: async (context: MethodContext) => {
                          await inner.authorize?.(context);
                          await authorize(context);
                      },
                  }),
        };
    },
};

/** Complete a method a trait declares. */
export function defineMethod<Configuration extends MethodConfiguration>(
    definition: MethodDefinition<Configuration>,
): MethodDeclaration<Configuration> {
    // hand the handler each call as the method types it, refused once here where it is not
    const { handler, execute, authorize, ...shared } = definition;
    const declared: MethodDeclaration<Configuration> = Method.declaration({
        ...shared,
        [METHOD]: true,
        isPredicted: definition.isPredicted ?? true,
        handler: (context: MethodContext, execution: Execution) =>
            handler(Method.requirePrepared(declared, context), execution),
        ...(authorize === undefined
            ? {}
            : {
                  authorize: (context: MethodContext, execution: StoreExecution) =>
                      authorize(Method.requireContext(declared, context), execution),
              }),
        execute(this: Method, context: MethodContext, execution: Execution) {
            return execute === undefined
                ? this.handler(context, execution)
                : execute.call(this, Method.requirePrepared(declared, context), execution);
        },
    });

    return declared;
}
