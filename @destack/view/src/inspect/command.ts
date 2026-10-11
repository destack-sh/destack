import type { ObjectType } from "@destack/object";
import { graph } from "@destack/package";
import { CommandDescription } from "@destack/package/manifest";
import { schema } from "@destack/schema";
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
export function commandSymbols(command: Command): graph.MemberSymbol[] {
    return schema.array(graph.MemberSymbol).parse([
        {
            relationships: [
                {
                    kind: "invokes",
                    symbol: {
                        packageId: command.object.package.id,
                        kind: "method",
                        name: command.method,
                        parent: { kind: "object", name: command.object.name },
                    },
                },
            ],
        },
    ]);
}
