import { t } from "@destack/locale";
import {
    BoxSides,
    Dimension,
    Layout,
    LAYOUT_ALIGNS,
    LAYOUT_DIRECTIONS,
    LAYOUT_JUSTIFIES,
    LayoutAlign,
    LayoutDirection,
    LayoutJustify,
} from "@destack/style";
import * as style from "@destack/style";
import { type JSX, useLocale } from "@destack/view";
import { DimensionField } from "../dimension-field/index.ts";
import { Switch } from "../switch/index.ts";
import { choiceType, jsonType, ValueGroup, ValuePart, valueStyles } from "./group.tsx";
import type { NamedValueFieldProperties } from "./value-field.tsx";

/** The types of a layout's parts. */
const TYPES = {
    direction: choiceType(LayoutDirection.options, LAYOUT_DIRECTIONS),
    align: choiceType(LayoutAlign.options, LAYOUT_ALIGNS),
    justify: choiceType(LayoutJustify.options, LAYOUT_JUSTIFIES),
    padding: jsonType(BoxSides),
};

/** The layout an input starts at, children side by side from the start. */
const START_LAYOUT: Layout = {
    direction: "row",
    gap: Dimension.zero,
    align: "start",
    justify: "start",
    isWrapping: false,
    padding: BoxSides.all(Dimension.zero),
};

/** Render the input of a layout: its direction, gap, alignment, wrapping and padding. */
export function LayoutInput(properties: NamedValueFieldProperties): JSX.Element {
    // read the layout, children side by side where none is given
    const locale = useLocale();
    const layout = (): Layout => Layout.safeParse(properties.value).data ?? START_LAYOUT;
    const change = (next: Partial<Layout>): void =>
        properties.onValueChange({ ...layout(), ...next });

    return (
        <ValueGroup aria-label={properties["aria-label"]} xstyle={valueStyles.group}>
            {/* Direction, gap and wrapping */}
            <div {...style.attrs(valueStyles.row)}>
                <ValuePart
                    type={TYPES.direction}
                    schema={LayoutDirection}
                    label={t.context("label")`Direction`}
                    value={layout().direction}
                    disabled={properties.disabled}
                    onValueChange={(direction) => change({ direction })}
                />
                <DimensionField
                    aria-label={locale.render(t.context("label")`Gap`)}
                    disabled={properties.disabled === true}
                    value={layout().gap}
                    onValueChange={(gap) => change({ gap })}
                />
                <Switch
                    aria-label={locale.render(t.context("label")`Wrap`)}
                    disabled={properties.disabled}
                    checked={layout().isWrapping}
                    onCheckedChange={(isWrapping) => change({ isWrapping })}
                />
            </div>

            {/* Alignment across and along */}
            <div {...style.attrs(valueStyles.row)}>
                <ValuePart
                    type={TYPES.align}
                    schema={LayoutAlign}
                    label={t.context("label")`Align`}
                    value={layout().align}
                    disabled={properties.disabled}
                    onValueChange={(align) => change({ align })}
                />
                <ValuePart
                    type={TYPES.justify}
                    schema={LayoutJustify}
                    label={t.context("label")`Justify`}
                    value={layout().justify}
                    disabled={properties.disabled}
                    onValueChange={(justify) => change({ justify })}
                />
            </div>

            {/* Padding */}
            <ValuePart
                type={TYPES.padding}
                schema={BoxSides}
                label={t.context("label")`Padding`}
                value={layout().padding}
                disabled={properties.disabled}
                onValueChange={(padding) => change({ padding })}
            />
        </ValueGroup>
    );
}
