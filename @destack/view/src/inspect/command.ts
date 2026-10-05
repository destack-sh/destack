import type { ObjectType } from "@destack/object";
import { graph } from "@destack/package";
import { CommandDescription } from "@destack/package/manifest";
import { schema, type JsonObject } from "@destack/schema";
import type { Command } from "../declare/command.ts";

/** Describe a command by its title, the method it calls and its key combination. */
export function describeCommand<Object extends ObjectType>(
    command: Command<Object>,
): CommandDescription {
    return {
        title: command.title,
        packageId: command.object.package.id,
        type: command.object.name,
        method: command.method,
        ...(command.keybinding === undefined ? {} : { keybinding: command.keybinding }),
    };
}

/** Invoke the object method a command calls. */
export function commandSymbols(input: JsonObject): graph.MemberSymbol[] {
    const command = CommandDescription.parse(input);

    return schema.array(graph.MemberSymbol).parse([
        {
            relationships: [
                {
                    kind: "invokes",
                    symbol: {
                        packageId: command.packageId,
                        kind: "method",
                        name: command.method,
                        parent: { kind: "object", name: command.type },
                    },
                },
            ],
        },
    ]);
}
