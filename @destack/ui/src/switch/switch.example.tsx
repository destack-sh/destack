import { defineExample } from "@destack/package/declare";
import { Label } from "../label/index.ts";
import { Switch } from "./switch.tsx";

/** A switch that turns notifications on and off. */
export const switchNotifications = defineExample({
    of: Switch,
    name: "notifications",
    description: "a switch that turns notifications on and off",
    render: () => (
        <Label>
            <Switch name="notifications" checked /> Notifications
        </Label>
    ),
});

/** The notifications switch unavailable. */
export const switchNotificationsDisabled = defineExample({
    of: Switch,
    name: "notifications-disabled",
    description: "the notifications switch unavailable",
    render: () => (
        <Label>
            <Switch name="notifications" checked disabled /> Notifications
        </Label>
    ),
});
