import { defineExample } from "@destack/package/declare";
import { Label } from "../label/index.ts";
import { RadioGroup, RadioGroupIndicator, RadioGroupItem } from "./radio-group.tsx";

/** The density choices of a list. */
export const radioGroupListDensity = defineExample({
    of: RadioGroup,
    name: "list-density",
    description: "the density choices of a list",
    properties: {
        name: "density",
        "aria-label": "Density",
        defaultValue: "regular",
    },
    render: (properties) => (
        <RadioGroup {...properties}>
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
    properties: {
        name: "density",
        "aria-label": "Density",
        defaultValue: "compact",
    },
    render: (properties) => (
        <RadioGroup {...properties}>
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
    properties: {
        name: "density",
        "aria-label": "Density",
        defaultValue: "regular",
        disabled: true,
    },
    render: (properties) => (
        <RadioGroup {...properties}>
            <Label>
                <RadioGroupItem value="compact" /> Compact
            </Label>
            <Label>
                <RadioGroupItem value="regular" /> Regular
            </Label>
        </RadioGroup>
    ),
});

/** The density choices composed from each radio and its indicator. */
export const radioGroupListDensityComposed = defineExample({
    of: RadioGroup,
    name: "list-density-composed",
    description: "the density choices composed from each radio and its indicator",
    properties: {
        name: "density",
        "aria-label": "Density",
        defaultValue: "regular",
    },
    render: (properties) => (
        <RadioGroup {...properties}>
            <Label>
                <RadioGroupItem value="compact">
                    <RadioGroupIndicator />
                </RadioGroupItem>{" "}
                Compact
            </Label>
            <Label>
                <RadioGroupItem value="regular">
                    <RadioGroupIndicator />
                </RadioGroupItem>{" "}
                Regular
            </Label>
        </RadioGroup>
    ),
});
