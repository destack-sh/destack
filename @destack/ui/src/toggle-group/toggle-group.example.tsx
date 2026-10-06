import { defineExample } from "@destack/package/declare";
import { ToggleGroup, ToggleGroupItem } from "./toggle-group.tsx";

/** The alignment of a paragraph as a single toggle group. */
export const toggleGroupParagraphAlignment = defineExample({
    of: ToggleGroup,
    name: "paragraph-alignment",
    description: "the alignment of a paragraph as a single toggle group",
    render: () => (
        <ToggleGroup type="single" defaultValue="left" variant="outline" aria-label="Alignment">
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
    description:
        "the alignment of a paragraph with none chosen yet and right alignment unavailable",
    render: () => (
        <ToggleGroup type="single" aria-label="Alignment">
            <ToggleGroupItem value="left">Left</ToggleGroupItem>
            <ToggleGroupItem value="center">Center</ToggleGroupItem>
            <ToggleGroupItem value="right" disabled>
                Right
            </ToggleGroupItem>
            <ToggleGroupItem value="justify">Justify</ToggleGroupItem>
        </ToggleGroup>
    ),
});
