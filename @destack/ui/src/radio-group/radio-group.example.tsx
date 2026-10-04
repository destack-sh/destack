import type { JSX } from "@solidjs/web";
import { Label } from "../label/index.ts";
import { RadioGroup, RadioGroupItem } from "./radio-group.tsx";

/** Show the density choices of a list. */
export function RadioGroupExample(): JSX.Element {
    return (
        <RadioGroup name="density" aria-label="Density">
            <Label>
                <RadioGroupItem value="compact" /> Compact
            </Label>
            <Label>
                <RadioGroupItem value="regular" checked /> Regular
            </Label>
        </RadioGroup>
    );
}
