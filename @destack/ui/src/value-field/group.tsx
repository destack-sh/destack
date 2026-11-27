import dotsSixIcon from "@destack/icon/phosphor/dots-six-vertical";
import eyeIcon from "@destack/icon/phosphor/eye";
import plusIcon from "@destack/icon/phosphor/plus";
import eyeSlashIcon from "@destack/icon/phosphor/eye-slash";
import minusIcon from "@destack/icon/phosphor/minus";
import { type Message, t } from "@destack/locale";
import type { IconName } from "@destack/icon/name";
import type { ActionValue, FieldChoice } from "@destack/object";
import { type JsonObject, JsonValue, present, type schema, toJsonSchema } from "@destack/schema";
import * as style from "@destack/style";
import { space } from "@destack/theme/tokens.stylex";
import {
    type Accessor,
    createEffect,
    createSignal,
    For,
    type JSX,
    Show,
    useContext,
    useLocale,
} from "@destack/view";
import { Button } from "../button/index.ts";
import { FieldActionsContext, FieldContext, useFieldControl } from "../field/control.ts";
import {
    FieldAction,
    FieldLegend,
    type FieldOrientation,
    FieldSet,
    FieldSetContext,
} from "../field/field.tsx";
import { Icon } from "../icon/index.ts";
import {
    Sortable,
    SortableContainer,
    SortableHandle,
    SortableItem,
    SortableMove,
} from "../sortable/index.ts";
import { stateStyles, useValueHeading, ValueHeadingContext } from "./state.tsx";
import { ValueField } from "./value-field.tsx";

/** The layouts of a value's parts: a column, a row, a grid of two, or a stack's item. */
export const valueStyles = style.create({
    group: {
        display: "flex",
        flexDirection: "column",
        gap: space[2],
    },
    row: {
        display: "flex",
        flexWrap: "wrap",
        alignItems: "center",
        gap: space[2],
    },
    grid: {
        display: "grid",
        gridTemplateColumns: "repeat(2, minmax(0, 1fr))",
        gap: space[2],
    },
    item: {
        display: "flex",
        alignItems: "center",
        gap: space[1],
    },
    stack: {
        display: "flex",
        flexDirection: "column",
        gap: space[1],
    },
    wide: {
        gridColumn: "span 2",
        minWidth: 0,
    },
    fields: {
        display: "grid",
        flex: 1,
        gridAutoFlow: "column",
        gridAutoColumns: "minmax(0, 1fr)",
        alignItems: "center",
        gap: space[1],
        minWidth: 0,
    },
    actions: {
        display: "flex",
        alignItems: "center",
    },
    summary: {
        width: "100%",
        minWidth: 0,
        justifyContent: "flex-start",
        gap: space[2],
    },
});

/** The properties of the set of a value's parts. */
export interface ValueSetProperties {
    /** What the value is called, shown as the set's legend. */
    readonly label?: string | undefined;
    /** What the value is called where no legend shows, such as a part of another value. */
    readonly "aria-label"?: string | undefined;
    /** The id of the element describing the value, none without one. */
    readonly "aria-describedby"?: string | undefined;
    /** The actions on the whole value, at the end of its legend, such as one to add a fill. */
    readonly actions?: JSX.Element;
    /** The StyleX styles applied after the set's styles, such as a row's. */
    readonly xstyle?: style.Styles;
    /** The rows of parts, each part named on its own. */
    readonly children: JSX.Element;
}

/** Render the set of a value's parts: its legend and actions, then its rows. */
export function ValueSet(properties: ValueSetProperties): JSX.Element {
    // label the set by its legend, else by the nearest field, as a field labels a group of controls
    const field = useFieldControl();
    field?.group();
    const isNested = useContext(FieldSetContext);

    // gather the actions of the sets inside without a legend, and hand them on without one too
    const outer = useContext(FieldActionsContext);
    const [inner, setInner] = createSignal<{ readonly actions: Accessor<JSX.Element> } | undefined>(
        undefined,
    );
    const actions = (): JSX.Element => (
        <>
            {properties.actions}
            {inner()?.actions()}
        </>
    );
    const isHanded = (): boolean => properties.label === undefined && outer !== null;
    const hasActions = (): boolean => properties.actions !== undefined || inner() !== undefined;
    createEffect(
        () => isHanded() && hasActions(),
        (handed) => {
            // hand the actions on while the set has some and no legend, taking them back otherwise
            if (handed) {
                outer?.(actions);

                return () => outer?.(undefined);
            }

            return undefined;
        },
    );
    const isLegend = (): boolean =>
        !isHanded() &&
        (properties.label !== undefined ||
            properties.actions !== undefined ||
            inner() !== undefined);

    // take the state and reset of the value's field, which no set inside takes again
    const heading = useValueHeading();

    return (
        <FieldSet
            {...(properties.label === undefined ? field?.groupAttributes() : {})}
            aria-label={properties.label === undefined ? properties["aria-label"] : undefined}
            aria-describedby={
                properties["aria-describedby"] ?? field?.groupAttributes()["aria-describedby"]
            }
            xstyle={properties.xstyle}
            data-state={heading?.state()}
        >
            {/* Legend, muted while the value is unset, then its actions and reset */}
            <Show when={isLegend()}>
                <FieldLegend
                    variant={isNested ? "label" : "legend"}
                    xstyle={heading?.state() === "unset" && stateStyles.unset}
                >
                    {properties.label}
                    <FieldAction>
                        {actions()}
                        {heading?.reset}
                    </FieldAction>
                </FieldLegend>
            </Show>

            {/* Rows, outside the field so no part takes its id */}
            <ValueHeadingContext value={null}>
                <FieldActionsContext
                    value={(next) => setInner(next === undefined ? undefined : { actions: next })}
                >
                    <FieldContext value={null}>{properties.children}</FieldContext>
                </FieldActionsContext>
            </ValueHeadingContext>
        </FieldSet>
    );
}

/** Render a value that is none, such as no stroke: its legend and a button to add one. */
export function ValueNone(properties: {
    /** What the value is called, shown as the legend. */
    readonly label?: string | undefined;
    /** What the value is called where no legend shows. */
    readonly "aria-label"?: string | undefined;
    /** Whether it takes no input. */
    readonly disabled: boolean | undefined;
    /** Add a value, as the input starts a new one. */
    readonly onAdd: () => void;
}): JSX.Element {
    const locale = useLocale();
    const name = (): string => properties.label ?? properties["aria-label"] ?? "";

    return (
        <ValueSet
            label={properties.label}
            aria-label={properties["aria-label"]}
            actions={
                <Button
                    variant="ghost"
                    size="sm"
                    isIconOnly
                    aria-label={locale.render(t.context("action")`Add ${name()}`)}
                    disabled={properties.disabled}
                    onClick={() => properties.onAdd()}
                >
                    <Icon icon={plusIcon} />
                </Button>
            }
        >
            {undefined}
        </ValueSet>
    );
}

/** The properties of a value's stack: its items in an order a person drags. */
export interface ValueStackProperties<Item> {
    /** The items, first on top. */
    readonly items: readonly Item[];
    /** What an item is called by its place, such as Layer 2. */
    readonly label: (index: number) => string;
    /** Whether an item shows, none for a stack whose items always show. */
    readonly isVisible?: ((item: Item) => boolean) | undefined;
    /** Show or hide the item at a place. */
    readonly onVisibleChange?: ((index: number, isVisible: boolean) => void) | undefined;
    /** Whether it takes no input. */
    readonly disabled: boolean | undefined;
    /** Handle the items being reordered or removed. */
    readonly onValueChange: (items: readonly Item[]) => void;
    /** Render an item's fields, each row kept by its place as the item changes. */
    readonly children: (item: Accessor<Item>, index: number) => JSX.Element;
}

/** Render a value's stack: rows a person drags to reorder, shows or hides and removes. */
export function ValueStack<Item>(properties: ValueStackProperties<Item>): JSX.Element {
    const locale = useLocale();

    // key each item by its place, as items repeat, and place a moved one before its next neighbour
    const move = (change: SortableMove): void => {
        // find the moved item among the items keyed by place
        const entries = properties.items.map((item, index) => ({ key: String(index), item }));
        const moved = present(
            entries.find((entry) => entry.key === change.id),
            "the moved item",
        );

        // place it before its next neighbour
        const placed = SortableMove.place(entries, moved, change, (entry) => entry.key);
        properties.onValueChange(placed.map((entry) => entry.item));
    };

    return (
        <Sortable onMove={move}>
            <SortableContainer id="items" xstyle={valueStyles.stack}>
                <For each={properties.items} keyed={false}>
                    {(item, index) => (
                        <SortableItem
                            id={String(index)}
                            disabled={properties.disabled === true}
                            label={properties.label(index)}
                        >
                            {/* Handle, fields, then shown or hidden and removed */}
                            <div
                                role="group"
                                aria-label={properties.label(index)}
                                data-slot="value-item"
                                {...style.attrs(valueStyles.item)}
                            >
                                <SortableHandle>
                                    <Icon icon={dotsSixIcon} />
                                </SortableHandle>
                                <div {...style.attrs(valueStyles.fields)}>
                                    {properties.children(item, index)}
                                </div>
                                <span {...style.attrs(valueStyles.actions)}>
                                    <Show when={properties.isVisible}>
                                        {(isVisible) => (
                                            <Button
                                                variant="ghost"
                                                size="sm"
                                                isIconOnly
                                                aria-label={locale.render(
                                                    t.context(
                                                        "action",
                                                    )`Show ${properties.label(index)}`,
                                                )}
                                                aria-pressed={
                                                    isVisible()(item()) ? "true" : "false"
                                                }
                                                disabled={properties.disabled}
                                                onClick={() =>
                                                    properties.onVisibleChange?.(
                                                        index,
                                                        !isVisible()(item()),
                                                    )
                                                }
                                            >
                                                <Icon
                                                    icon={
                                                        isVisible()(item()) ? eyeIcon : eyeSlashIcon
                                                    }
                                                />
                                            </Button>
                                        )}
                                    </Show>
                                    <Button
                                        variant="ghost"
                                        size="sm"
                                        isIconOnly
                                        aria-label={locale.render(
                                            t.context("action")`Remove ${properties.label(index)}`,
                                        )}
                                        disabled={properties.disabled}
                                        onClick={() =>
                                            properties.onValueChange(
                                                properties.items.toSpliced(index, 1),
                                            )
                                        }
                                    >
                                        <Icon icon={minusIcon} />
                                    </Button>
                                </span>
                            </div>
                        </SortableItem>
                    )}
                </For>
            </SortableContainer>
        </Sortable>
    );
}

/** Type the values of an enum as choices, each titled and described as given, with its icon. */
export function choiceType<Key extends string>(
    options: readonly Key[],
    titles: Readonly<Record<Key, { readonly title: Message; readonly description?: Message }>>,
    icons?: Readonly<Record<Key, IconName>>,
): { readonly kind: "choice"; readonly choices: readonly FieldChoice[] } {
    return {
        kind: "choice",
        choices: options.map((value) => ({
            value,
            ...titles[value],
            ...(icons === undefined ? {} : { icon: icons[value] }),
        })),
    };
}

/** Describe a value type's schema as the JSON Schema a JSON input carries. */
export function jsonType(definition: schema.Schema): {
    readonly kind: "json";
    readonly schema: JsonObject;
} {
    return { kind: "json", schema: JsonValue.object(toJsonSchema(definition)) };
}

/** Render the field of one part of a value, giving back only what the part's schema takes. */
export function ValuePart<Value extends JsonValue>(properties: {
    /** What the part takes, such as a choice of an enum's values or a named JSON value. */
    readonly type: ActionValue;
    /** The part's schema, which reads what its field gives. */
    readonly schema: schema.Schema<Value>;
    /** What the part is called. */
    readonly label: Message;
    /** Whether the label shows above the part, as a part of its own such as padding. */
    readonly isLabelShown?: boolean;
    /** How a shown label lies: above the part, or across from it as a property sheet's row. */
    readonly orientation?: FieldOrientation;
    /** The part's value. */
    readonly value: Value;
    /** Whether it takes no input. */
    readonly disabled: boolean | undefined;
    /** Handle the part changing to a value its schema takes. */
    readonly onValueChange: (value: Value) => void;
}): JSX.Element {
    // give back what the field gives, where the part's schema reads it
    const locale = useLocale();
    const give = (next: JsonValue | undefined): void => {
        const read = properties.schema.safeParse(next);
        if (read.success) {
            properties.onValueChange(read.data);
        }
    };

    return (
        <ValueField
            type={properties.type}
            label={properties.isLabelShown === true ? locale.render(properties.label) : undefined}
            aria-label={locale.render(properties.label)}
            orientation={properties.orientation}
            required
            disabled={properties.disabled}
            value={properties.value}
            onValueChange={give}
        />
    );
}
