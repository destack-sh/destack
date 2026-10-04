import { DeclarationName, ModuleMetadata, type Package } from "@destack/package";
import type { CallableName, ObjectType } from "@destack/object";

/** A command as its package defines it: a named call of one object type's method. */
export interface CommandDefinition<Object extends ObjectType = ObjectType> {
    /** The name, unique among the package's commands. */
    readonly name: string;
    /** The title the command menu shows. */
    readonly title: string;
    /** The object type the command calls. */
    readonly object: Object;
    /** The method the command calls. */
    readonly method: CallableName<Object>;
    /** The key combination running it, in VS Code's keybinding syntax such as `mod+shift+a`. */
    readonly keybinding?: string;
}

/** A command a package declares, offered by the shell in its command menu, menus, shortcuts, the terminal and agents. */
export class Command<Object extends ObjectType = ObjectType> {
    /** The declaring package. */
    readonly package: Package;
    /** The name. */
    readonly name: string;
    /** The title the command menu shows. */
    readonly title: string;
    /** The object type the command calls. */
    readonly object: Object;
    /** The method the command calls. */
    readonly method: CallableName<Object>;
    /** The key combination running it. */
    readonly keybinding: string | undefined;

    /** Keep a declared command. */
    constructor(owner: Package, definition: CommandDefinition<Object>) {
        // keep the package and the definition
        this.package = owner;
        this.name = definition.name;
        this.title = definition.title;
        this.object = definition.object;
        this.method = definition.method;
        this.keybinding = definition.keybinding;
    }
}

/** Declare a command: a named call of an object type's method. */
export function defineCommand<const Object extends ObjectType>(
    definition: CommandDefinition<Object>,
    module?: ModuleMetadata,
): Command<Object> {
    // stamp the declaring package and validate the name and method
    const owner = ModuleMetadata.require(module, "defineCommand").package;
    DeclarationName.parse(definition.name);
    if (definition.object.methods[definition.method]?.isSystem === true) {
        throw new TypeError(
            `command ${definition.name} calls the system method ${definition.method}`,
        );
    }

    return new Command(owner, definition);
}
