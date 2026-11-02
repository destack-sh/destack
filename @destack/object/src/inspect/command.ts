import { graph } from "@destack/package";
import { schema } from "@destack/schema";
import type { Command, CommandDescription } from "../declare/index.ts";

/** Describe a command by its title, the call it makes, its condition and its keys. */
export function describeCommand(command: Command): CommandDescription {
    return command.describe();
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
