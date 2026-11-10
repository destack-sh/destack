import { currentSpace } from "@destack/account/native";
import { present } from "@destack/schema";
import type { Verb } from "../command/command.ts";
import { Context } from "./context.ts";

/** Switch the space command lines act in without --space. */
export function switchCommand(context: Context): Verb {
    return {
        description:
            "Switch the space command lines act in, stored as this computer's space setting",
        arguments: [
            {
                name: "space",
                // offer the spaces of the person's accounts
                complete: async () => (await context.person()).person.spaces(),
            },
        ],
        json: true,
        async run({ arguments: { space: named }, json }) {
            // resolve the space as the person names it, requiring one they can open
            const resolved = await context.resolve(present(named, "the space argument"));
            const space = await (await context.person()).address(resolved);
            const client = await context.native();

            // set it as this computer's space
            await client.set(currentSpace, resolved);
            await context.terminal.print(
                json ? JSON.stringify({ space }) : `Switched to ${space}.`,
            );
        },
    };
}
