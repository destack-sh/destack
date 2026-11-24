import type { Message, Shortcut } from "@destack/locale";
import {
    DeclarationName,
    ModuleMetadata,
    type PackageVersion,
    PackageError,
} from "@destack/package";
import { DeclarationReference } from "@destack/package/declare";
import { type JsonObject, mapValues, schema } from "@destack/schema";
import type { Moment } from "@destack/db";
import type { ObjectTypeReference } from "@destack/sync";
import { Casing } from "../object/name.ts";
import type { ObjectType } from "../object/object.ts";
import type { ActionCall } from "./call.ts";
import type { ActionDescription } from "./description.ts";
import { ActionInput } from "./input.ts";
import { ActionKeybinding, type ActionKeybindingDefinition } from "./keybinding.ts";
import {
    type ActionInputDefinition,
    type ActionInputs,
    type ActionTemplate,
    ContextVariable,
    ObjectInput,
    Variable,
    type VariablesOf,
} from "./variable.ts";

/** The declared inputs of an action, as a type only. */
declare const INPUTS: unique symbol;

/** Where focus may be for an action's keys to run it: outside form fields, or anywhere. */
export const ActionFocus = schema.enum(["page", "any"]);
/** Where focus may be for a key combination to run an action. */
export type ActionFocus = schema.Infer<typeof ActionFocus>;

/** An action as its package defines it: an operation calling a method, or one a view handles. */
export interface ActionDefinition<
    Inputs extends ActionInputs = ActionInputs,
    Keys extends string = Shortcut,
    Mac extends string = Keys,
> {
    /** The name, unique among the package's actions. */
    readonly name: string;
    /** The title menus, the command palette and the command line show. */
    readonly title: Message;
    /** The sentence for an action with its inputs, such as a confirmation, its title by default. */
    readonly summary?: Message<keyof Inputs & string>;
    /** The group the command palette shows the title under, such as `Git` in `Git: Commit`. */
    readonly category?: Message;
    /** The further words a search finds the action by. */
    readonly keywords?: Message;
    /** The key combinations that run it until a person chooses others. */
    readonly keybinding?: ActionKeybindingDefinition<Keys, Mac>;
    /** Where focus may be for the keys to run it, outside form fields by default. */
    readonly focus?: ActionFocus;
    /** Whether it removes something, which menus mark, a method's own by default. */
    readonly isDestructive?: boolean;
    /** The inputs, by name, none by default. */
    readonly input?: Inputs;
    /** The method it calls, reading its inputs, absent for an action the open view handles. */
    readonly call?: (variables: VariablesOf<Inputs>) => ActionTemplate;
}

/** An operation people invoke from menus, the command palette, keys or a notification. */
export class Action<Inputs extends ActionInputs = ActionInputs> {
    /** The inputs as declared, as a type only. */
    declare readonly [INPUTS]?: Inputs;
    /** The declaring package. */
    readonly package: PackageVersion;
    /** The name, unique among the package's actions. */
    readonly name: string;
    /** The title menus, the command palette and the command line show. */
    readonly title: Message;
    /** The sentence for an action with its inputs, absent to use the title. */
    readonly summary: Message | undefined;
    /** The group the command palette shows the title under, absent for none. */
    readonly category: Message | undefined;
    /** The further words a search finds the action by, absent for none. */
    readonly keywords: Message | undefined;
    /** The key combinations running it until a person chooses others, absent for none. */
    readonly keybinding: ActionKeybinding | undefined;
    /** Where focus may be for the keys to run it. */
    readonly focus: ActionFocus;
    /** Whether it removes something, which menus mark. */
    readonly isDestructive: boolean;
    /** The inputs people and programs give it. */
    readonly inputs: readonly ActionInput[];
    /** The method it calls, absent for an action the open view handles. */
    readonly call: ActionCall | undefined;

    /** Create an action of a package, refusing a name or keys editors do not write. */
    constructor(
        owner: PackageVersion,
        definition: Omit<ActionDefinition<Inputs>, "call" | "input">,
        call: ActionCall | undefined,
        inputs: readonly ActionInput[] = call?.inputs ?? [],
    ) {
        // set the package and the definition
        this.package = owner;
        this.name = DeclarationName.parse(definition.name);
        this.title = definition.title;
        this.summary = definition.summary;
        this.category = definition.category;
        this.keywords = definition.keywords;
        this.keybinding =
            definition.keybinding === undefined
                ? undefined
                : ActionKeybinding.of(definition.keybinding);
        this.focus = definition.focus ?? "page";
        this.isDestructive = definition.isDestructive ?? call?.isDestructive ?? false;
        this.inputs = inputs;
        this.call = call;

        // refuse key combinations editors do not write
        if (this.keybinding !== undefined && !ActionKeybinding.safeParse(this.keybinding).success) {
            const { key, mac } = this.keybinding;
            const keys = [...new Set([...key, ...mac])].join("', '");
            throw PackageError.of(
                "DEFINITION_INVALID",
                `key combinations '${keys}' of action '${this.name}' are invalid: editors do not write them`,
            );
        }
    }

    /** The action's identity: its package and name. */
    get reference(): DeclarationReference {
        return { packageId: this.package.id, name: DeclarationName.parse(this.name) };
    }

    /** The word for the action on its object, as a command line writes it. */
    get verb(): string {
        return Casing.kebab(this.name.slice(this.name.lastIndexOf(".") + 1));
    }

    /** Build the name of an action on a type from its verb, such as `task.moveTo`. */
    static nameOf(type: ObjectType, verb: string): string {
        return `${Casing.camel(type.name)}.${verb}`;
    }

    /** Report whether the action applies to a row at a moment, by its call's condition. */
    applies(row: JsonObject, moment: Moment): boolean {
        return this.call?.applies(row, moment) ?? true;
    }

    /** Describe the action by its title, its keys, its inputs and the call it makes. */
    describe(): ActionDescription {
        // leave out what its method says of itself
        const { call, inputs, isDestructive } = this;
        const isOwn = isDestructive !== (call?.isDestructive ?? false);
        const isAsking = call === undefined && inputs.length > 0;

        return {
            package: this.package,
            name: this.name,
            title: this.title,
            ...(this.summary === undefined ? {} : { summary: this.summary }),
            ...(this.category === undefined ? {} : { category: this.category }),
            ...(this.keywords === undefined ? {} : { keywords: this.keywords }),
            ...(this.keybinding === undefined ? {} : { keybinding: this.keybinding }),
            focus: this.focus,
            ...(isOwn ? { isDestructive } : {}),
            ...(isAsking ? { inputs: inputs.map((each) => each.describe()) } : {}),
            ...(call === undefined ? {} : { call: call.describe() }),
        };
    }

    /** Read an action an installation declares from its name and description, finding its types. */
    static of(
        name: string,
        description: ActionDescription,
        typeOf: (reference: ObjectTypeReference) => ObjectType,
    ): Action {
        // find the type it calls, and the types its inputs take
        const { package: owner, call, inputs = [], ...definition } = description;
        const type =
            call === undefined ? undefined : typeOf({ packageId: call.packageId, type: call.type });
        const described = call === undefined ? undefined : type?.describedCall(name, call, typeOf);

        return new Action(
            owner,
            { ...definition, name },
            described,
            described?.inputs ?? inputs.map((each) => ActionInput.from(each, typeOf)),
        );
    }
}

/** Declare an action: an operation people invoke, calling a method or handled by the open view. */
export function defineAction<
    const Inputs extends ActionInputs = {},
    const Keys extends string = never,
    const Mac extends string = never,
>(definition: ActionDefinition<Inputs, Keys, Mac>, module?: ModuleMetadata): Action<Inputs> {
    // take the declaring package and the call apart from the rest
    const owner = ModuleMetadata.require(module, "defineAction").package;
    const { call, input, ...declared } = definition;

    // write the call as a template over the inputs' variables
    const inputs: ActionInputs = input ?? {};
    const template = call?.(mapValues(input ?? {}, variableOf));
    if (template !== undefined) {
        requireName(definition.name, template);
    }
    const taken = Object.entries(inputs).map(([name, each]) => ActionInput.of(name, each));

    return new Action<Inputs>(
        owner,
        declared,
        template?.object.actionCall(definition.name, inputs, template),
        taken,
    );
}

/** Refuse an action whose name leaves out the type it calls, or is another method's of it. */
function requireName(name: string, { object, method }: ActionTemplate): void {
    // refuse a name without the type
    const prefix = Action.nameOf(object, "");
    const verb = name.slice(prefix.length);
    if (!name.startsWith(prefix)) {
        throw PackageError.of(
            "DEFINITION_INVALID",
            `action '${name}' is invalid: it calls '${object.name}', so its name starts with '${prefix}'`,
        );
    }
    // refuse the name of another method's own action
    else if (verb !== method && Object.hasOwn(object.methods, verb)) {
        throw PackageError.of(
            "DEFINITION_INVALID",
            `action '${name}' is invalid: it calls '${method}', but has the name of method '${verb}'`,
        );
    }
}

/** Build the variable a call reads an input by, with the fields of its object or value. */
function variableOf(definition: ActionInputDefinition, name: string): Variable {
    if (definition instanceof ObjectInput) {
        return definition.object.variable(name);
    } else if (definition instanceof ContextVariable) {
        return definition.variable(name);
    }

    return new Variable(name);
}
