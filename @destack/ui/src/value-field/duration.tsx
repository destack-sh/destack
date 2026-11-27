import { Duration, DurationUnit } from "@destack/style";
import type { JSX } from "@destack/view";
import { UnitParts } from "../dimension-field/index.ts";
import { ValueGroup, valueStyles } from "./group.tsx";
import type { NamedValueFieldProperties } from "./value-field.tsx";

/** No time, in milliseconds. */
const NO_TIME: Duration = { value: 0, unit: "ms" };

/** Render the input of a length of time: its number, and its unit picked beside it. */
export function DurationInput(properties: NamedValueFieldProperties): JSX.Element {
    // read the duration, none where none is given
    const duration = (): Duration => Duration.safeParse(properties.value).data ?? NO_TIME;

    return (
        <ValueGroup
            aria-label={properties["aria-label"]}
            aria-describedby={properties["aria-describedby"]}
            xstyle={valueStyles.row}
        >
            <UnitParts
                units={DurationUnit.options}
                value={duration()}
                disabled={properties.disabled === true}
                onValueChange={(next) => properties.onValueChange(next)}
            />
        </ValueGroup>
    );
}
