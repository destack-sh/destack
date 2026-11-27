import { Overflow, OVERFLOWS } from "@destack/style";
import type { JSX } from "@destack/view";
import { choiceType } from "./group.tsx";
import { type NamedValueFieldProperties, ValueField } from "./value-field.tsx";

/** The type of an overflow, a choice of its values. */
const TYPE = choiceType(Overflow.options, OVERFLOWS);

/** Render the input of an overflow: what a box does with content past its edges. */
export function OverflowInput(properties: NamedValueFieldProperties): JSX.Element {
    return <ValueField {...properties} type={TYPE} required />;
}
