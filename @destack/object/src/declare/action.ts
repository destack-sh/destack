import { Condition, type Moment } from "@destack/db";
import { Message, t } from "@destack/locale";
import {
    defineKind,
    DeclarationName,
    ModuleMetadata,
    PackageVersion,
    PackageError,
    PackageId,
} from "@destack/package";
import { DeclarationReference } from "@destack/package/declare";
import { type JsonObject, JsonValue, schema } from "@destack/schema";
import type { CallableName, MethodInput, MethodKind } from "../method/index.ts";
import type { MethodName } from "../method/procedure.ts";
import type { ObjectType, PermissionOf } from "../object/object.ts";
import type { ConditionOf } from "../query/index.ts";
import { Shortcut, type ShortcutText } from "./shortcut.ts";

/** The titles of the built-in methods people run on objects from a list. */
const METHOD_TITLES: Readonly<Partial<Record<MethodKind, Message>>> = {
    update: t`Edit`,
    delete: t`Delete`,
    archive: t`Archive`,
    unarchive: t`Unarchive`,
    restore: t`Restore`,
    purge: t`Delete for good`,
};

/** A property of a method's input as JSON Schema: its type and its choices. */
const PROPERTY = schema.looseObject({
    type: schema.unknown().exactOptional(),
    enum: schema.array(schema.string()).exactOptional(),
});

/** The method kinds that remove their target, for good unless its type has a trash. */
const DELETING_KINDS: ReadonlySet<string> = new Set(["delete", "deleteMany"]);

/** The input fields an action never asks for: the request, and the revision a change expects. */
const FILLED_FIELDS: ReadonlySet<string> = new Set(["requestId", "revision"]);

/** Where focus may be for an action's keys to run it: outside form fields, or anywhere. */
export const ActionFocus = schema.enum(["page", "any"]);
/** Where focus may be for a key combination to run an action. */
export type ActionFocus = schema.Infer<typeof ActionFocus>;

/** One input a person gives an action, as in Raycast: its name, kind and choices. */
export type ActionArgument =
    | {
          readonly name: string;
          readonly kind: "text" | "number" | "boolean";
          readonly isRequired: boolean;
      }
    | {
          readonly name: string;
          readonly kind: "choice";
          readonly choices: readonly string[];
          readonly isRequired: boolean;
      };

/** The input of the method an action calls, as a client gives it. */
type CalledInput<Object extends ObjectType, Method extends CallableName<Object>> = MethodInput<
    Object,
    Method & MethodName<Object>
>;

/** The method call an action makes, as its package defines it. */
export interface ActionCallDefinition<
    Object extends ObjectType = ObjectType,
    Method extends CallableName<Object> = CallableName<Object>,
    Preset extends object = {},
> {
    /** The object type the action calls. */
    readonly object: Object;
    /** The method the action calls. */
    readonly method: Method;
    /** The input fields the action presets, the caller giving the rest and the target. */
    readonly input?: Preset &
        Partial<CalledInput<Object, Method>> & {
            readonly [Field in Exclude<keyof Preset, keyof CalledInput<Object, Method>>]: never;
        };
    /** The condition the target meets for the action to apply, any target without one. */
    readonly when?: ConditionOf<Object, Object>;
}

/** An action as its package defines it: an operation calling a method, or one a view handles. */
export interface ActionDefinition<
    Object extends ObjectType = ObjectType,
    Method extends CallableName<Object> = CallableName<Object>,
    Preset extends object = {},
    Keys extends string = Shortcut,
> {
    /** The name, unique among the package's actions. */
    readonly name: string;
    /** The title menus, the command palette and the command line show. */
    readonly title: Message;
    /** The group the command palette shows the title under, such as `Git` in `Git: Commit`. */
    readonly category?: Message;
    /** The key combination that runs it, in the keybinding syntax editors write. */
    readonly keybinding?: string extends Keys ? Shortcut : Keys & ShortcutText<Keys>;
    /** Where focus may be for the keys to run it, outside form fields by default. */
    readonly focus?: ActionFocus;
    /** The method it calls, absent for an action the open view handles. */
    readonly call?: ActionCallDefinition<Object, Method, Preset>;
}

/** What an action's call acts on: one object it targets, or its scope, as `create` does. */
export type ActionTarget = "object" | "scope";

/** The input a caller gives an action's call: the fields it leaves, with the target's `id`. */
export type ActionInput<
    Object extends ObjectType,
    Method extends CallableName<Object>,
    Preset,
> = Omit<MethodInput<Object, Method & MethodName<Object>>, keyof Preset>;

/** The query of an action's targets: those meeting its condition that the person may call it on. */
export interface ActionQuery<Object extends ObjectType> {
    /** The condition the targets meet. */
    readonly where: ConditionOf<Object, Object>;
    /** The permission the person has on each target, the method's. */
    readonly permission: PermissionOf<Object>;
}

/** The method call an action makes: an object type's method, with some input preset. */
export class ActionCall<
    Object extends ObjectType = ObjectType,
    Method extends CallableName<Object> = CallableName<Object>,
    Preset extends object = {},
> {
    /** The object type the action calls. */
    readonly object: Object;
    /** The method the action calls. */
    readonly method: Method;
    /** The input fields the action presets. */
    readonly preset: JsonObject;
    /** The condition the target meets for the action to apply, absent for any target. */
    readonly when: ConditionOf<Object, Object> | undefined;

    /** Call a method, refusing a condition on a method without a target. */
    constructor(name: string, definition: ActionCallDefinition<Object, Method, Preset>) {
        // set the call, presetting no input by default
        this.object = definition.object;
        this.method = definition.method;
        this.preset = JsonValue.object(definition.input ?? {});
        this.when = definition.when;

        // refuse a condition where no target exists to meet it
        if (this.when !== undefined && this.target === "scope") {
            throw PackageError.of(
                "DEFINITION_INVALID",
                `action '${name}' is invalid: it has a condition, but '${this.object.name}.${this.method}' acts on its scope`,
            );
        }
    }

    /** What the call acts on: the method's target object, or its scope. */
    get target(): ActionTarget {
        return this.object.method(this.method).target === "scope" ? "scope" : "object";
    }

    /** Whether the call removes its target for good, which people confirm first. */
    get isDestructive(): boolean {
        return ActionCall.isDestructive(this.object, this.method);
    }

    /** Report whether the call applies to a row at a moment, by its condition. */
    applies(row: JsonObject, moment: Moment): boolean {
        return (
            this.when === undefined ||
            Condition.matches(Condition.json(this.when), this.object.table, row, moment)
        );
    }

    /** Report whether a type's method removes its target for good, which people confirm first. */
    static isDestructive(object: ObjectType, method: string): boolean {
        const { kind } = object.method(method);

        // count a delete as final unless the type has a trash: a controller's deletion removes for good
        const isRecoverable = object.lifecycle.recoverable !== undefined;

        return kind === "purge" || (DELETING_KINDS.has(kind) && !isRecoverable);
    }

    /** The JSON Schema of the input a caller gives beside the target and the preset fields. */
    get parameters(): JsonObject {
        // read the method's input as callers write it
        const described = this.object.describeMethods()[this.method];
        if (described === undefined) {
            throw new TypeError(`${this.object.name} has no method '${this.method}' to call`);
        }
        const input = InputSchema.parse(described.input);

        // leave out the target, the scope, what the action presets and what calls fill
        const route = this.object.route.field;
        const left = new Set([
            ...FILLED_FIELDS,
            "id",
            ...(route === undefined ? [] : [route]),
            ...Object.keys(this.preset),
        ]);
        const properties = Object.entries(input.properties ?? {}).filter(
            ([name]) => !left.has(name),
        );
        const required = (input.required ?? []).filter((name) => !left.has(name));

        return { ...input, properties: Object.fromEntries(properties), required };
    }

    /** List the inputs a person gives the call, scalar ones alone: structure goes as JSON. */
    get arguments(): ActionArgument[] {
        // read each property's type and choices, and whether it is required
        const parameters = InputSchema.parse(this.parameters);
        const required = new Set(parameters.required ?? []);

        return Object.entries(parameters.properties ?? {}).flatMap(
            ([name, value]): ActionArgument[] => {
                // read a choice, a number, a boolean or text, leaving out structure
                const property = PROPERTY.parse(value);
                const isRequired = required.has(name);
                if (property.enum !== undefined) {
                    return [{ name, kind: "choice", choices: property.enum, isRequired }];
                } else if (property.type === "number" || property.type === "integer") {
                    return [{ name, kind: "number", isRequired }];
                } else if (property.type === "boolean") {
                    return [{ name, kind: "boolean", isRequired }];
                } else if (property.type === "string") {
                    return [{ name, kind: "text", isRequired }];
                }

                return [];
            },
        );
    }

    /** Query the targets the call applies to: every one, or those among some objects. */
    query(ids?: readonly string[]): ActionQuery<Object> {
        // match the condition, and the given objects alone when some are given
        const target: ConditionOf<Object, Object> =
            ids === undefined ? {} : { id: { in: [...ids] } };
        const where = this.when === undefined ? target : { AND: [target, this.when] };

        // require the method's permission on each target
        const { permission } = this.object.method(this.method);
        if (permission === undefined || !this.object.isPermission(permission)) {
            throw new TypeError(`'${this.object.name}.${this.method}' declares no permission`);
        }

        return { where, permission };
    }

    /** Write the method's input: the preset fields, then the caller's, with the target. */
    input(given: ActionInput<Object, Method, Preset>): JsonObject {
        return { ...this.preset, ...JsonValue.object(given) };
    }

    /** Describe the call by its type, method, preset input and condition. */
    describe(): ActionCallDescription {
        return {
            packageId: this.object.package.id,
            type: this.object.name,
            method: this.method,
            input: this.preset,
            ...(this.when === undefined ? {} : { when: Condition.schema.parse(this.when) }),
        };
    }
}

/** An operation people invoke from menus, the command palette, keys or a notification. */
export class Action<
    Object extends ObjectType = ObjectType,
    Method extends CallableName<Object> = CallableName<Object>,
    Preset extends object = {},
> {
    /** The declaring package. */
    readonly package: PackageVersion;
    /** The name, unique among the package's actions. */
    readonly name: string;
    /** The title menus, the command palette and the command line show. */
    readonly title: Message;
    /** The group the command palette shows the title under, absent for none. */
    readonly category: Message | undefined;
    /** The key combination running it, absent for none. */
    readonly keybinding: Shortcut | undefined;
    /** Where focus may be for the keys to run it. */
    readonly focus: ActionFocus;
    /** The method it calls, absent for an action the open view handles. */
    readonly call: ActionCall<Object, Method, Preset> | undefined;

    /** Create an action of a package, refusing a name or keys editors do not write. */
    constructor(owner: PackageVersion, definition: ActionDefinition<Object, Method, Preset>) {
        // set the package and the definition
        this.package = owner;
        this.name = DeclarationName.parse(definition.name);
        this.title = definition.title;
        this.category = definition.category;
        this.keybinding = definition.keybinding;
        this.focus = definition.focus ?? "page";
        this.call =
            definition.call === undefined ? undefined : new ActionCall(this.name, definition.call);

        // refuse a key combination editors do not write
        if (this.keybinding !== undefined && !Shortcut.safeParse(this.keybinding).success) {
            throw PackageError.of(
                "DEFINITION_INVALID",
                `key combination '${this.keybinding}' of action '${this.name}' is invalid: editors do not write it`,
            );
        }
    }

    /** The action's identity: its package and name. */
    get reference(): DeclarationReference {
        return { packageId: this.package.id, name: DeclarationName.parse(this.name) };
    }

    /** Whether the action removes its target for good, which people confirm first. */
    get isDestructive(): boolean {
        return this.call?.isDestructive ?? false;
    }

    /** Report whether the action applies to a row at a moment, by its call's condition. */
    applies(row: JsonObject, moment: Moment): boolean {
        return this.call?.applies(row, moment) ?? true;
    }

    /** List the actions on some rows: the declared calls applying, then one per titled method. */
    static available(
        object: ObjectType,
        rows: readonly JsonObject[],
        declared: readonly Action[],
        moment: Moment,
    ): Action[] {
        // take the declared calls on the type's objects applying to every row
        const own = declared.filter(
            (action) =>
                action.call?.object === object &&
                action.call.target === "object" &&
                rows.every((row) => action.applies(row, moment)),
        );

        // derive an action per titled method clients run on an object, left out by the declared ones
        const covered = new Set(
            declared.flatMap((action) =>
                action.call?.object === object ? [action.call.method] : [],
            ),
        );
        const derived = Object.keys(object.methods).flatMap((name) => {
            // title the method when clients call it on an object
            const title = Action.#titleOf(object, name, rows);

            return covered.has(name) || !Action.#isClientMethod(object, name) || title === undefined
                ? []
                : [new Action(object.package, { name, title, call: { object, method: name } })];
        });

        return [...own, ...derived];
    }

    /** Report whether clients call a method on an object. */
    static #isClientMethod(object: ObjectType, name: string): boolean {
        const { target, visibility = "client" } = object.method(name);

        return target !== "scope" && visibility === "client";
    }

    /** Title a method people run on objects, absent for one they do not run from a list. */
    static #titleOf(
        object: ObjectType,
        name: string,
        rows: readonly JsonObject[],
    ): Message | undefined {
        // title a transition by the state it enters
        const method = object.method(name);
        const machine = Object.entries(object.fields).find(
            ([, field]) => field.machine?.transitions[name] !== undefined,
        );
        const transition = machine?.[1].machine?.transitions[name];
        if (method.kind === "transition") {
            const state = machine?.[0];
            const isLeft =
                state !== undefined &&
                rows.every((row) => {
                    const value = row[state];

                    return typeof value === "string" && transition?.from.includes(value) === true;
                });

            return transition !== undefined && isLeft ? t`Move to ${transition.to}` : undefined;
        }

        // title a custom method by its declaration, and the built-in ones by their kind
        return method.kind === "custom" ? method.title : METHOD_TITLES[method.kind];
    }

    /** Describe the action by its title, its keys and the call it makes. */
    describe(): ActionDescription {
        return {
            package: this.package,
            name: this.name,
            title: this.title,
            ...(this.category === undefined ? {} : { category: this.category }),
            ...(this.keybinding === undefined ? {} : { keybinding: this.keybinding }),
            focus: this.focus,
            ...(this.call === undefined ? {} : { call: this.call.describe() }),
        };
    }

    /** Read an action an installation declares from its name, description and called type. */
    static of(name: string, description: ActionDescription, type?: ObjectType): Action {
        // refuse a type other than the one the description calls
        const { call } = description;
        if (
            call !== undefined &&
            (type?.package.id !== call.packageId || type.name !== call.type)
        ) {
            throw new TypeError(
                `action '${name}' calls ${call.type}, not ${type?.name ?? "nothing"}`,
            );
        }

        return new Action(description.package, {
            name,
            title: description.title,
            ...(description.category === undefined ? {} : { category: description.category }),
            ...(description.keybinding === undefined ? {} : { keybinding: description.keybinding }),
            focus: description.focus,
            ...(call === undefined || type === undefined
                ? {}
                : {
                      call: {
                          object: type,
                          method: call.method,
                          input: call.input,
                          ...(call.when === undefined ? {} : { when: call.when }),
                      },
                  }),
        });
    }
}

/** Declare an action: an operation people invoke, calling a method or handled by the open view. */
export function defineAction<
    const Object extends ObjectType = ObjectType,
    const Method extends CallableName<Object> = CallableName<Object>,
    const Preset extends object = {},
    const Keys extends string = never,
>(
    definition: ActionDefinition<Object, Method, Preset, Keys>,
    module?: ModuleMetadata,
): Action<Object, Method, Preset> {
    const owner = ModuleMetadata.require(module, "defineAction").package;

    return new Action(owner, definition);
}

/** The method call an action makes, as its declaration describes it. */
export const ActionCallDescription = schema.object({
    /** The package declaring the object type. */
    packageId: PackageId,
    /** The object type the action calls. */
    type: schema.string().min(1),
    /** The method the action calls. */
    method: schema.string().min(1),
    /** The input fields the action presets. */
    input: schema.jsonObject(),
    /** The condition the target meets for the action to apply, absent for any target. */
    when: Condition.schema.exactOptional(),
});
/** The method call an action makes, as its declaration describes it. */
export type ActionCallDescription = schema.Infer<typeof ActionCallDescription>;

/** An operation people invoke, as menus and the command palette offer it. */
export const ActionDescription = schema.object({
    /** The package declaring the action. */
    package: PackageVersion,
    /** The name, unique among the package's actions. */
    name: schema.string().min(1),
    /** The action's title. */
    title: Message,
    /** The group the command palette shows the title under. */
    category: Message.exactOptional(),
    /** The key combination running it. */
    keybinding: Shortcut.exactOptional(),
    /** Where focus may be for the keys to run it. */
    focus: ActionFocus,
    /** The method it calls, absent for an action the open view handles. */
    call: ActionCallDescription.exactOptional(),
});
/** An operation people invoke, as its declaration describes it. */
export type ActionDescription = schema.Infer<typeof ActionDescription>;

/** The action declaration kind, which object defines. */
export const ACTION_KIND = defineKind({ name: "action", description: ActionDescription });

/** A method's input as JSON Schema: an object with its properties and the required ones. */
const InputSchema = schema.looseObject({
    /** The properties by name. */
    properties: schema.record(schema.string(), schema.json()).exactOptional(),
    /** The names of the required properties. */
    required: schema.array(schema.string()).exactOptional(),
});
