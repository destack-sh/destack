import { defineScenario } from "@destack/package/declare";
import { desktopInteraction } from "../scenario/interaction.ts";
import { Desktop } from "./desktop.ts";

/** Start the desktop on a laptop until it opens its first window, quit it, and start it again. */
export const startQuitAndRestart = defineScenario({
    of: Desktop,
    interaction: desktopInteraction,
    name: "start-quit-and-restart",
    description:
        "start the desktop on a laptop until its first window opens, quit it, and start it again",
    given: { examples: [] },
    parameters: {},
    when: [
        { operation: "desktop.start", step: { action: "start" } },
        { operation: "desktop.quit", step: { action: "quit" } },
        { operation: "desktop.restart", step: { action: "start" } },
    ],
    then: [{ observe: { count: "window" }, is: 2 }],
});
