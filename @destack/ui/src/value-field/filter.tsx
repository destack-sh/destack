import { Button } from "../button/index.ts";
import plusIcon from "@destack/icon/phosphor/plus";
import angleIcon from "@destack/icon/phosphor/arrow-clockwise";
import amountIcon from "@destack/icon/phosphor/percent";
import { t } from "@destack/locale";
import { Filter, FILTER_KINDS, FilterKind, FilterStack } from "@destack/style";
import { type JSX, Show, useLocale } from "@destack/view";
import { Icon } from "../icon/index.ts";
import { choiceType, jsonType, ValueStack, ValuePart, ValueSet, valueStyles } from "./group.tsx";
import { type NamedValueInputProperties, readValue } from "./value-field.tsx";
import { DimensionPart, NumberPart } from "./dimension.tsx";

/** The types of a filter's kind, and of each filter of a stack. */
const TYPES = {
    kind: choiceType(FilterKind.options, FILTER_KINDS),
    filter: jsonType(Filter),
};

/** Render the input of a filter: its kind, and its radius, angle or amount. */
export function FilterInput(properties: NamedValueInputProperties): JSX.Element {
    // read the filter, a slight blur where none is given
    const locale = useLocale();
    const filter = (): Filter => readValue(properties.value, Filter) ?? Filter.of("blur");
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
        <ValueSet aria-label={properties["aria-label"]} xstyle={valueStyles.grid}>
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
            <Show when={angle()}>
                {(turn) => (
                    <NumberPart
                        label={locale.render(t.context("label")`Angle`)}
                        start={<Icon icon={angleIcon} />}
                        format={{ style: "unit", unit: "degree", unitDisplay: "narrow" }}
                        disabled={properties.disabled}
                        value={turn()}
                        onValueChange={(next) => give({ kind: "hue-rotate", angle: next })}
                    />
                )}
            </Show>
            <Show when={amount()}>
                {(scaled) => (
                    <NumberPart
                        label={locale.render(t.context("label")`Amount`)}
                        start={<Icon icon={amountIcon} />}
                        min={0}
                        step={0.05}
                        format={{ style: "percent" }}
                        disabled={properties.disabled}
                        value={scaled().amount}
                        onValueChange={(next) => give({ kind: scaled().kind, amount: next })}
                    />
                )}
            </Show>
        </ValueSet>
    );
}

/** Render the input of filters: an input for each, applied in order. */
export function FilterStackInput(properties: NamedValueInputProperties): JSX.Element {
    // read the filters, none where none are given
    const locale = useLocale();
    const filters = (): readonly Filter[] => readValue(properties.value, FilterStack) ?? [];
    const write = (next: readonly Filter[]): void => properties.onValueChange([...next]);

    return (
        <ValueSet
            actions={
                <Button
                    variant="ghost"
                    size="sm"
                    isIconOnly
                    aria-label={locale.render(t.context("action")`Add filter`)}
                    disabled={properties.disabled}
                    onClick={() => write([...filters(), Filter.of("blur")])}
                >
                    <Icon icon={plusIcon} />
                </Button>
            }
            label={properties.label}
            aria-label={properties["aria-label"]}
        >
            {/* Filters in order */}
            <ValueStack
                items={filters()}
                label={(index) => locale.render(t.context("label")`Filter ${index + 1}`)}
                disabled={properties.disabled}
                onValueChange={write}
            >
                {(filter, index) => (
                    <ValuePart
                        type={TYPES.filter}
                        schema={Filter}
                        label={t.context("label")`Filter ${index + 1}`}
                        value={filter()}
                        disabled={properties.disabled}
                        onValueChange={(changed) => write(filters().with(index, changed))}
                    />
                )}
            </ValueStack>
        </ValueSet>
    );
}
