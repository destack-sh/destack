import { t } from "@destack/locale";
import { defineExample } from "@destack/package/declare";
import { Label } from "../label/index.ts";
import { RadioGroup, RadioGroupIndicator, RadioGroupItem } from "./radio-group.tsx";

/** The density choices of a list. */
export const radioGroupListDensity = defineExample({
    of: RadioGroup,
    name: "listDensity",
    description: t`The density choices of a list.`,
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

/** The density choices of a list, the spacious radio set on its own. */
export const radioGroupItemDefault = defineExample({
    of: RadioGroupItem,
    name: "default",
    description: t`The density choices of a list, the spacious radio set on its own.`,
    properties: { disabled: true },
    render: (properties) => (
        <RadioGroup name="density" aria-label="Density" defaultValue="regular">
            <Label>
                <RadioGroupItem value="regular" /> Regular
            </Label>
            <Label>
                <RadioGroupItem value="spacious" {...properties} /> Spacious
            </Label>
        </RadioGroup>
    ),
});

/** The density choices composed from each radio and its indicator. */
export const radioGroupListDensityComposed = defineExample({
    of: RadioGroup,
    name: "listDensityComposed",
    description: t`The density choices composed from each radio and its indicator.`,
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
