import { t } from "@destack/locale";
import { installation } from "@destack/stack/object";
import { defineScenario } from "@destack/package/declare";
import { machineInteraction } from "./interaction.ts";

/** Publish an app's release. */
export const publishRelease = defineScenario({
    of: installation,
    interaction: machineInteraction,
    name: "publishRelease",
    description: t`Publish an app's release.`,
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
    name: "installApp",
    description: t`Install a published app in the owner's home.`,
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
    name: "launchApp",
    description: t`Launch the start page from the browser.`,
    given: { examples: [] },
    parameters: { launches: 3 },
    when: [
        {
            operation: "app.launch",
            times: "{launches}",
            step: { action: "launch", installation: "home", view: "start" },
        },
    ],
});

/** Update an installed app to its next release. */
export const updateApp = defineScenario({
    of: installation,
    interaction: machineInteraction,
    name: "updateApp",
    description: t`Update an installed app to its next release.`,
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
