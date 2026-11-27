import { Opacity } from "@destack/style";
import type { JSX } from "@destack/view";
import { NumberField } from "../number-field/index.ts";
import type { NamedValueFieldProperties } from "./value-field.tsx";

/** Render the input of an opacity: a share from none to all. */
export function OpacityInput(properties: NamedValueFieldProperties): JSX.Element {
    // read the opacity, opaque where none is given
    const opacity = (): Opacity => Opacity.safeParse(properties.value).data ?? 1;

    return (
        <NumberField
            aria-label={properties["aria-label"]}
            aria-describedby={properties["aria-describedby"]}
            min={0}
            max={1}
            step={0.01}
            format={{ style: "percent" }}
            disabled={properties.disabled}
            value={opacity()}
            onValueChange={(next) => next !== undefined && properties.onValueChange(next)}
        />
    );
}
