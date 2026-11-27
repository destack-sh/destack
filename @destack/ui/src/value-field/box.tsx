import { Toggle } from "../toggle/index.ts";
import bottomLeftIcon from "@destack/icon/phosphor/arrow-down-left";
import bottomRightIcon from "@destack/icon/phosphor/arrow-down-right";
import bottomIcon from "@destack/icon/phosphor/arrow-line-down";
import leftIcon from "@destack/icon/phosphor/arrow-line-left";
import rightIcon from "@destack/icon/phosphor/arrow-line-right";
import topIcon from "@destack/icon/phosphor/arrow-line-up";
import topLeftIcon from "@destack/icon/phosphor/arrow-up-left";
import topRightIcon from "@destack/icon/phosphor/arrow-up-right";
import eachSideIcon from "@destack/icon/phosphor/bounding-box";
import cornersIcon from "@destack/icon/phosphor/corners-out";
import eachCornerIcon from "@destack/icon/phosphor/frame-corners";
import sidesIcon from "@destack/icon/phosphor/square";
import { type Message, t } from "@destack/locale";
import { present } from "@destack/schema";
import { type Bindable, BoxSides, Corners, Dimension } from "@destack/style";
import { createSignal, For, type JSX, Show, untrack, useLocale } from "@destack/view";
import type { IconBodies } from "@destack/icon";
import { FieldGroup } from "../field/field.tsx";
import { Icon } from "../icon/index.ts";
import { ValueSet } from "./group.tsx";
import { type NamedValueInputProperties, readValue } from "./value-field.tsx";
import { DimensionPart } from "./dimension.tsx";

/** Render the input of a box's corners: one radius for all, or one for each corner. */
export function CornersInput(properties: NamedValueInputProperties): JSX.Element {
    // read the corners, square ones where none are given
    const corners = (): Corners => readValue(properties.value, Corners) ?? Corners.initial;
    const labels = {
        topLeft: t.context("label")`Top left`,
        topRight: t.context("label")`Top right`,
        bottomRight: t.context("label")`Bottom right`,
        bottomLeft: t.context("label")`Bottom left`,
    } satisfies Record<keyof Corners, Message>;

    return (
        <SidesParts
            label={properties.label}
            aria-label={properties["aria-label"]}
            keys={["topLeft", "topRight", "bottomRight", "bottomLeft"]}
            labels={labels}
            each={t.context("action")`Each corner`}
            every={t.context("label")`All corners`}
            everyIcon={cornersIcon}
            eachIcon={eachCornerIcon}
            icons={{
                topLeft: topLeftIcon,
                topRight: topRightIcon,
                bottomRight: bottomRightIcon,
                bottomLeft: bottomLeftIcon,
            }}
            value={corners()}
            disabled={properties.disabled === true}
            all={(length) => Corners.all(length)}
            onValueChange={(next) => properties.onValueChange(next)}
        />
    );
}

/** Render the input of a box's sides: one length for all, or one for each side. */
export function BoxSidesInput(properties: NamedValueInputProperties): JSX.Element {
    // read the sides, none where none are given
    const sides = (): BoxSides => readValue(properties.value, BoxSides) ?? BoxSides.initial;
    const labels = {
        top: t.context("label")`Top`,
        right: t.context("label")`Right`,
        bottom: t.context("label")`Bottom`,
        left: t.context("label")`Left`,
    } satisfies Record<keyof BoxSides, Message>;

    return (
        <SidesParts
            label={properties.label}
            aria-label={properties["aria-label"]}
            keys={["top", "right", "bottom", "left"]}
            labels={labels}
            each={t.context("action")`Each side`}
            every={t.context("label")`All sides`}
            everyIcon={sidesIcon}
            eachIcon={eachSideIcon}
            icons={{ top: topIcon, right: rightIcon, bottom: bottomIcon, left: leftIcon }}
            value={sides()}
            disabled={properties.disabled === true}
            all={(length) => BoxSides.all(length)}
            onValueChange={(next) => properties.onValueChange(next)}
        />
    );
}

/** Render four lengths of a box: one for all while they match, else one for each. */
function SidesParts<Key extends string>(properties: {
    /** What the four are called, shown as their legend. */
    readonly label: string | undefined;
    /** What the four are called where no legend shows. */
    readonly "aria-label": string | undefined;
    /** The four, in the order CSS writes them. */
    readonly keys: readonly Key[];
    /** What each of the four is called. */
    readonly labels: Readonly<Record<Key, Message>>;
    /** What the toggle to set each one apart is called. */
    readonly each: Message;
    /** What the one length of all four is called. */
    readonly every: Message;
    /** The icon of the one length of all four. */
    readonly everyIcon: IconBodies;
    /** The icon of the toggle to set each one apart. */
    readonly eachIcon: IconBodies;
    /** The icon of each of the four. */
    readonly icons: Readonly<Record<Key, IconBodies>>;
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
        <ValueSet
            label={properties.label}
            aria-label={properties["aria-label"]}
            actions={
                <Toggle
                    size="sm"
                    isIconOnly
                    aria-label={locale.render(properties.each)}
                    disabled={properties.disabled}
                    pressed={isEach()}
                    onPressedChange={setEach}
                >
                    <Icon icon={properties.eachIcon} />
                </Toggle>
            }
        >
            {/* One length for all four, or each one's own */}
            <Show
                when={isEach()}
                fallback={
                    <FieldGroup columns={1}>
                        <DimensionPart
                            label={locale.render(properties.every)}
                            start={<Icon icon={properties.everyIcon} />}
                            disabled={properties.disabled}
                            value={present(lengths()[0], "the first length")}
                            onValueChange={(length) =>
                                properties.onValueChange(properties.all(length))
                            }
                        />
                    </FieldGroup>
                }
            >
                <FieldGroup columns={2}>
                    <For each={properties.keys}>
                        {(key) => (
                            <DimensionPart
                                label={locale.render(properties.labels[key])}
                                start={<Icon icon={properties.icons[key]} />}
                                disabled={properties.disabled}
                                value={properties.value[key]}
                                onValueChange={(length) =>
                                    properties.onValueChange({
                                        ...properties.value,
                                        [key]: length,
                                    })
                                }
                            />
                        )}
                    </For>
                </FieldGroup>
            </Show>
        </ValueSet>
    );
}
