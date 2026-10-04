import { Icon } from "@destack/icon";
import type { JSX } from "@solidjs/web";
import { Toggle } from "./toggle.tsx";

/** Show a toggle that turns bold text on and off. */
export function ToggleExample(): JSX.Element {
    return (
        <Toggle variant="outline" aria-label="Bold">
            <Icon name="text-b" />
        </Toggle>
    );
}
