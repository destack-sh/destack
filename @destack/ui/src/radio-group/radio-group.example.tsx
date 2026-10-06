import { defineExample } from "@destack/package/declare";
import { Label } from "../label/index.ts";
import { RadioGroup, RadioGroupItem } from "./radio-group.tsx";

/** The density choices of a list. */
export const radioGroupListDensity = defineExample({
    of: RadioGroup,
    name: "list-density",
    description: "the density choices of a list",
    render: () => (
        <RadioGroup name="density" aria-label="Density" defaultValue="regular">
            <Label>
                <RadioGroupItem value="compact" /> Compact
            </Label>
            <Label>
                <RadioGroupItem value="regular" /> Regular
            </Label>
        </RadioGroup>
    ),
});

/** The density choices of a list with compact chosen. */
export const radioGroupListDensityCompact = defineExample({
    of: RadioGroup,
    name: "list-density-compact",
    description: "the density choices of a list with compact chosen",
    render: () => (
        <RadioGroup name="density" aria-label="Density" defaultValue="compact">
            <Label>
                <RadioGroupItem value="compact" /> Compact
            </Label>
            <Label>
                <RadioGroupItem value="regular" /> Regular
            </Label>
        </RadioGroup>
    ),
});

/** The density choices of a list unavailable. */
export const radioGroupListDensityDisabled = defineExample({
    of: RadioGroup,
    name: "list-density-disabled",
    description: "the density choices of a list unavailable",
    render: () => (
        <RadioGroup name="density" aria-label="Density" defaultValue="regular" disabled>
            <Label>
                <RadioGroupItem value="compact" /> Compact
            </Label>
            <Label>
                <RadioGroupItem value="regular" /> Regular
            </Label>
        </RadioGroup>
    ),
});
