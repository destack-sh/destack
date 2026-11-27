import { type Message, t } from "@destack/locale";
import { present } from "@destack/schema";
import { type Bindable, BoxSides, Corners, Dimension } from "@destack/style";
import * as style from "@destack/style";
import { createSignal, For, type JSX, untrack, useLocale } from "@destack/view";
import { Toggle } from "../toggle/index.ts";
import { ValueGroup, valueStyles } from "./group.tsx";
import type { NamedValueFieldProperties } from "./value-field.tsx";
import { DimensionPart } from "./dimension.tsx";

/** Render the input of a box's corners: one radius for all, or one for each corner. */
export function CornersInput(properties: NamedValueFieldProperties): JSX.Element {
    // read the corners, square ones where none are given
    const corners = (): Corners =>
        Corners.safeParse(properties.value).data ?? Corners.all(Dimension.zero);
    const labels = {
        topLeft: t.context("label")`Top left`,
        topRight: t.context("label")`Top right`,
        bottomRight: t.context("label")`Bottom right`,
        bottomLeft: t.context("label")`Bottom left`,
    } satisfies Record<keyof Corners, Message>;

    return (
        <SidesParts
            label={properties["aria-label"]}
            keys={["topLeft", "topRight", "bottomRight", "bottomLeft"]}
            labels={labels}
            each={t.context("action")`Each corner`}
            value={corners()}
            disabled={properties.disabled === true}
            all={(length) => Corners.all(length)}
            onValueChange={(next) => properties.onValueChange(next)}
        />
    );
}

/** Render the input of a box's sides: one length for all, or one for each side. */
export function BoxSidesInput(properties: NamedValueFieldProperties): JSX.Element {
    // read the sides, none where none are given
    const sides = (): BoxSides =>
        BoxSides.safeParse(properties.value).data ?? BoxSides.all(Dimension.zero);
    const labels = {
        top: t.context("label")`Top`,
        right: t.context("label")`Right`,
        bottom: t.context("label")`Bottom`,
        left: t.context("label")`Left`,
    } satisfies Record<keyof BoxSides, Message>;

    return (
        <SidesParts
            label={properties["aria-label"]}
            keys={["top", "right", "bottom", "left"]}
            labels={labels}
            each={t.context("action")`Each side`}
            value={sides()}
            disabled={properties.disabled === true}
            all={(length) => BoxSides.all(length)}
            onValueChange={(next) => properties.onValueChange(next)}
        />
    );
}

/** Render four lengths of a box: one for all while they match, else one for each. */
function SidesParts<Key extends string>(properties: {
    /** What the four are called. */
    readonly label: string | undefined;
    /** The four, in the order CSS writes them. */
    readonly keys: readonly Key[];
    /** What each of the four is called. */
    readonly labels: Readonly<Record<Key, Message>>;
    /** What the toggle to set each one apart is called. */
    readonly each: Message;
    /** The four lengths. */
    readonly value: Readonly<Record<Key, Bindable<Dimension>>>;
    /** Whether it takes no input. */
    readonly disabled: boolean;
    /** Give all four one length. */
    readonly all: (length: Bindable<Dimension>) => Readonly<Record<Key, Bindable<Dimension>>>;
    /** Handle the lengths changing. */
    readonly onValueChange: (value: Readonly<Record<Key, Bindable<Dimension>>>) => void;
}): JSX.Element {
    // show one length while all match, and each apart once they differ or a person asks
    const locale = useLocale();
    const lengths = () => properties.keys.map((key) => properties.value[key]);
    const distinct = untrack(() => new Set(lengths().map((length) => Dimension.css(length))));
    const [isEach, setEach] = createSignal(distinct.size > 1);

    return (
        <ValueGroup aria-label={properties.label} xstyle={valueStyles.group}>
            {/* One length, or each */}
            <div {...style.attrs(valueStyles.row)}>
                <DimensionPart
                    label={properties.label}
                    disabled={properties.disabled || isEach()}
                    value={present(lengths()[0], "the first length")}
                    onValueChange={(length) => properties.onValueChange(properties.all(length))}
                />
                <Toggle
                    pressed={isEach()}
                    disabled={properties.disabled}
                    aria-label={locale.render(properties.each)}
                    onPressedChange={setEach}
                >
                    {locale.render(properties.each)}
                </Toggle>
            </div>

            {/* Each one's own */}
            <div hidden={!isEach()} {...style.attrs(valueStyles.grid)}>
                <For each={properties.keys}>
                    {(key) => (
                        <DimensionPart
                            label={locale.render(properties.labels[key])}
                            disabled={properties.disabled}
                            value={properties.value[key]}
                            onValueChange={(length) =>
                                properties.onValueChange({ ...properties.value, [key]: length })
                            }
                        />
                    )}
                </For>
            </div>
        </ValueGroup>
    );
}
