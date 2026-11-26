import { t } from "@destack/locale";
import { defineExample } from "@destack/package/declare";
import { Label } from "../label/index.ts";
import { Switch, SwitchThumb } from "./switch.tsx";

/** A switch that turns notifications on and off. */
export const switchNotifications = defineExample({
    of: Switch,
    name: "notifications",
    description: t`A switch that turns notifications on and off.`,
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

/** A switch composed from its root and thumb parts. */
export const switchComposed = defineExample({
    of: Switch,
    name: "composed",
    description: t`A switch composed from its root and thumb parts.`,
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
