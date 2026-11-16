import { Message } from "@destack/locale";
import { CommandReference, Shortcut, type ShortcutText } from "@destack/object";
import {
    DeclarationName,
    defineKind,
    ModuleMetadata,
    PackageVersion,
    PackageError,
} from "@destack/package";
import { schema } from "@destack/schema";

/** Where focus may be for an action's keys to run it: outside form fields, or anywhere. */
export const ActionFocus = schema.enum(["page", "any"]);
/** Where focus may be for a key combination to run an action. */
export type ActionFocus = schema.Infer<typeof ActionFocus>;

/** An action as its package defines it: an operation of an open view. */
export interface ActionDefinition<Keys extends string = Shortcut> {
    /** The name, unique among the package's actions and commands. */
    readonly name: string;
    /** The title menus and the command palette show. */
    readonly title: Message;
    /** The group the palette and menus show the title under, such as `Edit` in `Edit: Undo`. */
    readonly category?: Message;
    /** The key combination that runs it, in the keybinding syntax editors write. */
    readonly keybinding?: string extends Keys ? Shortcut : Keys & ShortcutText<Keys>;
    /** Where focus may be for the keys to run it, outside form fields by default. */
    readonly focus?: ActionFocus;
}

/** An operation of an open view that a mounted component handles. */
export class Action {
    /** The declaring package. */
    readonly package: PackageVersion;
    /** The name. */
    readonly name: string;
    /** The title menus and the command palette show. */
    readonly title: Message;
    /** The group the palette and menus show the title under, absent for none. */
    readonly category: Message | undefined;
    /** The key combination that runs it, absent for none. */
    readonly keybinding: Shortcut | undefined;
    /** Where focus may be for the keys to run it. */
    readonly focus: ActionFocus;

    /** Create an action of a package, refusing a name or keys editors do not write. */
    constructor(
        owner: PackageVersion,
        definition: Omit<ActionDefinition, "keybinding"> & {
            readonly keybinding?: Shortcut;
        },
    ) {
        // refuse an invalid name or key combination
        DeclarationName.parse(definition.name);
        if (
            definition.keybinding !== undefined &&
            !Shortcut.safeParse(definition.keybinding).success
        ) {
            throw PackageError.of(
                "DEFINITION_INVALID",
                `key combination '${definition.keybinding}' of action '${definition.name}' is invalid: editors do not write it`,
            );
        }

        // copy the package and the definition
        this.package = owner;
        this.name = definition.name;
        this.title = definition.title;
        this.category = definition.category;
        this.keybinding = definition.keybinding;
        this.focus = definition.focus ?? "page";
    }

    /** The action's identity, shared with its package's commands: its package and name. */
    get reference(): CommandReference {
        return { packageId: this.package.id, name: DeclarationName.parse(this.name) };
    }

    /** Describe the action by its title and keys. */
    describe(): ActionDescription {
        return {
            package: this.package,
            name: this.name,
            title: this.title,
            ...(this.category === undefined ? {} : { category: this.category }),
            ...(this.keybinding === undefined ? {} : { keybinding: this.keybinding }),
            focus: this.focus,
        };
    }
}

/** An operation of an open view, as menus and the command palette offer it. */
export const ActionDescription = schema.object({
    /** The package declaring the action. */
    package: PackageVersion,
    /** The name, unique among the package's actions and commands. */
    name: schema.string().min(1),
    /** The action's title. */
    title: Message,
    /** The group the palette and menus show the title under. */
    category: Message.exactOptional(),
    /** The key combination running it. */
    keybinding: Shortcut.exactOptional(),
    /** Where focus may be for the keys to run it. */
    focus: ActionFocus,
});
/** An operation of an open view, as its declaration describes it. */
export type ActionDescription = schema.Infer<typeof ActionDescription>;

/** The action declaration kind, which view defines. */
export const ACTION_KIND = defineKind({ name: "action", description: ActionDescription });

/** Declare an action: an operation of an open view that a mounted component handles. */
export function defineAction<const Keys extends string = never>(
    definition: ActionDefinition<Keys>,
    module?: ModuleMetadata,
): Action {
    const owner = ModuleMetadata.require(module, "defineAction").package;

    return new Action(owner, definition);
}
