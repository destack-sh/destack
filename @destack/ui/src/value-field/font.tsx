import {
    FONT_STYLES,
    FONT_WEIGHTS,
    FontFamily,
    FontFeatures,
    FontStyle,
    FontVariations,
    FontWeight,
} from "@destack/style";
import type { JSX } from "@destack/view";
import { Input } from "../input/index.ts";
import { choiceType } from "./group.tsx";
import { TagsParts } from "./tags.tsx";
import { type NamedValueFieldProperties, ValueField } from "./value-field.tsx";

/** The fonts an input starts at, the browser's sans serif. */
const START_FAMILY: FontFamily = ["sans-serif"];

/** The weight an input starts at, a regular one. */
const START_WEIGHT: FontWeight = 400;

/** The type of a font style, a choice of its values. */
const STYLE_TYPE = choiceType(FontStyle.options, FONT_STYLES);

/** Render the input of a font family: the fonts as CSS lists them, the first one first. */
export function FontFamilyInput(properties: NamedValueFieldProperties): JSX.Element {
    // read the fonts, the browser's sans serif where none are given
    const families = (): FontFamily => FontFamily.safeParse(properties.value).data ?? START_FAMILY;

    return (
        <Input
            aria-label={properties["aria-label"]}
            aria-describedby={properties["aria-describedby"]}
            disabled={properties.disabled}
            value={families().join(", ")}
            onChange={(event) => {
                const next = FontFamily.read(event.currentTarget.value);
                if (next.length > 0) {
                    properties.onValueChange(next);
                }
            }}
        />
    );
}

/** Render the input of a font weight: the weights fonts name, and any other it is given. */
export function FontWeightInput(properties: NamedValueFieldProperties): JSX.Element {
    // read the weight, a regular one where none is given, and offer it where no name fits
    const weight = (): FontWeight => FontWeight.safeParse(properties.value).data ?? START_WEIGHT;
    const type = () => {
        const named = Object.values(FONT_WEIGHTS).map((each) => ({
            value: String(each.weight),
            title: each.title,
        }));
        const isNamed = named.some((each) => each.value === String(weight()));

        return {
            kind: "choice" as const,
            choices: isNamed ? named : [...named, { value: String(weight()) }],
        };
    };
    const give = (key: unknown): void => {
        const next = FontWeight.safeParse(Number(key)).data;
        if (next !== undefined) {
            properties.onValueChange(next);
        }
    };

    return (
        <ValueField
            type={type()}
            aria-label={properties["aria-label"]}
            aria-describedby={properties["aria-describedby"]}
            required
            disabled={properties.disabled}
            value={String(weight())}
            onValueChange={give}
        />
    );
}

/** Render the input of a font style: upright, italic or oblique. */
export function FontStyleInput(properties: NamedValueFieldProperties): JSX.Element {
    return <ValueField {...properties} type={STYLE_TYPE} required />;
}

/** Render the input of OpenType features: each feature's tag and setting. */
export function FontFeaturesInput(properties: NamedValueFieldProperties): JSX.Element {
    // read the features, none where none are given
    const features = (): FontFeatures => FontFeatures.safeParse(properties.value).data ?? {};

    return (
        <TagsParts
            label={properties["aria-label"]}
            disabled={properties.disabled === true}
            value={features()}
            schema={FontFeatures}
            start={1}
            step={1}
            onValueChange={(next) => properties.onValueChange(next)}
        />
    );
}

/** Render the input of variable font axes: each axis's tag and value. */
export function FontVariationsInput(properties: NamedValueFieldProperties): JSX.Element {
    // read the axes, none where none are given
    const variations = (): FontVariations => FontVariations.safeParse(properties.value).data ?? {};

    return (
        <TagsParts
            label={properties["aria-label"]}
            disabled={properties.disabled === true}
            value={variations()}
            schema={FontVariations}
            start={0}
            step={0.1}
            onValueChange={(next) => properties.onValueChange(next)}
        />
    );
}
