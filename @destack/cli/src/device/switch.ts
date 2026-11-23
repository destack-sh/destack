import { currentSpace } from "@destack/account/native";
import { present } from "@destack/schema";
import type { CommandBody } from "../command/command.ts";

/** Switch the space command lines act in without --space. */
export const switchCommand: CommandBody = {
    arguments: [
        {
            name: "space",
            // offer the spaces of the person's accounts
            complete: async (context) => (await context.launcher()).spaces(),
        },
    ],
    isJson: true,
    async run({ arguments: { space: named }, isJson: json }, context) {
        // resolve the space as the person names it, requiring one they can open
        const resolved = await context.resolve(present(named, "the space argument"));
        const space = await (await context.launcher()).address(resolved);
        const client = await context.native();

        // set it as this computer's space
        await client.set(currentSpace, resolved);
        await context.terminal.print(
            json
                ? JSON.stringify({ space })
                : context.terminal.event("Switched", "done", `to space '${space}'`),
        );
    },
};
