import { t } from "@destack/locale";
import {
    Color,
    TEXT_ALIGNS,
    TEXT_CASES,
    TEXT_DECORATIONS,
    TEXT_WRAPS,
    TextAlign,
    TextCase,
    TextDecoration,
    TextFormat,
    TextStroke,
    TextWrap,
    TRUNCATION_KINDS,
    TruncationKind,
} from "@destack/style";
import * as style from "@destack/style";
import { type JSX, Show, useLocale } from "@destack/view";
import { NumberField } from "../number-field/index.ts";
import { choiceType, jsonType, ValueGroup, ValuePart, valueStyles } from "./group.tsx";
import type { NamedValueFieldProperties } from "./value-field.tsx";
import { DimensionPart } from "./dimension.tsx";

/** The types of a text format's and a text stroke's parts. */
const TYPES = {
    align: choiceType(TextAlign.options, TEXT_ALIGNS),
    case: choiceType(TextCase.options, TEXT_CASES),
    decoration: choiceType(TextDecoration.options, TEXT_DECORATIONS),
    wrap: choiceType(TextWrap.options, TEXT_WRAPS),
    truncation: choiceType(TruncationKind.options, TRUNCATION_KINDS),
    color: jsonType(Color.bindable),
};

/** The text format an input starts at, as the browser sets text. */
const START_FORMAT: TextFormat = {
    align: "start",
    case: "none",
    decoration: "none",
    wrap: "wrap",
    truncation: { kind: "none" },
};

/** The text stroke an input starts at, a thin black line. */
const START_STROKE: TextStroke = { color: "#000000", width: { value: 1, unit: "px" } };

/** Render the input of a text format: its alignment, case, decoration, wrapping and truncation. */
export function TextFormatInput(properties: NamedValueFieldProperties): JSX.Element {
    // read the format, the browser's where none is given
    const locale = useLocale();
    const format = (): TextFormat => TextFormat.safeParse(properties.value).data ?? START_FORMAT;
    const change = (next: Partial<TextFormat>): void =>
        properties.onValueChange({ ...format(), ...next });
    const lines = (): number | undefined => {
        const { truncation } = format();

        return truncation.kind === "lines" ? truncation.count : undefined;
    };

    return (
        <ValueGroup aria-label={properties["aria-label"]} xstyle={valueStyles.group}>
            {/* Alignment and case */}
            <div {...style.attrs(valueStyles.row)}>
                <ValuePart
                    type={TYPES.align}
                    schema={TextAlign}
                    label={t.context("label")`Align`}
                    value={format().align}
                    disabled={properties.disabled}
                    onValueChange={(align) => change({ align })}
                />
                <ValuePart
                    type={TYPES.case}
                    schema={TextCase}
                    label={t.context("label")`Case`}
                    value={format().case}
                    disabled={properties.disabled}
                    onValueChange={(next) => change({ case: next })}
                />
            </div>

            {/* Decoration and wrapping */}
            <div {...style.attrs(valueStyles.row)}>
                <ValuePart
                    type={TYPES.decoration}
                    schema={TextDecoration}
                    label={t.context("label")`Decoration`}
                    value={format().decoration}
                    disabled={properties.disabled}
                    onValueChange={(decoration) => change({ decoration })}
                />
                <ValuePart
                    type={TYPES.wrap}
                    schema={TextWrap}
                    label={t.context("label")`Wrap`}
                    value={format().wrap}
                    disabled={properties.disabled}
                    onValueChange={(wrap) => change({ wrap })}
                />
            </div>

            {/* Truncation, and its lines where it cuts */}
            <div {...style.attrs(valueStyles.row)}>
                <ValuePart
                    type={TYPES.truncation}
                    schema={TruncationKind}
                    label={t.context("label")`Truncation`}
                    value={format().truncation.kind}
                    disabled={properties.disabled}
                    onValueChange={(kind) =>
                        change({
                            truncation: kind === "lines" ? { kind, count: 1 } : { kind },
                        })
                    }
                />
                <Show when={lines()}>
                    <NumberField
                        aria-label={locale.render(t.context("label")`Lines`)}
                        min={1}
                        step={1}
                        disabled={properties.disabled}
                        value={lines()}
                        onValueChange={(count) =>
                            count !== undefined && change({ truncation: { kind: "lines", count } })
                        }
                    />
                </Show>
            </div>
        </ValueGroup>
    );
}

/** Render the input of a text stroke: its color and width. */
export function TextStrokeInput(properties: NamedValueFieldProperties): JSX.Element {
    // read the stroke, a thin black line where none is given
    const locale = useLocale();
    const stroke = (): TextStroke => TextStroke.safeParse(properties.value).data ?? START_STROKE;
    const change = (next: Partial<TextStroke>): void =>
        properties.onValueChange({ ...stroke(), ...next });

    return (
        <ValueGroup aria-label={properties["aria-label"]} xstyle={valueStyles.row}>
            <ValuePart
                type={TYPES.color}
                schema={Color.bindable}
                label={t.context("label")`Color`}
                value={stroke().color}
                disabled={properties.disabled}
                onValueChange={(color) => change({ color })}
            />
            <DimensionPart
                label={locale.render(t.context("label")`Width`)}
                units={["px", "em"]}
                disabled={properties.disabled === true}
                value={stroke().width}
                onValueChange={(width) => change({ width })}
            />
        </ValueGroup>
    );
}
