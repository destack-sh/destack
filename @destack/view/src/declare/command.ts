import { DeclarationName, ModuleMetadata, type Package, PackageId } from "@destack/package";
import type { CallableName, ObjectType } from "@destack/object";
import { defineSchema, schema } from "@destack/schema";

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

/** A command's identity across installations: its package and its name. */
export const CommandReference = Object.assign(
    defineSchema(
        schema.object({
            /** The package declaring the command. */
            packageId: PackageId,
            /** The command's name in its package. */
            name: DeclarationName,
        }),
    ),
    {
        /** The key naming a command in settings: its package and name, as `<package>/<name>`. */
        key: defineSchema(schema.templateLiteral([PackageId, "/", DeclarationName])),

        /** Write the key naming a command in settings. */
        format(reference: CommandReference): string {
            return `${reference.packageId}/${reference.name}`;
        },
    },
);
/** A command's identity across installations. */
export type CommandReference = schema.Infer<typeof CommandReference>;
