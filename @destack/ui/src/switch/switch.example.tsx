import type { JSX } from "@solidjs/web";
import { Label } from "../label/index.ts";
import { Switch } from "./switch.tsx";

/** Show a switch that turns notifications on and off. */
export function SwitchExample(): JSX.Element {
    return (
        <Label>
            <Switch name="notifications" checked /> Notifications
        </Label>
    );
}
