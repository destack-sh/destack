import { defineExample } from "@destack/package/declare";
import { Icon } from "@destack/ui/icon";
import { Toggle } from "./toggle.tsx";

/** A toggle that turns bold text on and off. */
export const toggleBold = defineExample({
    of: Toggle,
    name: "bold",
    description: "a toggle that turns bold text on and off",
    properties: {
        variant: "outline",
        "aria-label": "Bold",
    },
    render: (properties) => (
        <Toggle {...properties}>
            <Icon name="text-b" />
        </Toggle>
    ),
});

/** The bold toggle turned on. */
export const toggleBoldPressed = defineExample({
    of: Toggle,
    name: "bold-pressed",
    description: "the bold toggle turned on",
    properties: {
        variant: "outline",
        "aria-label": "Bold",
        defaultPressed: true,
    },
    render: (properties) => (
        <Toggle {...properties}>
            <Icon name="text-b" />
        </Toggle>
    ),
});

/** The bold toggle unavailable. */
export const toggleBoldDisabled = defineExample({
    of: Toggle,
    name: "bold-disabled",
    description: "the bold toggle unavailable",
    properties: {
        variant: "outline",
        "aria-label": "Bold",
        disabled: true,
    },
    render: (properties) => (
        <Toggle {...properties}>
            <Icon name="text-b" />
        </Toggle>
    ),
});
