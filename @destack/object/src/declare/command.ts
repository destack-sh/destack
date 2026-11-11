import { Condition } from "@destack/db";
import { Message } from "@destack/locale";
import {
    defineDeclarationKind,
    DeclarationName,
    ModuleMetadata,
    Package,
    PackageError,
    PackageId,
} from "@destack/package";
import { type JsonObject, JsonValue, schema } from "@destack/schema";
import type { CallableName, MethodInput } from "../method/index.ts";
import type { MethodName } from "../method/procedure.ts";
import type { ObjectType, PermissionOf } from "../object/object.ts";
import type { ConditionOf } from "../query/index.ts";
import { Shortcut, type ShortcutText } from "./shortcut.ts";

/** The method kinds that remove their target, for good unless its type deletes to a trash. */
const DELETING_KINDS: ReadonlySet<string> = new Set(["delete", "deleteMany"]);

/** The input fields a command never asks for: the request, and the revision a change expects. */
const FILLED_FIELDS: ReadonlySet<string> = new Set(["requestId", "revision"]);

/** The input of the method a command calls, as a client gives it. */
type CalledInput<Object extends ObjectType, Method extends CallableName<Object>> = MethodInput<
    Object,
    Method & MethodName<Object>
>;

/** A command as its package defines it: a titled call of an object type's method. */
export interface CommandDefinition<
    Object extends ObjectType = ObjectType,
    Method extends CallableName<Object> = CallableName<Object>,
    Preset extends object = {},
    Keys extends string = Shortcut,
> {
    /** The name, unique among the package's commands. */
    readonly name: string;
    /** The title menus, the command palette and the command line show. */
    readonly title: Message;
    /** The group the command palette shows the title under, such as `Git` in `Git: Commit`. */
    readonly category?: Message;
    /** The object type the command calls. */
    readonly object: Object;
    /** The method the command calls. */
    readonly method: Method;
    /** The input fields the command presets, the caller giving the rest and the target. */
    readonly input?: Preset &
        Partial<CalledInput<Object, Method>> & {
            readonly [Field in Exclude<keyof Preset, keyof CalledInput<Object, Method>>]: never;
        };
    /** The condition the target meets for the command to apply, any target without one. */
    readonly when?: ConditionOf<Object, Object>;
    /** The key combination that runs it, in the keybinding syntax editors write. */
    readonly keybinding?: string extends Keys ? Shortcut : Keys & ShortcutText<Keys>;
}

/** What a command acts on: one object it targets, or its scope, as `create` does. */
export type CommandTarget = "object" | "scope";

/** The input a caller gives a command: the fields it does not preset, with the target's `id`. */
export type CommandInput<
    Object extends ObjectType,
    Method extends CallableName<Object>,
    Preset,
> = Omit<MethodInput<Object, Method & MethodName<Object>>, keyof Preset>;

/** The query of a command's targets: those meeting its condition that the person may call it on. */
export interface CommandQuery<Object extends ObjectType> {
    /** The condition the targets meet. */
    readonly where: ConditionOf<Object, Object>;
    /** The permission the person has on each target, the method's. */
    readonly permission: PermissionOf<Object>;
}

/** A command: a titled call of an object type's method with some input preset. */
export class Command<
    Object extends ObjectType = ObjectType,
    Method extends CallableName<Object> = CallableName<Object>,
    Preset extends object = {},
> {
    /** The declaring package. */
    readonly package: Package;
    /** The name. */
    readonly name: string;
    /** The title menus, the command palette and the command line show. */
    readonly title: Message;
    /** The group the command palette shows the title under, absent for none. */
    readonly category: Message | undefined;
    /** The object type the command calls. */
    readonly object: Object;
    /** The method the command calls. */
    readonly method: Method;
    /** The input fields the command presets. */
    readonly preset: JsonObject;
    /** The condition the target meets for the command to apply, absent for any target. */
    readonly when: ConditionOf<Object, Object> | undefined;
    /** The key combination running it, absent for none. */
    readonly keybinding: Shortcut | undefined;

    /** Create a command of a package, refusing a condition on a method without a target. */
    constructor(owner: Package, definition: CommandDefinition<Object, Method, Preset>) {
        // set the package and the definition, presetting no input by default
        this.package = owner;
        this.name = DeclarationName.parse(definition.name);
        this.title = definition.title;
        this.category = definition.category;
        this.object = definition.object;
        this.method = definition.method;
        this.preset = JsonValue.object(definition.input ?? {});
        this.when = definition.when;
        this.keybinding = definition.keybinding;

        // refuse a condition where no target exists to meet it
        if (this.when !== undefined && this.target === "scope") {
            throw PackageError.of(
                "DEFINITION_INVALID",
                `command '${this.name}' is invalid: it has a condition, but '${this.object.name}.${this.method}' acts on its scope`,
            );
        }
        // refuse a key combination editors do not write
        else if (this.keybinding !== undefined && !Shortcut.safeParse(this.keybinding).success) {
            throw PackageError.of(
                "DEFINITION_INVALID",
                `key combination '${this.keybinding}' of command '${this.name}' is invalid: editors do not write it`,
            );
        }
    }

    /** What the command acts on: the method's target object, or its scope. */
    get target(): CommandTarget {
        return this.object.method(this.method).target === "scope" ? "scope" : "object";
    }

    /** Whether the command removes its target for good, which people confirm first. */
    get isDestructive(): boolean {
        const { kind } = this.object.method(this.method);

        return kind === "purge" || (DELETING_KINDS.has(kind) && !this.object.isTrashable());
    }

    /** The JSON Schema of the input a caller gives beside the target and the preset fields. */
    get parameters(): JsonObject {
        // read the method's input as callers write it
        const described = this.object.describeMethods()[this.method];
        if (described === undefined) {
            throw new TypeError(`${this.object.name} has no method '${this.method}' to call`);
        }
        const input = InputSchema.parse(described.input);

        // leave out the target, the scope, what the command presets and what calls fill
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

    /** Query the targets the command applies to: every one, or those among some objects. */
    query(ids?: readonly string[]): CommandQuery<Object> {
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
    input(given: CommandInput<Object, Method, Preset>): JsonObject {
        return { ...this.preset, ...JsonValue.object(given) };
    }

    /** Describe the command by its title, the call it makes, its condition and its keys. */
    describe(): CommandDescription {
        return {
            package: this.package,
            name: this.name,
            title: this.title,
            ...(this.category === undefined ? {} : { category: this.category }),
            packageId: this.object.package.id,
            type: this.object.name,
            method: this.method,
            input: this.preset,
            ...(this.when === undefined ? {} : { when: Condition.schema.parse(this.when) }),
            ...(this.keybinding === undefined ? {} : { keybinding: this.keybinding }),
        };
    }

    /** Read a command an installation declares from its name, description and called type. */
    static of(name: string, description: CommandDescription, type: ObjectType): Command {
        // refuse a type other than the one the description calls
        if (type.package.id !== description.packageId || type.name !== description.type) {
            throw new TypeError(`command '${name}' calls ${description.type}, not ${type.name}`);
        }

        return new Command(description.package, {
            name,
            title: description.title,
            ...(description.category === undefined ? {} : { category: description.category }),
            object: type,
            method: description.method,
            input: description.input,
            ...(description.when === undefined ? {} : { when: description.when }),
            ...(description.keybinding === undefined ? {} : { keybinding: description.keybinding }),
        });
    }
}

/** Declare a command: a titled call of an object type's method with some input preset. */
export function defineCommand<
    const Object extends ObjectType,
    const Method extends CallableName<Object>,
    const Preset extends object = {},
    const Keys extends string = never,
>(
    definition: CommandDefinition<Object, Method, Preset, Keys>,
    module?: ModuleMetadata,
): Command<Object, Method, Preset> {
    const owner = ModuleMetadata.require(module, "defineCommand").package;

    return new Command(owner, definition);
}

/** A command's identity across installations: its package and its name. */
export const CommandReference = Object.assign(
    schema.object({
        /** The package declaring the command. */
        packageId: PackageId,
        /** The command's name in its package. */
        name: DeclarationName,
    }),
    {
        /** The key of a command in settings: its package and name, as `<package>/<name>`. */
        key: schema.templateLiteral([PackageId, "/", DeclarationName]),

        /** Write the key of a command in settings. */
        format(reference: CommandReference): string {
            return `${reference.packageId}/${reference.name}`;
        },
    },
);
/** A command's identity across installations. */
export type CommandReference = schema.Infer<typeof CommandReference>;

/** A titled call of an object type's method, as menus and the command palette offer it. */
export const CommandDescription = schema.object({
    /** The package declaring the command. */
    package: Package,
    /** The name, unique among the package's commands. */
    name: schema.string().min(1),
    /** The command's title. */
    title: Message,
    /** The group the command palette shows the title under. */
    category: Message.exactOptional(),
    /** The package declaring the object type. */
    packageId: PackageId,
    /** The object type the command calls. */
    type: schema.string().min(1),
    /** The method the command calls. */
    method: schema.string().min(1),
    /** The input fields the command presets. */
    input: schema.jsonObject(),
    /** The condition the target meets for the command to apply, absent for any target. */
    when: Condition.schema.exactOptional(),
    /** The key combination running it. */
    keybinding: Shortcut.exactOptional(),
});
/** A titled call of an object type's method, as its declaration describes it. */
export type CommandDescription = schema.Infer<typeof CommandDescription>;

/** The command declaration kind, which object defines. */
export const COMMAND_KIND = defineDeclarationKind("command", CommandDescription);

/** A method's input as JSON Schema: an object with its properties and the required ones. */
const InputSchema = schema.looseObject({
    /** The properties by name. */
    properties: schema.record(schema.string(), schema.json()).exactOptional(),
    /** The names of the required properties. */
    required: schema.array(schema.string()).exactOptional(),
});
