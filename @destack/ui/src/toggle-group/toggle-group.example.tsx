import { t } from "@destack/locale";
import { defineExample } from "@destack/package/declare";
import { ToggleGroup, ToggleGroupItem } from "./toggle-group.tsx";

/** The alignment of a paragraph as a single toggle group. */
export const toggleGroupParagraphAlignment = defineExample({
    of: ToggleGroup,
    name: "paragraphAlignment",
    description: t`The alignment of a paragraph as a single toggle group.`,
    properties: {
        defaultValue: "left",
        variant: "outline",
        "aria-label": "Alignment",
    },
    render: (properties) => (
        <ToggleGroup {...properties}>
            <ToggleGroupItem value="left">Left</ToggleGroupItem>
            <ToggleGroupItem value="center">Center</ToggleGroupItem>
            <ToggleGroupItem value="right">Right</ToggleGroupItem>
        </ToggleGroup>
    ),
});

/** The alignment of a paragraph with none chosen yet and right alignment unavailable. */
export const toggleGroupUnaligned = defineExample({
    of: ToggleGroup,
    name: "unaligned",
    description: t`The alignment of a paragraph with none chosen yet and right alignment unavailable.`,
    properties: {
        "aria-label": "Alignment",
    },
    render: (properties) => (
        <ToggleGroup {...properties}>
            <ToggleGroupItem value="left">Left</ToggleGroupItem>
            <ToggleGroupItem value="center">Center</ToggleGroupItem>
            <ToggleGroupItem value="right" disabled>
                Right
            </ToggleGroupItem>
            <ToggleGroupItem value="justify">Justify</ToggleGroupItem>
        </ToggleGroup>
    ),
});

/** A size picker as a segmented track, its chosen segment raised. */
export const toggleGroupSegmented = defineExample({
    of: ToggleGroup,
    name: "segmented",
    description: t`A size picker as a segmented track, its chosen segment raised.`,
    properties: {
        defaultValue: "default",
        variant: "segmented",
        "aria-label": "Size",
    },
    render: (properties) => (
        <ToggleGroup {...properties}>
            <ToggleGroupItem value="sm">Small</ToggleGroupItem>
            <ToggleGroupItem value="default">Default</ToggleGroupItem>
            <ToggleGroupItem value="lg">Large</ToggleGroupItem>
        </ToggleGroup>
    ),
});
