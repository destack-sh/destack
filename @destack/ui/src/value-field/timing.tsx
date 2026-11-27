import { TimingFunction } from "@destack/style";
import type { JSX } from "@destack/view";
import { TimingField } from "../timing-field/index.ts";
import { type NamedValueInputProperties, readValue } from "./value-field.tsx";

/** Render the input of a timing function as its field. */
export function TimingFunctionInput(properties: NamedValueInputProperties): JSX.Element {
    return (
        <TimingField
            label={properties.label}
            aria-label={properties["aria-label"]}
            disabled={properties.disabled}
            value={readValue(properties.value, TimingFunction)}
            onValueChange={(next) => properties.onValueChange(next)}
        />
    );
}
