import { currentSpace } from "@destack/account/native";
import { present } from "@destack/schema";
import type { CommandBody } from "../command/command.ts";

/** Switch the space command lines act in without --space, or list the spaces to switch to. */
export const switchCommand: CommandBody = {
    arguments: [
        {
            name: "space",
            default: "",
            // offer the spaces of the person's accounts and those shared with them
            complete: async (context) => (await context.launcher()).spaces(),
        },
    ],
    isJson: true,
    async run({ arguments: { space: named }, isJson: json }, context) {
        // list the spaces without one, marking the current one, as git branch does
        const launcher = await context.launcher();
        if (named === "") {
            const spaces = await launcher.spaces();
            const current = await context.space();
            const address = current === undefined ? undefined : await launcher.address(current);
            const rows = spaces.map((space) => ({ space, isCurrent: space === address }));
            await context.terminal.print(
                json
                    ? JSON.stringify(rows)
                    : rows.map((row) => `${row.isCurrent ? "*" : " "} ${row.space}`).join("\n"),
            );

            return;
        }

        // resolve the space as the person names it, requiring one they can open
        const resolved = await context.resolve(present(named, "the space argument"));
        const space = await launcher.address(resolved);
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
