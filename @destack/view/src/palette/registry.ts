import type { Localization, Message } from "@destack/locale";
import { Shortcut } from "@destack/object";
import { DeclarationName } from "@destack/package";
import type { ObjectReference } from "@destack/sync";
import { type JsonObject, JsonValue, schema, toJsonSchema } from "@destack/schema";

/** A command a running view registers, as the palette over it lists it. */
export const RegisteredCommand = schema.object({
    /** The name, unique among the view's commands. */
    name: DeclarationName,
    /** The title in the person's language. */
    title: schema.string().min(1),
    /** The group the palette shows the title under, in the person's language. */
    category: schema.string().min(1).exactOptional(),
    /** The JSON Schema of the input the palette asks for, absent for none. */
    input: schema.jsonObject().exactOptional(),
    /** The key combination the view gives it, absent for none. */
    keybinding: Shortcut.exactOptional(),
});
/** A command a running view registers. */
export type RegisteredCommand = schema.Infer<typeof RegisteredCommand>;

/** A command a running view registers: what the palette shows of it, and what it runs. */
interface CommandFields {
    /** The name, unique among the view's commands. */
    readonly name: string;
    /** The title menus and the command palette show. */
    readonly title: Message;
    /** The group the palette shows the title under, such as `Git` in `Git: Commit`. */
    readonly category?: Message;
    /** The key combination that runs it, in the keybinding syntax editors write. */
    readonly keybinding?: Shortcut;
}

/** A command a running view registers, running with the input the palette asks for or with none. */
export type CommandRegistration<
    Input extends schema.Schema<JsonObject> = schema.Schema<JsonObject>,
> = CommandFields &
    (
        | {
              /** The schema of the input the palette asks for. */
              readonly input: Input;
              /** Run the command in the page with its input. */
              readonly run: (input: schema.Infer<Input>) => Promise<void> | void;
          }
        | {
              /** No input to ask for. */
              readonly input?: undefined;
              /** Run the command in the page. */
              readonly run: () => Promise<void> | void;
          }
    );

/** A registered command, running with input its schema parses. */
interface Registered extends CommandFields {
    /** The schema of the input the palette asks for, absent for none. */
    readonly input: schema.Schema<JsonObject> | undefined;
    /** Run the command in the page with the input the palette gives. */
    readonly run: (input: JsonObject) => Promise<void> | void;
}

/** The commands a running view's components register, by name, and the objects they act on. */
export class CommandRegistry {
    /** The registered commands, by name. */
    readonly #commands: Map<string, Registered>;
    /** Read the objects the person selected in the page, absent while no component tracks them. */
    #selection: (() => readonly ObjectReference[]) | undefined;

    /** Register no command and no selection yet. */
    constructor() {
        this.#commands = new Map<string, Registered>();
        this.#selection = undefined;
    }

    /** Register a command, refusing a name taken, and return what unregisters it. */
    register<Input extends schema.Schema<JsonObject>>(
        command: CommandRegistration<Input>,
    ): () => void {
        // refuse keys editors do not write, or a name another component registers
        DeclarationName.parse(command.name);
        if (command.keybinding !== undefined && !Shortcut.safeParse(command.keybinding).success) {
            throw new TypeError(
                `command '${command.name}' has keys editors do not write: '${command.keybinding}'`,
            );
        } else if (this.#commands.has(command.name)) {
            throw new TypeError(`the view already registers a command '${command.name}'`);
        }

        // store the command with a run parsing its input by its schema, or taking none
        const { name, title, category, keybinding } = command;
        const fields = {
            name,
            title,
            ...(category === undefined ? {} : { category }),
            ...(keybinding === undefined ? {} : { keybinding }),
        };
        const registered: Registered = takesInput(command)
            ? {
                  ...fields,
                  input: command.input,
                  run: (input) => command.run(command.input.parse(input)),
              }
            : { ...fields, input: undefined, run: () => command.run() };
        this.#commands.set(name, registered);

        return () => {
            if (this.#commands.get(name) === registered) {
                this.#commands.delete(name);
            }
        };
    }

    /** List the registered commands as the palette shows them, in the person's language. */
    list(localization: Localization): RegisteredCommand[] {
        return [...this.#commands.values()].map(({ name, title, category, input, keybinding }) => ({
            name,
            title: localization.render(title),
            ...(category === undefined ? {} : { category: localization.render(category) }),
            ...(input === undefined ? {} : { input: JsonValue.object(toJsonSchema(input)) }),
            ...(keybinding === undefined ? {} : { keybinding }),
        }));
    }

    /** Track the objects the person selects, refusing a second component tracking them. */
    select(read: () => readonly ObjectReference[]): () => void {
        // refuse two components tracking one selection
        if (this.#selection !== undefined) {
            throw new TypeError("the view already tracks a selection");
        }
        this.#selection = read;

        return () => {
            if (this.#selection === read) {
                this.#selection = undefined;
            }
        };
    }

    /** Read the objects the person selected in the page, none while no component tracks them. */
    selection(): ObjectReference[] {
        return [...(this.#selection?.() ?? [])];
    }

    /** Run a registered command by name with the input the palette asked for. */
    async run(name: string, input: JsonObject): Promise<void> {
        const command = this.#commands.get(name);
        if (command === undefined) {
            throw new TypeError(`the view registers no command '${name}'`);
        }

        await command.run(input);
    }
}

/** Report whether a registered command asks for input. */
function takesInput<Input extends schema.Schema<JsonObject>>(
    command: CommandRegistration<Input>,
): command is Extract<CommandRegistration<Input>, { readonly input: Input }> {
    return command.input !== undefined;
}
