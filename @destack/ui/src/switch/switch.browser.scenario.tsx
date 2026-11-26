import { t } from "@destack/locale";
import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { Switch } from "./switch.tsx";
import { switchNotifications } from "./switch.example.tsx";

/** The switch of the notifications example. */
const notifications = { role: "switch", name: "Notifications" };

/** Turn a switch off with Space and back on with a press. */
export const switchToggleWithSpaceAndPress = defineScenario({
    of: Switch,
    interaction: viewInteraction,
    name: "toggleWithSpaceAndPress",
    description: t`Turn a switch off with Space and back on with a press.`,
    given: { examples: [switchNotifications] },
    parameters: {},
    when: [
        {
            step: { action: "focus", target: notifications },
            then: [
                { observe: { kind: "state", target: notifications, state: "checked" }, is: true },
            ],
        },
        {
            step: { action: "press", key: " " },
            then: [
                { observe: { kind: "state", target: notifications, state: "checked" }, is: false },
            ],
        },
        {
            step: { action: "click", target: notifications },
            then: [
                { observe: { kind: "state", target: notifications, state: "checked" }, is: true },
            ],
        },
    ],
});
