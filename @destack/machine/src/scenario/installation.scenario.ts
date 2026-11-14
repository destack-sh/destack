import { installation } from "@destack/object";
import { defineScenario } from "@destack/package/declare";
import { machineInteraction } from "./interaction.ts";

/** Publish an app's release and install it in the owner's home until the home applies it. */
export const publishAndInstall = defineScenario({
    of: installation,
    interaction: machineInteraction,
    name: "publish-and-install",
    description:
        "publish an app's release and install it in the owner's home until the home applies it",
    given: { examples: [] },
    parameters: {},
    when: [
        {
            operation: "release.publish",
            times: 1,
            step: { action: "publish", fixture: "apps" },
        },
        {
            operation: "app.install",
            times: 1,
            step: { action: "install", fixture: "apps" },
        },
    ],
    then: [{ observe: { count: "team" }, is: 0 }],
});

/** Launch the start page in the owner's home from the browser, until its document loads. */
export const launchFromBrowser = defineScenario({
    of: installation,
    interaction: machineInteraction,
    name: "launch-from-browser",
    description:
        "launch the start page in the owner's home from the browser until its document loads",
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

/** Publish an app's next release and update its installation in the owner's home until applied. */
export const updateToNewRelease = defineScenario({
    of: installation,
    interaction: machineInteraction,
    name: "update-to-new-release",
    description:
        "publish an app's next release and update its installation in the owner's home until it applies",
    given: { examples: [] },
    parameters: {},
    when: [
        { action: "publish", fixture: "apps" },
        { action: "install", fixture: "apps" },
        {
            operation: "release.publish",
            times: 1,
            step: { action: "publish", fixture: "apps-next" },
        },
        {
            operation: "app.update",
            times: 1,
            step: { action: "update", installation: "apps", release: "apps-next" },
        },
    ],
    then: [{ observe: { count: "team" }, is: 0 }],
});
