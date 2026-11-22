import { defineExample } from "@destack/package/declare";
import { Label } from "../label/index.ts";
import { Switch, SwitchThumb } from "./switch.tsx";

/** A switch that turns notifications on and off. */
export const switchNotifications = defineExample({
    of: Switch,
    name: "notifications",
    description: "a switch that turns notifications on and off",
    properties: {
        name: "notifications",
        defaultChecked: true,
    },
    render: (properties) => (
        <Label>
            <Switch {...properties} /> Notifications
        </Label>
    ),
});

/** The notifications switch unavailable. */
export const switchNotificationsDisabled = defineExample({
    of: Switch,
    name: "notifications-disabled",
    description: "the notifications switch unavailable",
    properties: {
        name: "notifications",
        checked: true,
        disabled: true,
    },
    render: (properties) => (
        <Label>
            <Switch {...properties} /> Notifications
        </Label>
    ),
});

/** A switch composed from its root and thumb parts. */
export const switchComposed = defineExample({
    of: Switch,
    name: "composed",
    description: "a switch composed from its root and thumb parts",
    properties: {
        name: "airplane-mode",
    },
    render: (properties) => (
        <Label>
            <Switch {...properties}>
                <SwitchThumb />
            </Switch>{" "}
            Airplane mode
        </Label>
    ),
});
