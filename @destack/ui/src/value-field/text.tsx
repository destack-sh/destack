import { Button } from "../button/index.ts";
import linesIcon from "@destack/icon/phosphor/list";
import settingsIcon from "@destack/icon/phosphor/sliders-horizontal";
import { t } from "@destack/locale";
import {
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
import { present } from "@destack/schema";
import * as style from "@destack/style";
import { type JSX, Show, useLocale } from "@destack/view";
import { FieldGroup } from "../field/field.tsx";
import { Icon } from "../icon/index.ts";
import { Inspector } from "../inspector/inspector.tsx";
import { Popover, PopoverContent, PopoverTrigger } from "../popover/index.ts";
import { NumberPart } from "./dimension.tsx";
import { choiceType, ValueNone, ValuePart, ValueSet, valueStyles } from "./group.tsx";
import { type NamedValueInputProperties, readValue } from "./value-field.tsx";
import { DimensionPart } from "./dimension.tsx";
import { ColorPart } from "./color.tsx";

/** The types of a text format's and a text stroke's parts. */
const TYPES = {
    align: choiceType(TextAlign.options, TEXT_ALIGNS, {
        start: "text-align-left",
        center: "text-align-center",
        end: "text-align-right",
        justify: "text-align-justify",
    }),
    case: choiceType(TextCase.options, TEXT_CASES),
    decoration: choiceType(TextDecoration.options, TEXT_DECORATIONS),
    wrap: choiceType(TextWrap.options, TEXT_WRAPS),
    truncation: choiceType(TruncationKind.options, TRUNCATION_KINDS),
};

/** The styles of a text format's popover. */
const styles = style.create({
    popover: {
        width: "18rem",
    },
});

/** The text stroke a person adds, a thin black line. */
const NEW_STROKE: TextStroke = { color: "#000000", width: { value: 1, unit: "px" } };

/** Render the input of a text format: its alignment, case, decoration, wrapping and truncation. */
export function TextFormatInput(properties: NamedValueInputProperties): JSX.Element {
    // read the format, the browser's where none is given
    const locale = useLocale();
    const format = (): TextFormat => readValue(properties.value, TextFormat) ?? TextFormat.initial;
    const change = (next: Partial<TextFormat>): void =>
        properties.onValueChange({ ...format(), ...next });
    const lines = (): number | undefined => {
        const { truncation } = format();

        return truncation.kind === "lines" ? truncation.count : undefined;
    };

    return (
        <ValueSet
            label={properties.label}
            aria-label={properties["aria-label"]}
            actions={
                <Popover>
                    <PopoverTrigger
                        render={(attributes) => (
                            <Button
                                {...attributes}
                                variant="ghost"
                                size="sm"
                                isIconOnly
                                aria-label={locale.render(t.context("label")`Text settings`)}
                                disabled={properties.disabled}
                            >
                                <Icon icon={settingsIcon} />
                            </Button>
                        )}
                    />
                    <PopoverContent
                        align="end"
                        aria-label={locale.render(t.context("label")`Text settings`)}
                        xstyle={styles.popover}
                    >
                        <Inspector>
                            <ValuePart
                                isLabelShown
                                type={TYPES.wrap}
                                schema={TextWrap}
                                label={t.context("label")`Wrap`}
                                value={format().wrap}
                                disabled={properties.disabled}
                                onValueChange={(wrap) => change({ wrap })}
                            />
                            <ValuePart
                                isLabelShown
                                type={TYPES.truncation}
                                schema={TruncationKind}
                                label={t.context("label")`Truncation`}
                                value={format().truncation.kind}
                                disabled={properties.disabled}
                                onValueChange={(kind) =>
                                    change({
                                        truncation:
                                            kind === "lines" ? { kind, count: 1 } : { kind },
                                    })
                                }
                            />
                            <Show when={lines()}>
                                {(count) => (
                                    <FieldGroup columns={2}>
                                        <NumberPart
                                            label={locale.render(t.context("label")`Lines`)}
                                            start={<Icon icon={linesIcon} />}
                                            min={1}
                                            step={1}
                                            disabled={properties.disabled}
                                            value={count()}
                                            onValueChange={(next) =>
                                                change({
                                                    truncation: { kind: "lines", count: next },
                                                })
                                            }
                                        />
                                    </FieldGroup>
                                )}
                            </Show>
                        </Inspector>
                    </PopoverContent>
                </Popover>
            }
        >
            {/* Alignment */}
            <FieldGroup columns={1}>
                <ValuePart
                    type={TYPES.align}
                    schema={TextAlign}
                    label={t.context("label")`Align`}
                    value={format().align}
                    disabled={properties.disabled}
                    onValueChange={(align) => change({ align })}
                />
            </FieldGroup>

            {/* Case and decoration */}
            <FieldGroup columns={2}>
                <ValuePart
                    type={TYPES.case}
                    schema={TextCase}
                    label={t.context("label")`Case`}
                    value={format().case}
                    disabled={properties.disabled}
                    onValueChange={(next) => change({ case: next })}
                />
                <ValuePart
                    type={TYPES.decoration}
                    schema={TextDecoration}
                    label={t.context("label")`Decoration`}
                    value={format().decoration}
                    disabled={properties.disabled}
                    onValueChange={(decoration) => change({ decoration })}
                />
            </FieldGroup>
        </ValueSet>
    );
}

/** Render the input of a text stroke: its color and width. */
export function TextStrokeInput(properties: NamedValueInputProperties): JSX.Element {
    // read the stroke, a thin black line where none is given
    const locale = useLocale();
    const stroke = (): TextStroke | undefined => readValue(properties.value, TextStroke);
    const change = (next: Partial<TextStroke>): void =>
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
                <ValueSet aria-label={properties["aria-label"]} xstyle={valueStyles.row}>
                    <ColorPart
                        label={locale.render(t.context("label")`Color`)}
                        disabled={properties.disabled}
                        value={current().color}
                        onValueChange={(color) => change({ color })}
                    />
                    <DimensionPart
                        label={locale.render(t.context("label")`Width`)}
                        units={["px", "em"]}
                        disabled={properties.disabled === true}
                        value={current().width}
                        onValueChange={(width) => change({ width })}
                    />
                </ValueSet>
            )}
        </Show>
    );
}
