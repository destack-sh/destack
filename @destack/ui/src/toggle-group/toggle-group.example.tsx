import type { JSX } from "@solidjs/web";
import { ToggleGroup, ToggleGroupItem } from "./toggle-group.tsx";

/** Show the alignment of a paragraph as a single toggle group. */
export function ToggleGroupExample(): JSX.Element {
    return (
        <ToggleGroup type="single" defaultValue="left" variant="outline" aria-label="Alignment">
            <ToggleGroupItem value="left">Left</ToggleGroupItem>
            <ToggleGroupItem value="center">Center</ToggleGroupItem>
            <ToggleGroupItem value="right">Right</ToggleGroupItem>
        </ToggleGroup>
    );
}
