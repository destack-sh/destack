import { Popover, PopoverContent, PopoverTrigger } from "../popover/index.ts";
import { Icon } from "../icon/index.ts";
import { Button } from "../button/index.ts";
import plusIcon from "@destack/icon/phosphor/plus";
import { t } from "@destack/locale";
import {
    MOTION_PROPERTIES,
    type MotionFrame,
    MotionProperty,
    MotionValue,
    Transition,
} from "@destack/style";
import * as style from "@destack/style";
import { For, type JSX, Show, useLocale } from "@destack/view";
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuTrigger,
} from "../dropdown-menu/index.ts";
import { jsonType, ValuePart, ValueSet, valueStyles } from "../value-field/group.tsx";
import { ValueField } from "../value-field/value-field.tsx";

/** The styles of a frame's popover. */
const styles = style.create({
    picker: {
        width: "18rem",
    },
});

/** The type of a frame's transition. */
const TRANSITION_TYPE = jsonType(Transition);

/** Render a frame, such as a state's or the entry's: its values and how it moves to them. */
export function FrameField(properties: {
    /** What the frame is called. */
    readonly label: string;
    /** The frame. */
    readonly value: MotionFrame;
    /** Whether it takes no input. */
    readonly disabled: boolean | undefined;
    /** Handle the frame changing. */
    readonly onValueChange: (value: MotionFrame) => void;
}): JSX.Element {
    // offer the properties the frame leaves out, each added at CSS's initial value
    const locale = useLocale();
    const values = (): readonly MotionValue[] => properties.value.values;
    const change = (next: readonly MotionValue[]): void =>
        properties.onValueChange({ ...properties.value, values: [...next] });
    const free = (): MotionProperty[] =>
        MotionProperty.options.filter(
            (property) => !values().some((each) => each.property === property),
        );
    const add = (property: MotionProperty): void =>
        change([
            ...values(),
            MotionValue.parse({ property, value: MOTION_PROPERTIES[property].initial }),
        ]);

    return (
        <ValueSet
            label={properties.label}
            actions={
                <PropertyMenu
                    label={locale.render(t.context("action")`Add value`)}
                    properties={free()}
                    disabled={properties.disabled}
                    onSelect={add}
                />
            }
        >
            {/* One field per value */}
            <For each={values()}>
                {(each, index) => (
                    <ValueField
                        label={locale.render(MOTION_PROPERTIES[each.property].title)}
                        type={jsonType(MOTION_PROPERTIES[each.property].schema)}
                        orientation="horizontal"
                        value={each.value}
                        inherited={undefined}
                        disabled={properties.disabled}
                        onValueChange={(next) =>
                            change(
                                next === undefined
                                    ? values().toSpliced(index(), 1)
                                    : values().with(
                                          index(),
                                          MotionValue.parse({
                                              property: each.property,
                                              value: next,
                                          }),
                                      ),
                            )
                        }
                    />
                )}
            </For>

            {/* How it moves to them */}
            <ValuePart
                type={TRANSITION_TYPE}
                schema={Transition}
                label={t.context("label")`Transition`}
                isLabelShown
                value={properties.value.transition}
                disabled={properties.disabled}
                onValueChange={(transition) =>
                    properties.onValueChange({ ...properties.value, transition })
                }
            />
        </ValueSet>
    );
}

/** Render a menu of the properties motion animates, such as those a frame leaves out. */
export function PropertyMenu(properties: {
    /** What picking a property does, such as adding a value. */
    readonly label: string;
    /** The properties it offers, none hiding the menu. */
    readonly properties: readonly MotionProperty[];
    /** Whether it takes no input. */
    readonly disabled: boolean | undefined;
    /** Handle a person picking a property. */
    readonly onSelect: (property: MotionProperty) => void;
}): JSX.Element {
    const locale = useLocale();

    return (
        <Show when={properties.properties.length > 0}>
            <DropdownMenu>
                <DropdownMenuTrigger
                    render={(attributes) => (
                        <Button
                            {...attributes}
                            variant="ghost"
                            size="sm"
                            isIconOnly
                            aria-label={properties.label}
                            disabled={properties.disabled}
                        >
                            <Icon icon={plusIcon} />
                        </Button>
                    )}
                />
                <DropdownMenuContent align="end">
                    <For each={properties.properties}>
                        {(property) => (
                            <DropdownMenuItem onSelect={() => properties.onSelect(property)}>
                                {locale.render(MOTION_PROPERTIES[property].title)}
                            </DropdownMenuItem>
                        )}
                    </For>
                </DropdownMenuContent>
            </DropdownMenu>
        </Show>
    );
}

/** Render a frame as a summary of the properties it moves, which opens the frame in a popover. */
export function FramePopover(properties: {
    /** What the frame is called. */
    readonly label: string;
    /** The frame. */
    readonly value: MotionFrame;
    /** How the summary looks: outlined as a field, or quiet as a stack's row. */
    readonly variant: "outline" | "ghost";
    /** Whether it takes no input. */
    readonly disabled: boolean | undefined;
    /** Handle the frame changing. */
    readonly onValueChange: (value: MotionFrame) => void;
}): JSX.Element {
    // name the properties the frame moves, or say it moves none
    const locale = useLocale();
    const summary = (): string =>
        properties.value.values
            .map((value) => locale.render(MOTION_PROPERTIES[value.property].title))
            .join(", ") || locale.render(t.context("label")`No values`);

    return (
        <Popover>
            <PopoverTrigger
                render={(attributes) => (
                    <Button
                        {...attributes}
                        variant={properties.variant}
                        aria-label={properties.label}
                        disabled={properties.disabled}
                        xstyle={valueStyles.summary}
                    >
                        {summary()}
                    </Button>
                )}
            />
            <PopoverContent
                title={properties.label}
                side="left"
                align="start"
                xstyle={styles.picker}
            >
                <FrameField
                    label={properties.label}
                    value={properties.value}
                    disabled={properties.disabled}
                    onValueChange={properties.onValueChange}
                />
            </PopoverContent>
        </Popover>
    );
}
