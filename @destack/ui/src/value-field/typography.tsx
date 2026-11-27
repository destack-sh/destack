import { t } from "@destack/locale";
import { Dimension, FontFamily, FontWeight, Typography } from "@destack/style";
import * as style from "@destack/style";
import { type JSX, useLocale } from "@destack/view";
import { DimensionField } from "../dimension-field/index.ts";
import { NumberField } from "../number-field/index.ts";
import { jsonType, ValueGroup, ValuePart, valueStyles } from "./group.tsx";
import type { NamedValueFieldProperties } from "./value-field.tsx";

/** The types of a typography's parts. */
const TYPES = {
    fontFamily: jsonType(FontFamily),
    fontWeight: jsonType(FontWeight),
};

/** The typography an input starts at, the browser's body text. */
const START_TYPOGRAPHY: Typography = {
    fontFamily: ["sans-serif"],
    fontSize: { value: 1, unit: "rem" },
    fontWeight: 400,
    letterSpacing: Dimension.zero,
    lineHeight: 1.5,
};

/** Render the input of typography: its fonts, size, weight, letter spacing and line height. */
export function TypographyInput(properties: NamedValueFieldProperties): JSX.Element {
    // read the typography, the browser's body text where none is given
    const locale = useLocale();
    const typography = (): Typography =>
        Typography.safeParse(properties.value).data ?? START_TYPOGRAPHY;
    const change = (next: Partial<Typography>): void =>
        properties.onValueChange({ ...typography(), ...next });

    return (
        <ValueGroup aria-label={properties["aria-label"]} xstyle={valueStyles.group}>
            {/* Fonts */}
            <ValuePart
                type={TYPES.fontFamily}
                schema={FontFamily}
                label={t.context("label")`Font family`}
                value={typography().fontFamily}
                disabled={properties.disabled}
                onValueChange={(fontFamily) => change({ fontFamily })}
            />

            {/* Size, weight, letter spacing and line height */}
            <div {...style.attrs(valueStyles.grid)}>
                <DimensionField
                    aria-label={locale.render(t.context("label")`Size`)}
                    units={["px", "rem", "em"]}
                    disabled={properties.disabled === true}
                    value={typography().fontSize}
                    onValueChange={(fontSize) => change({ fontSize })}
                />
                <ValuePart
                    type={TYPES.fontWeight}
                    schema={FontWeight}
                    label={t.context("label")`Weight`}
                    value={typography().fontWeight}
                    disabled={properties.disabled}
                    onValueChange={(fontWeight) => change({ fontWeight })}
                />
                <DimensionField
                    aria-label={locale.render(t.context("label")`Letter spacing`)}
                    units={["px", "em"]}
                    disabled={properties.disabled === true}
                    value={typography().letterSpacing}
                    onValueChange={(letterSpacing) => change({ letterSpacing })}
                />
                <NumberField
                    aria-label={locale.render(t.context("label")`Line height`)}
                    min={0.5}
                    max={4}
                    step={0.05}
                    disabled={properties.disabled}
                    value={typography().lineHeight}
                    onValueChange={(lineHeight) =>
                        lineHeight !== undefined && change({ lineHeight })
                    }
                />
            </div>
        </ValueGroup>
    );
}
