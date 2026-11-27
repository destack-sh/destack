import { CubicBezier, Easing, EASINGS } from "@destack/style";
import type { JSX } from "@destack/view";
import { choiceType } from "./group.tsx";
import { type NamedValueFieldProperties, ValueField } from "./value-field.tsx";

/** Render the input of an easing: the curves CSS names, and a custom one where given. */
export function CubicBezierInput(properties: NamedValueFieldProperties): JSX.Element {
    // read the curve, an ease out where none is given, and offer it where CSS names it not
    const curve = (): CubicBezier =>
        CubicBezier.safeParse(properties.value).data ?? EASINGS["ease-out"].curve;
    const type = () => {
        const named = choiceType(Easing.options, EASINGS);
        const custom = CubicBezier.css(curve());

        return CubicBezier.easing(curve()) === undefined
            ? { ...named, choices: [...named.choices, { value: custom }] }
            : named;
    };
    const give = (key: unknown): void => {
        const easing = Easing.safeParse(key).data;
        if (easing !== undefined) {
            properties.onValueChange([...EASINGS[easing].curve]);
        }
    };

    return (
        <ValueField
            type={type()}
            aria-label={properties["aria-label"]}
            aria-describedby={properties["aria-describedby"]}
            required
            disabled={properties.disabled}
            value={CubicBezier.css(curve())}
            onValueChange={give}
        />
    );
}
