import { installation } from "@destack/object";
import { defineScenario } from "@destack/package/declare";
import { machineInteraction } from "./interaction.ts";

/** Publish an app's release. */
export const publishRelease = defineScenario({
    of: installation,
    interaction: machineInteraction,
    name: "publish-release",
    description: "publish an app's release",
    given: { examples: [] },
    parameters: {},
    when: [
        {
            operation: "release.publish",
            times: 1,
            step: { action: "publish", fixture: "apps" },
        },
    ],
});

/** Install a published app in the owner's home. */
export const installApp = defineScenario({
    of: installation,
    interaction: machineInteraction,
    name: "install-app",
    description: "install a published app in the owner's home",
    given: { examples: [] },
    parameters: {},
    when: [
        { action: "publish", fixture: "apps" },
        {
            operation: "app.install",
            times: 1,
            step: { action: "install", fixture: "apps" },
        },
    ],
    then: [{ observe: { count: "team" }, is: 0 }],
});

/** Launch the start page from the browser. */
export const launchApp = defineScenario({
    of: installation,
    interaction: machineInteraction,
    name: "launch-app",
    description: "launch the start page from the browser",
    given: { examples: [] },
    parameters: { launches: 3 },
    when: [
        {
            operation: "app.launch",
            times: "{launches}",
            step: { action: "launch", installation: "start", view: "start" },
        },
    ],
});

/** Update an installed app to its next release. */
export const updateApp = defineScenario({
    of: installation,
    interaction: machineInteraction,
    name: "update-app",
    description: "update an installed app to its next release",
    given: { examples: [] },
    parameters: {},
    when: [
        { action: "publish", fixture: "apps" },
        { action: "install", fixture: "apps" },
        { action: "publish", fixture: "apps-next" },
        {
            operation: "app.update",
            times: 1,
            step: { action: "update", installation: "apps", release: "apps-next" },
        },
    ],
    then: [{ observe: { count: "team" }, is: 0 }],
});
