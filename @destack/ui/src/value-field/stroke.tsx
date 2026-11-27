import weightIcon from "@destack/icon/phosphor/line-segments";
import { t } from "@destack/locale";
import { present } from "@destack/schema";
import { LINE_STYLES, LineStyle, Stroke, STROKE_ALIGNS, StrokeAlign } from "@destack/style";
import { type JSX, Show, useLocale } from "@destack/view";
import { choiceType, ValueNone, ValuePart, ValueSet } from "./group.tsx";
import { type NamedValueInputProperties, readValue } from "./value-field.tsx";
import { DimensionPart } from "./dimension.tsx";
import { ColorPart } from "./color.tsx";
import { FieldGroup } from "../field/field.tsx";
import { Icon } from "../icon/index.ts";

/** The types of a stroke's parts. */
const TYPES = {
    style: choiceType(LineStyle.options, LINE_STYLES),
    align: choiceType(StrokeAlign.options, STROKE_ALIGNS),
};

/** The stroke a person adds, a thin black border, as design tools start one. */
const NEW_STROKE: Stroke = {
    color: "#000000",
    width: { value: 1, unit: "px" },
    style: "solid",
    align: "inside",
};

/** Render the input of a stroke: its color, width, how it is drawn and where it runs. */
export function StrokeInput(properties: NamedValueInputProperties): JSX.Element {
    // read the stroke, a thin black border where none is given
    const locale = useLocale();
    const stroke = (): Stroke | undefined => readValue(properties.value, Stroke);
    const change = (next: Partial<Stroke>): void =>
        properties.onValueChange({ ...present(stroke(), "the value changed"), ...next });

    return (
        <Show
            when={stroke()}
            fallback={
                <ValueNone
                    label={properties.label}
                    aria-label={properties["aria-label"]}
                    disabled={properties.disabled}
                    onAdd={() => properties.onValueChange(NEW_STROKE)}
                />
            }
        >
            {(current) => (
                <ValueSet label={properties.label} aria-label={properties["aria-label"]}>
                    {/* Color */}
                    <FieldGroup columns={1}>
                        <ColorPart
                            label={locale.render(t.context("label")`Color`)}
                            disabled={properties.disabled}
                            value={current().color}
                            onValueChange={(color) => change({ color })}
                        />
                    </FieldGroup>

                    {/* Where it runs, and its weight */}
                    <FieldGroup columns={2}>
                        <ValuePart
                            type={TYPES.align}
                            schema={StrokeAlign}
                            label={t.context("label")`Position`}
                            value={current().align}
                            disabled={properties.disabled}
                            onValueChange={(align) => change({ align })}
                        />
                        <DimensionPart
                            label={locale.render(t.context("label")`Weight`)}
                            start={<Icon icon={weightIcon} />}
                            disabled={properties.disabled === true}
                            value={current().width}
                            onValueChange={(width) => change({ width })}
                        />
                    </FieldGroup>

                    {/* How it is drawn */}
                    <FieldGroup columns={1}>
                        <ValuePart
                            type={TYPES.style}
                            schema={LineStyle}
                            label={t.context("label")`Style`}
                            value={current().style}
                            disabled={properties.disabled}
                            onValueChange={(line) => change({ style: line })}
                        />
                    </FieldGroup>
                </ValueSet>
            )}
        </Show>
    );
}
