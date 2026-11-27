import { DateRange } from "@destack/schema";
import type { JSX } from "@destack/view";
import { DatePicker } from "../date-picker/index.ts";
import type { NamedValueFieldProperties } from "./value-field.tsx";

/** Render the input of a range of dates: its first and last day, typed or picked. */
export function DateRangeInput(properties: NamedValueFieldProperties): JSX.Element {
    // read the range, none while it is not one
    const range = (): DateRange | undefined => DateRange.safeParse(properties.value).data;

    return (
        <DatePicker
            mode="range"
            aria-label={properties["aria-label"]}
            disabled={properties.disabled}
            selected={range()}
            onSelect={(selected) => {
                // give a range once it has its last day
                if (selected?.to !== undefined) {
                    properties.onValueChange(DateRange.of(selected.from, selected.to));
                }
            }}
        />
    );
}
