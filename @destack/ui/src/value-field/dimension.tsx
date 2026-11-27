import { type Message, t } from "@destack/locale";
import type { ActionValue } from "@destack/object";
import { schema } from "@destack/schema";
import { Dimension } from "@destack/style";
import { type JSX, Show, useLocale } from "@destack/view";
import { DimensionField } from "../dimension-field/index.ts";
import { ValueGroup, ValuePart, valueStyles } from "./group.tsx";
import type { NamedValueFieldProperties } from "./value-field.tsx";

/** Render the input of a length: its dimension field. */
export function DimensionInput(properties: NamedValueFieldProperties): JSX.Element {
    return (
        <DimensionField
            aria-label={properties["aria-label"]}
            aria-describedby={properties["aria-describedby"]}
            disabled={properties.disabled}
            value={Dimension.safeParse(properties.value).data}
            onValueChange={(next) => properties.onValueChange(next)}
        />
    );
}

/** A keyword, or a length under one kind, such as a sizing's fit or fixed length. */
type Keyword = { readonly kind: string; readonly length?: Dimension };

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
        <ValueGroup aria-label={locale.render(properties.label)} xstyle={valueStyles.row}>
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
                    <DimensionField
                        aria-label={locale.render(t.context("label")`Length`)}
                        disabled={properties.disabled === true}
                        value={length()}
                        onValueChange={(next) =>
                            give({ kind: properties.lengthKind, length: next })
                        }
                    />
                )}
            </Show>
        </ValueGroup>
    );
}
