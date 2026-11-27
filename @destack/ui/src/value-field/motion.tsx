import { Motion } from "@destack/style";
import type { JSX } from "@destack/view";
import { MotionField } from "../motion-field/index.ts";
import { type NamedValueInputProperties, readValue } from "./value-field.tsx";

/** Render the input of a motion as its field. */
export function MotionInput(properties: NamedValueInputProperties): JSX.Element {
    return (
        <MotionField
            label={properties.label}
            aria-label={properties["aria-label"]}
            disabled={properties.disabled}
            value={readValue(properties.value, Motion)}
            onValueChange={(next) => properties.onValueChange(next)}
        />
    );
}
