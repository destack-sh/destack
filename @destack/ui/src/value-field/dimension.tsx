import { type Message, type NumberFormat, t } from "@destack/locale";
import type { ActionValue } from "@destack/object";
import { schema } from "@destack/schema";
import { type Bindable, Dimension, type DimensionUnit } from "@destack/style";
import { type JSX, Show, useLocale } from "@destack/view";
import { DimensionField } from "../dimension-field/index.ts";
import { InputGroup } from "../input-group/index.ts";
import { NumberField, NumberFieldInput, NumberFieldPrefix } from "../number-field/index.ts";
import { TokenBinding } from "./bindable.tsx";
import { ValueSet, ValuePart, valueStyles } from "./group.tsx";
import { type NamedValueInputProperties, readValue } from "./value-field.tsx";

/** Render the input of a length: its dimension field. */
export function DimensionInput(properties: NamedValueInputProperties): JSX.Element {
    return (
        <DimensionField
            aria-label={properties["aria-label"]}
            aria-describedby={properties["aria-describedby"]}
            disabled={properties.disabled}
            value={readValue(properties.value, Dimension)}
            onValueChange={(next) => properties.onValueChange(next)}
        />
    );
}

/** Render a length of a value, or a reference to a dimension token in its place. */
export function DimensionPart(properties: {
    /** What the length is called. */
    readonly label: string | undefined;
    /** The units it offers, pixels, rem and percent by default. */
    readonly units?: readonly DimensionUnit[];
    /** The mark at the field's start, a letter or an icon that scrubs it. */
    readonly start?: JSX.Element;
    /** What follows the button to bind inside the field, such as a menu of the length's kind. */
    readonly end?: JSX.Element;
    /** Whether it takes no input. */
    readonly disabled: boolean | undefined;
    /** The length, or its token. */
    readonly value: Bindable<Dimension>;
    /** Handle the length or its token changing. */
    readonly onValueChange: (value: Bindable<Dimension>) => void;
}): JSX.Element {
    return (
        <TokenBinding
            type="dimension"
            label={properties.label}
            disabled={properties.disabled}
            value={properties.value}
            onValueChange={properties.onValueChange}
            detach={(probe, reference) => probe.dimension(reference)}
            css={(length) => Dimension.css(length)}
        >
            {(length, bind) => (
                <DimensionField
                    aria-label={properties.label}
                    units={properties.units}
                    disabled={properties.disabled === true}
                    value={length()}
                    start={properties.start}
                    end={
                        <>
                            {bind}
                            {properties.end}
                        </>
                    }
                    onValueChange={properties.onValueChange}
                />
            )}
        </TokenBinding>
    );
}

/** Render a number of a value in a compact field: a mark that scrubs it, then the number. */
export function NumberPart(properties: {
    /** What the number is called. */
    readonly label: string;
    /** The mark at the field's start, a letter or an icon. */
    readonly start: JSX.Element;
    /** The lowest number. */
    readonly min?: number;
    /** The highest number. */
    readonly max?: number;
    /** How far a step goes. */
    readonly step?: number;
    /** How the number is written. */
    readonly format?: NumberFormat;
    /** Whether it takes no input. */
    readonly disabled: boolean | undefined;
    /** The number. */
    readonly value: number;
    /** Handle the number changing. */
    readonly onValueChange: (value: number) => void;
}): JSX.Element {
    return (
        <NumberField
            aria-label={properties.label}
            min={properties.min}
            max={properties.max}
            step={properties.step}
            format={properties.format}
            disabled={properties.disabled}
            value={properties.value}
            onValueChange={(next) => next !== undefined && properties.onValueChange(next)}
        >
            <InputGroup>
                <NumberFieldPrefix label={properties.label}>{properties.start}</NumberFieldPrefix>
                <NumberFieldInput />
            </InputGroup>
        </NumberField>
    );
}

/** A keyword, or a length under one kind, such as a sizing's fit or fixed length. */
type Keyword = { readonly kind: string; readonly length?: Bindable<Dimension> };

/** Render a keyword or a length: the kind picked, and the length beside it where it has one. */
export function KeywordParts<Value extends Keyword>(properties: {
    /** What it is called. */
    readonly label: Message;
    /** The kinds it offers, each titled. */
    readonly kinds: ActionValue;
    /** The kind that has a length. */
    readonly lengthKind: string;
    /** The schema that reads what it gives. */
    readonly schema: schema.Schema<Value>;
    /** The keyword or the length. */
    readonly value: Value;
    /** Whether it takes no input. */
    readonly disabled: boolean | undefined;
    /** Handle another kind or length. */
    readonly onValueChange: (value: Value) => void;
}): JSX.Element {
    // reuse the length across a change of kind, and give back what the schema reads
    const locale = useLocale();
    const give = (next: Keyword): void => {
        const read = properties.schema.safeParse(next);
        if (read.success) {
            properties.onValueChange(read.data);
        }
    };
    const kind = (next: string): Keyword =>
        next === properties.lengthKind
            ? { kind: next, length: properties.value.length ?? Dimension.zero }
            : { kind: next };

    return (
        <ValueSet aria-label={locale.render(properties.label)} xstyle={valueStyles.row}>
            <ValuePart
                type={properties.kinds}
                schema={schema.string()}
                label={t.context("label")`Kind`}
                value={properties.value.kind}
                disabled={properties.disabled}
                onValueChange={(next) => give(kind(next))}
            />
            <Show when={properties.value.length}>
                {(length) => (
                    <DimensionPart
                        label={locale.render(t.context("label")`Length`)}
                        disabled={properties.disabled}
                        value={length()}
                        onValueChange={(next) =>
                            give({ kind: properties.lengthKind, length: next })
                        }
                    />
                )}
            </Show>
        </ValueSet>
    );
}
