import { t } from "@destack/locale";
import { Color, LINE_STYLES, LineStyle, Stroke, STROKE_ALIGNS, StrokeAlign } from "@destack/style";
import * as style from "@destack/style";
import { type JSX, useLocale } from "@destack/view";
import { DimensionField } from "../dimension-field/index.ts";
import { choiceType, jsonType, ValueGroup, ValuePart, valueStyles } from "./group.tsx";
import type { NamedValueFieldProperties } from "./value-field.tsx";

/** The types of a stroke's parts. */
const TYPES = {
    color: jsonType(Color.schema),
    style: choiceType(LineStyle.options, LINE_STYLES),
    align: choiceType(StrokeAlign.options, STROKE_ALIGNS),
};

/** The stroke an input starts at, a thin black border, as design tools start one. */
const START_STROKE: Stroke = {
    color: "#000000",
    width: { value: 1, unit: "px" },
    style: "solid",
    align: "inside",
};

/** Render the input of a stroke: its color, width, how it is drawn and where it runs. */
export function StrokeInput(properties: NamedValueFieldProperties): JSX.Element {
    // read the stroke, a thin black border where none is given
    const locale = useLocale();
    const stroke = (): Stroke => Stroke.safeParse(properties.value).data ?? START_STROKE;
    const change = (next: Partial<Stroke>): void =>
        properties.onValueChange({ ...stroke(), ...next });

    return (
        <ValueGroup aria-label={properties["aria-label"]} xstyle={valueStyles.group}>
            {/* Color and width */}
            <div {...style.attrs(valueStyles.row)}>
                <ValuePart
                    type={TYPES.color}
                    schema={Color.schema}
                    label={t.context("label")`Color`}
                    value={stroke().color}
                    disabled={properties.disabled}
                    onValueChange={(color) => change({ color })}
                />
                <DimensionField
                    aria-label={locale.render(t.context("label")`Width`)}
                    disabled={properties.disabled === true}
                    value={stroke().width}
                    onValueChange={(width) => change({ width })}
                />
            </div>

            {/* How it is drawn, and where it runs */}
            <div {...style.attrs(valueStyles.row)}>
                <ValuePart
                    type={TYPES.style}
                    schema={LineStyle}
                    label={t.context("label")`Style`}
                    value={stroke().style}
                    disabled={properties.disabled}
                    onValueChange={(line) => change({ style: line })}
                />
                <ValuePart
                    type={TYPES.align}
                    schema={StrokeAlign}
                    label={t.context("label")`Position`}
                    value={stroke().align}
                    disabled={properties.disabled}
                    onValueChange={(align) => change({ align })}
                />
            </div>
        </ValueGroup>
    );
}
