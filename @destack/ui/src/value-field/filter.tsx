import minusIcon from "@destack/icon/phosphor/minus";
import plusIcon from "@destack/icon/phosphor/plus";
import { t } from "@destack/locale";
import { Filter, FILTER_KINDS, FilterKind, FilterStack } from "@destack/style";
import * as style from "@destack/style";
import { For, type JSX, Show, useLocale } from "@destack/view";
import { Button } from "../button/index.ts";
import { Icon } from "../icon/index.ts";
import { NumberField } from "../number-field/index.ts";
import { choiceType, jsonType, ValueGroup, ValuePart, valueStyles } from "./group.tsx";
import type { NamedValueFieldProperties } from "./value-field.tsx";
import { DimensionPart } from "./dimension.tsx";

/** The types of a filter's kind, and of each filter of a stack. */
const TYPES = {
    kind: choiceType(FilterKind.options, FILTER_KINDS),
    filter: jsonType(Filter),
};

/** Render the input of a filter: its kind, and its radius, angle or amount. */
export function FilterInput(properties: NamedValueFieldProperties): JSX.Element {
    // read the filter, a slight blur where none is given
    const locale = useLocale();
    const filter = (): Filter => Filter.safeParse(properties.value).data ?? Filter.of("blur");
    const give = (next: Filter): void => properties.onValueChange(next);

    // read the kind's number, none for the other kinds
    const radius = () => {
        const current = filter();

        return current.kind === "blur" ? current.radius : undefined;
    };
    const angle = () => {
        const current = filter();

        return current.kind === "hue-rotate" ? current.angle : undefined;
    };
    const amount = () => {
        const current = filter();

        return "amount" in current ? current : undefined;
    };

    return (
        <ValueGroup aria-label={properties["aria-label"]} xstyle={valueStyles.row}>
            {/* Kind */}
            <ValuePart
                type={TYPES.kind}
                schema={FilterKind}
                label={t.context("label")`Kind`}
                value={filter().kind}
                disabled={properties.disabled}
                onValueChange={(kind) => give(Filter.of(kind))}
            />

            {/* Radius, angle or amount, each kept while its kind stays */}
            <Show when={radius()}>
                {(blur) => (
                    <DimensionPart
                        label={locale.render(t.context("label")`Radius`)}
                        units={["px", "rem"]}
                        disabled={properties.disabled === true}
                        value={blur()}
                        onValueChange={(next) => give({ kind: "blur", radius: next })}
                    />
                )}
            </Show>
            <Show when={filter().kind === "hue-rotate"}>
                <NumberField
                    aria-label={locale.render(t.context("label")`Angle`)}
                    format={{ style: "unit", unit: "degree" }}
                    disabled={properties.disabled}
                    value={angle()}
                    onValueChange={(next) =>
                        next !== undefined && give({ kind: "hue-rotate", angle: next })
                    }
                />
            </Show>
            <Show when={amount()}>
                <NumberField
                    aria-label={locale.render(t.context("label")`Amount`)}
                    min={0}
                    step={0.05}
                    format={{ style: "percent" }}
                    disabled={properties.disabled}
                    value={amount()?.amount}
                    onValueChange={(next) => {
                        const scaled = amount();
                        if (next !== undefined && scaled !== undefined) {
                            give({ kind: scaled.kind, amount: next });
                        }
                    }}
                />
            </Show>
        </ValueGroup>
    );
}

/** Render the input of filters: an input for each, applied in order. */
export function FilterStackInput(properties: NamedValueFieldProperties): JSX.Element {
    // read the filters, none where none are given
    const locale = useLocale();
    const filters = (): readonly Filter[] => FilterStack.safeParse(properties.value).data ?? [];
    const write = (next: readonly Filter[]): void => properties.onValueChange([...next]);

    return (
        <ValueGroup aria-label={properties["aria-label"]} xstyle={valueStyles.group}>
            {/* Filters in order, each row kept by its place as it changes */}
            <For each={filters()} keyed={false}>
                {(filter, index) => (
                    <div {...style.attrs(valueStyles.item)}>
                        <ValuePart
                            type={TYPES.filter}
                            schema={Filter}
                            label={t.context("label")`Filter ${index + 1}`}
                            value={filter()}
                            disabled={properties.disabled}
                            onValueChange={(changed) => write(filters().with(index, changed))}
                        />
                        <Button
                            variant="ghost"
                            isIconOnly
                            disabled={properties.disabled}
                            aria-label={locale.render(t.context("action")`Remove filter`)}
                            onClick={() => write(filters().toSpliced(index, 1))}
                        >
                            <Icon icon={minusIcon} />
                        </Button>
                    </div>
                )}
            </For>

            {/* Add one last */}
            <Button
                variant="ghost"
                disabled={properties.disabled}
                onClick={() => write([...filters(), Filter.of("blur")])}
            >
                <Icon icon={plusIcon} />
                {locale.render(t.context("action")`Add filter`)}
            </Button>
        </ValueGroup>
    );
}
