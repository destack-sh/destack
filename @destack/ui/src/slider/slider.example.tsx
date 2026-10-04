import type { JSX } from "@solidjs/web";
import { Slider } from "./slider.tsx";

/** Show a slider that sets the font size of a note. */
export function SliderExample(): JSX.Element {
    return <Slider name="size" min={12} max={24} step={1} value={16} aria-label="Font size" />;
}
