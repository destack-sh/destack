import { Button } from "../button/index.ts";
import gapIcon from "@destack/icon/phosphor/arrows-in-line-horizontal";
import settingsIcon from "@destack/icon/phosphor/sliders-horizontal";
import { t } from "@destack/locale";
import { BoxSides, Dimension, Layout, LAYOUT_DIRECTIONS, LayoutDirection } from "@destack/style";
import { present } from "@destack/schema";
import * as style from "@destack/style";
import { type JSX, Show, useLocale } from "@destack/view";
import { type Alignment, AlignmentField } from "../alignment-field/index.ts";
import {
    DropdownMenu,
    DropdownMenuCheckboxItem,
    DropdownMenuContent,
    DropdownMenuTrigger,
} from "../dropdown-menu/index.ts";
import { FieldGroup } from "../field/field.tsx";
import { Icon } from "../icon/index.ts";
import { choiceType, jsonType, ValueNone, ValuePart, ValueSet, valueStyles } from "./group.tsx";
import { type NamedValueInputProperties, readValue } from "./value-field.tsx";
import { DimensionPart } from "./dimension.tsx";

/** The types of a layout's parts. */
const TYPES = {
    direction: choiceType(LayoutDirection.options, LAYOUT_DIRECTIONS, {
        row: "arrow-right",
        column: "arrow-down",
    }),
    padding: jsonType(BoxSides),
};

/** The layout a person adds, children side by side from the start. */
const NEW_LAYOUT: Layout = {
    direction: "row",
    gap: Dimension.zero,
    align: "start",
    justify: "start",
    isWrapping: false,
    padding: BoxSides.initial,
};

/** Render the input of a layout: its direction, gap, alignment, wrapping and padding. */
export function LayoutInput(properties: NamedValueInputProperties): JSX.Element {
    // read the layout, children side by side where none is given
    const locale = useLocale();
    const layout = (): Layout | undefined => readValue(properties.value, Layout);
    const change = (next: Partial<Layout>): void =>
        properties.onValueChange({ ...present(layout(), "the value changed"), ...next });

    return (
        <Show
            when={layout()}
            fallback={
                <ValueNone
                    label={properties.label}
                    aria-label={properties["aria-label"]}
                    disabled={properties.disabled}
                    onAdd={() => properties.onValueChange(NEW_LAYOUT)}
                />
            }
        >
            {(current) => (
                <ValueSet
                    label={properties.label}
                    aria-label={properties["aria-label"]}
                    actions={
                        <DropdownMenu>
                            <DropdownMenuTrigger
                                render={(attributes) => (
                                    <Button
                                        {...attributes}
                                        variant="ghost"
                                        size="sm"
                                        isIconOnly
                                        aria-label={locale.render(
                                            t.context("label")`Layout settings`,
                                        )}
                                        disabled={properties.disabled}
                                    >
                                        <Icon icon={settingsIcon} />
                                    </Button>
                                )}
                            />
                            <DropdownMenuContent align="end">
                                <DropdownMenuCheckboxItem
                                    checked={current().isWrapping}
                                    onCheckedChange={(isWrapping) => change({ isWrapping })}
                                >
                                    {locale.render(t.context("label")`Wrap`)}
                                </DropdownMenuCheckboxItem>
                                <DropdownMenuCheckboxItem
                                    checked={current().justify === "space-between"}
                                    onCheckedChange={(isSpaced) =>
                                        change({ justify: isSpaced ? "space-between" : "start" })
                                    }
                                >
                                    {locale.render(t.context("label")`Space between`)}
                                </DropdownMenuCheckboxItem>
                                <DropdownMenuCheckboxItem
                                    checked={current().align === "stretch"}
                                    onCheckedChange={(isStretched) =>
                                        change({ align: isStretched ? "stretch" : "start" })
                                    }
                                >
                                    {locale.render(t.context("label")`Stretch`)}
                                </DropdownMenuCheckboxItem>
                            </DropdownMenuContent>
                        </DropdownMenu>
                    }
                >
                    {/* Direction and gap beside the alignment grid */}
                    <FieldGroup columns={2}>
                        <div {...style.attrs(valueStyles.group)}>
                            <ValuePart
                                type={TYPES.direction}
                                schema={LayoutDirection}
                                label={t.context("label")`Direction`}
                                value={current().direction}
                                disabled={properties.disabled}
                                onValueChange={(direction) => change({ direction })}
                            />
                            <DimensionPart
                                label={locale.render(t.context("label")`Gap`)}
                                start={<Icon icon={gapIcon} />}
                                disabled={properties.disabled === true}
                                value={current().gap}
                                onValueChange={(gap) => change({ gap })}
                            />
                        </div>
                        <AlignmentField
                            aria-label={locale.render(t.context("label")`Alignment`)}
                            disabled={properties.disabled}
                            value={placeOf(current())}
                            onValueChange={(place) =>
                                change(alignmentOf(current().direction, place))
                            }
                        />
                    </FieldGroup>

                    {/* Padding, a set of its own */}
                    <ValuePart
                        type={TYPES.padding}
                        schema={BoxSides}
                        label={t.context("label")`Padding`}
                        isLabelShown
                        value={current().padding}
                        disabled={properties.disabled}
                        onValueChange={(padding) => change({ padding })}
                    />
                </ValueSet>
            )}
        </Show>
    );
}

/** Read a layout's place on the alignment grid, none while it spaces or stretches its children. */
function placeOf(layout: Layout): Alignment | undefined {
    // read along the direction as across for a row, and as down for a column
    const { align, justify } = layout;
    if (align === "stretch" || justify === "space-between") {
        return undefined;
    }

    return layout.direction === "row" ? { x: justify, y: align } : { x: align, y: justify };
}

/** Read the alignment and justification a place on the grid gives a layout of a direction. */
function alignmentOf(
    direction: LayoutDirection,
    place: Alignment,
): Pick<Layout, "align" | "justify"> {
    return direction === "row"
        ? { align: place.y, justify: place.x }
        : { align: place.x, justify: place.y };
}
