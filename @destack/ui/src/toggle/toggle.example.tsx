import { t } from "@destack/locale";
import { defineExample } from "@destack/package/declare";
import { Icon } from "@destack/ui/icon";
import { Toggle } from "./toggle.tsx";

/** A toggle that turns bold text on and off. */
export const toggleBold = defineExample({
    of: Toggle,
    name: "bold",
    description: t`A toggle that turns bold text on and off.`,
    properties: {
        variant: "outline",
        isIconOnly: true,
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
    name: "boldPressed",
    description: t`The bold toggle turned on.`,
    properties: {
        variant: "outline",
        isIconOnly: true,
        "aria-label": "Bold",
        defaultPressed: true,
    },
    render: (properties) => (
        <Toggle {...properties}>
            <Icon name="text-b" />
        </Toggle>
    ),
});
