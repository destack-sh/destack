import groupIcon from "@destack/icon/phosphor/list-plus";
import plusIcon from "@destack/icon/phosphor/plus";
import funnelIcon from "@destack/icon/phosphor/funnel-simple";
import checkIcon from "@destack/icon/phosphor/check";
import xIcon from "@destack/icon/phosphor/x";
import { type Localization, plural, t } from "@destack/locale";
import type { ActionValue, Choice } from "@destack/object";
import type { JsonValue } from "@destack/schema";
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { color, motion, radius, size, space, stroke, weight } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import {
    type Accessor,
    createContext,
    createMemo,
    createSignal,
    flush,
    For,
    handleAction,
    type JSX,
    omit,
    Show,
    untrack,
    useContext,
    useLocale,
} from "@destack/view";
import { ChoiceIcon } from "../action/choice-icon.tsx";
import { Button, type ButtonProperties } from "../button/index.ts";
import { ButtonGroup, ButtonGroupText } from "../button-group/index.ts";
import {
    Command,
    CommandEmpty,
    CommandGroup,
    CommandInput,
    CommandItem,
    CommandList,
    CommandSeparator,
} from "../command/index.ts";
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuRadioGroup,
    DropdownMenuRadioItem,
    DropdownMenuTrigger,
} from "../dropdown-menu/index.ts";
import { Icon } from "../icon/index.ts";
import {
    contentOr,
    type ElementPartProperties,
    mergeProperties,
    renderPart,
} from "../part/index.ts";
import { Popover, PopoverContent, PopoverTrigger, usePopover } from "../popover/index.ts";
import { ValueField, ValueIcon } from "../value-field/index.ts";
import { addFilter, clearFilter } from "./actions.ts";
import {
    FilterContext,
    FilterControl,
    type FilterEntry,
    type FilterFieldDefinition,
    type FilterOptions,
    type FilterRule,
    type FilterSet,
    useFilter,
} from "./control.ts";
import { FILTER_SPANS, type FilterOperatorDefinition } from "./operator.ts";

/** How many chosen values a filter lists before it counts them. */
const LISTED = 2;

/** The styles of a filter and its parts. */
const styles = style.create({
    filter: {
        display: "flex",
        flexWrap: "wrap",
        alignItems: "center",
        gap: space[2],
        minWidth: 0,
    },
    item: {
        maxWidth: "100%",
        animationName: style.keyframes({
            from: { opacity: 0, transform: "scale(0.97)" },
        }),
        animationDuration: motion.durationShort,
        animationTimingFunction: motion.easingStandard,
    },
    field: {
        gap: space[1],
        paddingInline: space[2],
        borderColor: "transparent",
        fontWeight: weight.regular,
        whiteSpace: "nowrap",
    },
    operator: {
        color: color.mutedForeground,
        fontWeight: weight.regular,
    },
    value: {
        minWidth: 0,
        fontWeight: weight.regular,
    },
    words: {
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
        maxWidth: `calc(56 * ${space[1]})`,
    },
    menu: {
        padding: 0,
    },
    editor: {
        display: "grid",
        gap: space[2],
        padding: space[2],
    },
    count: {
        marginInlineStart: "auto",
        color: color.mutedForeground,
        fontVariantNumeric: "tabular-nums",
    },
    group: {
        display: "flex",
        flexDirection: "column",
        alignItems: "flex-start",
        gap: space[2],
        minWidth: 0,
    },
    nested: {
        padding: space[2],
        borderWidth: stroke.border,
        borderColor: color.border,
        borderRadius: radius[4],
    },
    footer: {
        display: "flex",
        gap: space[1],
    },
    row: {
        display: "flex",
        flexWrap: { default: "nowrap", [media.maxSm]: "wrap" },
        alignItems: "flex-start",
        gap: space[2],
        maxWidth: "100%",
    },
    connective: {
        display: "flex",
        alignItems: "center",
        flexShrink: 0,
        minWidth: `calc(14 * ${space[1]})`,
        height: size[1],
        paddingInline: space[2],
        color: color.mutedForeground,
    },
    panel: {
        width: "max-content",
        maxWidth: `min(36rem, calc(100vw - 2 * ${space[1]}))`,
        padding: space[3],
    },
    of: {
        color: color.mutedForeground,
    },
    check: (isOn: boolean) => ({ visibility: isOn ? "visible" : "hidden" }),
});

/** The set of conditions parts add to, null for the filter's own. */
const SetContext = createContext<(() => FilterSet) | null>(null);

/** The condition parts are of, null outside one. */
const ItemContext = createContext<(() => FilterEntry) | null>(null);

/** The place of a condition among those of its set, null outside a vertical list. */
const IndexContext = createContext<(() => number) | null>(null);

/** The properties of a filter, with its element's attributes. */
export type FilterProperties = FilterOptions &
    Omit<JSX.HTMLAttributes<HTMLDivElement>, "class" | keyof FilterOptions> &
    ElementPartProperties;

/** Render a condition a person edits: its parts, or those of a usual filter. */
export function Filter(properties: FilterProperties): JSX.Element {
    // share the conditions with the parts
    const control = new FilterControl(properties);
    const locale = useLocale();
    const rest = omit(properties, "fields", "value", "defaultValue", "onValueChange", "children");
    handleAction(clearFilter, {
        isEnabled: () => control.root().items.length > 0,
        run: () => control.clear(),
    });

    return (
        <FilterContext value={control}>
            {renderPart("div", "filter", rest, styles.filter, {
                role: "group" as const,
                "aria-label": locale.render(t`Filter`),
                get children() {
                    return contentOr(properties, () => (
                        <>
                            <FilterList />
                            <FilterAdd />
                            <FilterMatch />
                            <FilterClear />
                        </>
                    ));
                },
            })}
        </FilterContext>
    );
}

/** The properties of a list of conditions. */
export interface FilterListProperties {
    /** Whether conditions sit beside each other, or below with how they combine before each. */
    readonly orientation?: "horizontal" | "vertical";
    /** Render a condition, the usual parts by default. */
    readonly children?: (item: Accessor<FilterEntry>) => JSX.Element;
}

/** Render the conditions of the nearest set. */
export function FilterList(properties: FilterListProperties): JSX.Element {
    // follow each condition by its id as it changes
    const control = useFilter();
    const set = useContext(SetContext) ?? (() => control.root());
    const ids = createMemo(() => set().items.map(({ id }) => id));
    const isVertical = (): boolean => properties.orientation === "vertical";

    return (
        <For each={ids()}>
            {(id, index) => {
                let last = untrack(() => set().items.find((each) => each.id === id));
                const item = (): FilterEntry | undefined => {
                    last = set().items.find((each) => each.id === id) ?? last;

                    return last;
                };

                return (
                    <Show when={item()}>
                        {(entry) => (
                            <ItemContext value={entry}>
                                <Show
                                    when={isVertical()}
                                    fallback={
                                        properties.children?.(entry) ??
                                        (untrack(entry).kind === "set" ? (
                                            <FilterSummary />
                                        ) : (
                                            <FilterItem />
                                        ))
                                    }
                                >
                                    <div data-slot="filter-row" {...style.attrs(styles.row)}>
                                        <IndexContext value={index}>
                                            <FilterMatch />
                                        </IndexContext>
                                        {properties.children?.(entry) ??
                                            (untrack(entry).kind === "set" ? (
                                                <FilterGroup />
                                            ) : (
                                                <FilterItem />
                                            ))}
                                    </div>
                                </Show>
                            </ItemContext>
                        )}
                    </Show>
                );
            }}
        </For>
    );
}

/** The properties of an element of a filter, with its own attributes. */
export type FilterElementProperties = Omit<JSX.HTMLAttributes<HTMLDivElement>, "class"> &
    ElementPartProperties;

/** Render one condition joined into one control: its parts, or the usual ones. */
export function FilterItem(properties: FilterElementProperties): JSX.Element {
    // call the group as the field it is on
    const control = useFilter();
    const item = useFilterItem();
    const rest = omit(properties, "children", "xstyle");
    const label = (): string | undefined => {
        const entry = item();

        return entry.kind === "rule" ? control.field(entry.field)?.label : undefined;
    };

    return (
        <ButtonGroup
            divided
            data-slot="filter-item"
            aria-label={label()}
            {...rest}
            xstyle={[styles.item, properties.xstyle]}
        >
            {contentOr(properties, () => (
                <>
                    <FilterField />
                    <FilterOperator />
                    <FilterValue />
                    <FilterRemove />
                </>
            ))}
        </ButtonGroup>
    );
}

/** Render the field of the nearest condition. */
export function FilterField(properties: FilterElementProperties): JSX.Element {
    // read the condition's field
    const control = useFilter();
    const locale = useLocale();
    const item = useFilterItem();
    const field = (): FilterFieldDefinition | undefined => {
        const entry = item();

        return entry.kind === "rule" ? control.field(entry.field) : undefined;
    };

    return (
        <ButtonGroupText
            data-slot="filter-field"
            {...properties}
            xstyle={[text.xs, styles.field, properties.xstyle]}
        >
            {contentOr(properties, () => (
                <Show when={field()} fallback={locale.render(t`Another condition`)}>
                    {(each) => (
                        <>
                            <ValueIcon kind={each().type.kind} icon={each().icon} />
                            {each().label}
                        </>
                    )}
                </Show>
            ))}
        </ButtonGroupText>
    );
}

/** Render how the nearest condition compares, which a menu changes. */
export function FilterOperator(properties: ButtonProperties): JSX.Element {
    // list how the field compares
    const control = useFilter();
    const locale = useLocale();
    const item = useFilterItem();
    const rule = (): FilterRule | undefined => ruleOf(item());
    const title = (operator: FilterOperatorDefinition, count: number): string =>
        operator.title(locale, count);
    const operators = (): readonly FilterOperatorDefinition[] =>
        control.operators(rule()?.field ?? "");

    return (
        <Show when={rule()}>
            {(each) => (
                <DropdownMenu>
                    <DropdownMenuTrigger
                        render={(attributes) => (
                            <Button
                                {...attributes}
                                variant="secondary"
                                size="sm"
                                {...properties}
                                xstyle={[styles.operator, properties.xstyle]}
                            />
                        )}
                        data-slot="filter-operator"
                    >
                        {title(
                            control.operator(each()) ?? present(operators()),
                            each().values.length,
                        )}
                    </DropdownMenuTrigger>
                    <DropdownMenuContent align="start">
                        <DropdownMenuRadioGroup
                            value={each().operator}
                            onValueChange={(name) => {
                                const chosen = operators().find((other) => other.name === name);
                                if (chosen !== undefined) {
                                    control.compare(each(), chosen);
                                }
                            }}
                        >
                            <For each={operators()}>
                                {(operator) => (
                                    <DropdownMenuRadioItem value={operator.name}>
                                        {title(operator, each().values.length)}
                                    </DropdownMenuRadioItem>
                                )}
                            </For>
                        </DropdownMenuRadioGroup>
                    </DropdownMenuContent>
                </DropdownMenu>
            )}
        </Show>
    );
}

/** The properties of a condition's value. */
export type FilterValueProperties = ButtonProperties & {
    /** Write what the condition compares with, in words by default. */
    readonly words?: (values: readonly JsonValue[]) => JSX.Element;
    /** Render what a person gives it by, by the field's kind by default. */
    readonly editor?: (rule: Accessor<FilterRule>) => JSX.Element;
};

/** Render what the nearest condition compares with, which a popover changes. */
export function FilterValue(properties: FilterValueProperties): JSX.Element {
    // write what it compares with
    const control = useFilter();
    const locale = useLocale();
    const item = useFilterItem();
    const rest = omit(properties, "words", "editor", "xstyle");
    const rule = (): FilterRule | undefined => {
        const found = ruleOf(item());

        return found !== undefined && control.operator(found)?.arity !== "none" ? found : undefined;
    };
    const name = createMemo(() => rule()?.field ?? "");
    const choices = createMemo(() => control.choices(name(), locale));

    return (
        <Show when={rule()}>
            {(each) => (
                <Popover>
                    <PopoverTrigger
                        render={(attributes) => (
                            <Button
                                {...attributes}
                                variant="secondary"
                                size="sm"
                                {...rest}
                                xstyle={[styles.value, properties.xstyle]}
                            />
                        )}
                        data-slot="filter-value"
                        data-rule={each().id}
                    >
                        <For each={listed(each(), choices())}>
                            {(choice) => <ChoiceIcon choice={choice} />}
                        </For>
                        <span {...style.attrs(styles.words)}>
                            {properties.words?.(each().values) ??
                                wordsOf(each(), choices(), control, locale)}
                        </span>
                    </PopoverTrigger>
                    <PopoverContent align="start" xstyle={styles.menu}>
                        {properties.editor?.(each) ?? <FilterEditor rule={each()} />}
                    </PopoverContent>
                </Popover>
            )}
        </Show>
    );
}

/** Render what a person gives a condition's value by: a list to choose from, or a field. */
export function FilterEditor(properties: {
    /** The condition. */
    readonly rule: FilterRule;
}): JSX.Element {
    // read what the field has, and how it compares
    const control = useFilter();
    const popover = usePopover();
    const locale = useLocale();
    const field = (): FilterFieldDefinition | undefined => control.field(properties.rule.field);
    const arity = (): FilterOperatorDefinition["arity"] | undefined =>
        control.operator(properties.rule)?.arity;
    const name = createMemo(() => properties.rule.field);
    const spans = createMemo(() =>
        FILTER_SPANS.map((span) => ({ value: span.name, title: locale.render(span.title) })),
    );
    const listing = createMemo(() => control.choices(name(), locale));
    const entered = (): ActionValue | undefined => {
        // type what is not chosen from a list
        const type = field()?.type;

        return type === undefined || type.kind === "choice" || type.kind === "object"
            ? undefined
            : type.kind === "number"
              ? type
              : { kind: type.kind };
    };
    const choices = (): readonly Choice[] => (arity() === "span" ? spans() : listing());
    const isListed = (): boolean =>
        arity() === "many" || arity() === "span" || choices().length > 0;
    const labelOf = (place: number, label: string): string =>
        arity() !== "range"
            ? label
            : place === 0
              ? locale.render(t`${label}, from`)
              : locale.render(t`${label}, to`);
    const tally = (value: JsonValue): string => {
        const count = field()?.count?.(value);

        return count === undefined ? "" : locale.number(count);
    };
    const toggle = (value: JsonValue): void => {
        // add or drop one of several, else take it alone
        const { rule } = properties;
        const isOn = rule.values.includes(value);
        control.set(
            rule,
            arity() !== "many"
                ? [value]
                : isOn
                  ? rule.values.filter((each) => each !== value)
                  : [...rule.values, value],
        );
        if (arity() !== "many") {
            popover.close();
        }
    };
    const setAt = (place: number, value: JsonValue | undefined): void => {
        // leave the other end in its place
        const ends = [0, 1].map((each) =>
            each === place ? (value ?? null) : (properties.rule.values[each] ?? null),
        );
        control.set(properties.rule, arity() === "range" ? ends : [ends[0] ?? null]);
    };

    return (
        <Show
            when={isListed()}
            fallback={
                <div
                    {...style.attrs(styles.editor)}
                    onKeyDown={(event) => {
                        // take Enter as done, not as a press of what takes the focus back
                        if (event.key === "Enter") {
                            event.preventDefault();
                            popover.close();
                        }
                    }}
                >
                    <For each={arity() === "range" ? [0, 1] : [0]}>
                        {(place) => (
                            <Show when={entered()}>
                                {(type) => (
                                    <ValueField
                                        aria-label={labelOf(place, field()?.label ?? "")}
                                        type={type()}
                                        value={properties.rule.values[place] ?? undefined}
                                        onValueChange={(value) => setAt(place, value)}
                                    />
                                )}
                            </Show>
                        )}
                    </For>
                </div>
            }
        >
            <Command aria-label={field()?.label} shouldFilter={field()?.onSearch === undefined}>
                <CommandInput
                    placeholder={field()?.label}
                    onValueChange={(typed) => field()?.onSearch?.(typed)}
                />
                <CommandList>
                    <CommandEmpty>{locale.render(t`No results.`)}</CommandEmpty>
                    <CommandGroup>
                        <For each={choices()}>
                            {(choice) => (
                                <CommandItem
                                    value={JSON.stringify(choice.value)}
                                    keywords={[choice.title]}
                                    aria-checked={
                                        properties.rule.values.includes(choice.value)
                                            ? "true"
                                            : "false"
                                    }
                                    onSelect={() => toggle(choice.value)}
                                >
                                    <Icon
                                        icon={checkIcon}
                                        xstyle={styles.check(
                                            properties.rule.values.includes(choice.value),
                                        )}
                                    />
                                    <ChoiceIcon choice={choice} />
                                    {choice.title}
                                    <span {...style.attrs(styles.count)}>
                                        {tally(choice.value)}
                                    </span>
                                </CommandItem>
                            )}
                        </For>
                    </CommandGroup>
                </CommandList>
            </Command>
        </Show>
    );
}

/** Render the button that removes the nearest condition, the focus left on the next. */
export function FilterRemove(properties: ButtonProperties): JSX.Element {
    // remove the condition or the set
    const control = useFilter();
    const locale = useLocale();
    const item = useFilterItem();
    const remove = ({ currentTarget }: { readonly currentTarget: HTMLElement }): void => {
        // find where it is, as it goes away with the condition
        const root = currentTarget.closest("[data-slot=filter-group], [data-slot=filter]");
        const buttons = (): HTMLElement[] => [
            ...(root?.querySelectorAll<HTMLElement>("[data-slot=filter-remove]") ?? []),
        ];
        const place = buttons().indexOf(currentTarget);
        control.remove(item().id);
        flush();

        // focus the next condition's button, else the one before, else the menu
        const left = buttons();
        const menu = root?.querySelector<HTMLElement>("[data-slot=filter-add]");
        (left[place] ?? left.at(-1) ?? menu)?.focus();
    };

    return (
        <Button
            data-slot="filter-remove"
            variant="secondary"
            size="icon-sm"
            aria-label={locale.render(t`Remove filter`)}
            {...mergeProperties({ onClick: remove }, properties)}
        >
            {contentOr(properties, () => (
                <Icon icon={xIcon} />
            ))}
        </Button>
    );
}

/** The properties of the button that adds conditions. */
export type FilterAddProperties = ButtonProperties & {
    /** Whether it offers sets of conditions of which any holds. */
    readonly groups?: boolean;
};

/** Render the button that adds a condition to the nearest set, its fields and values searched. */
export function FilterAdd(properties: FilterAddProperties): JSX.Element {
    // open by its key where it adds to the filter itself
    const control = useFilter();
    const locale = useLocale();
    const set = useContext(SetContext);
    const [isOpen, setOpen] = createSignal(false, { ownedWrite: true });
    const [typed, setTyped] = createSignal("", { ownedWrite: true });
    const rest = omit(properties, "groups", "children");
    const fields = createMemo(() => Object.entries(control.fields()));
    const label = (): string => locale.render(t`Filter`);
    let trigger: HTMLElement | undefined;
    if (set === null) {
        handleAction(addFilter, { run: () => void setOpen(true) });
    }

    // add to the set, and open what a condition without a value asks for
    const add = (create: (set: number) => number, isAsked: boolean): void => {
        // close the menu and add it
        setOpen(false);
        const id = create((set ?? (() => control.root()))().id);
        flush();

        // focus what it compares with
        const scope = trigger?.closest("[data-slot=filter-group], [data-slot=filter]");
        const value = scope?.querySelector<HTMLElement>(`[data-rule="${String(id)}"]`);
        value?.focus();
        if (isAsked) {
            value?.click();
        }
    };

    return (
        <Popover open={isOpen()} onOpenChange={(next) => void setOpen(next)}>
            <PopoverTrigger
                render={(attributes) => (
                    <Button {...attributes} variant="ghost" size="sm" {...rest} />
                )}
                ref={(element: HTMLElement) => (trigger = element)}
                data-slot="filter-add"
            >
                {contentOr(properties, () => (
                    <>
                        <Icon icon={set === null ? funnelIcon : plusIcon} />
                        {label()}
                    </>
                ))}
            </PopoverTrigger>
            <PopoverContent align="start" xstyle={styles.menu}>
                <Command aria-label={label()}>
                    <CommandInput
                        placeholder={locale.render(t`Filter by…`)}
                        onValueChange={(next) => void setTyped(next)}
                    />
                    <CommandList>
                        <CommandEmpty>{locale.render(t`No results.`)}</CommandEmpty>
                        <CommandGroup>
                            <For each={fields()}>
                                {([name, field]) => (
                                    <CommandItem
                                        value={name}
                                        keywords={[field.label]}
                                        onSelect={() =>
                                            add((id) => control.add(name, [], id), true)
                                        }
                                    >
                                        <ValueIcon kind={field.type.kind} icon={field.icon} />
                                        {field.label}
                                    </CommandItem>
                                )}
                            </For>
                        </CommandGroup>
                        <Show when={typed().trim() !== ""}>
                            <CommandGroup>
                                <For each={fields()}>
                                    {([name, field]) => (
                                        <For each={control.suggestions(name, locale)}>
                                            {(choice) => (
                                                <CommandItem
                                                    value={`${name} ${JSON.stringify(choice.value)}`}
                                                    keywords={[choice.title]}
                                                    onSelect={() =>
                                                        add(
                                                            (id) =>
                                                                control.add(
                                                                    name,
                                                                    [choice.value],
                                                                    id,
                                                                ),
                                                            false,
                                                        )
                                                    }
                                                >
                                                    <ValueIcon
                                                        kind={field.type.kind}
                                                        icon={field.icon}
                                                    />
                                                    <span {...style.attrs(styles.of)}>
                                                        {field.label}
                                                    </span>
                                                    <ChoiceIcon choice={choice} />
                                                    {choice.title}
                                                </CommandItem>
                                            )}
                                        </For>
                                    )}
                                </For>
                            </CommandGroup>
                        </Show>
                        <Show when={properties.groups === true}>
                            <CommandSeparator />
                            <CommandGroup>
                                <CommandItem
                                    value="group"
                                    keywords={[locale.render(t`Filter group`)]}
                                    onSelect={() => add((id) => control.addSet(id), true)}
                                >
                                    <Icon icon={groupIcon} />
                                    {locale.render(t`Filter group`)}
                                </CommandItem>
                            </CommandGroup>
                        </Show>
                    </CommandList>
                </Command>
            </PopoverContent>
        </Popover>
    );
}

/** Render the button that shows every condition in a panel, counting them. */
export function FilterMenu(properties: ButtonProperties): JSX.Element {
    // count the conditions
    const control = useFilter();
    const locale = useLocale();
    const count = createMemo(() => countOf(control.root()));
    const rest = omit(properties, "children");

    return (
        <Popover>
            <PopoverTrigger
                render={(attributes) => (
                    <Button
                        {...attributes}
                        variant={count() > 0 ? "secondary" : "ghost"}
                        size="sm"
                        {...rest}
                    />
                )}
                data-slot="filter-menu"
            >
                {contentOr(properties, () => (
                    <>
                        <Icon icon={funnelIcon} />
                        {count() === 0 ? locale.render(t`Filter`) : countWords(count(), locale)}
                    </>
                ))}
            </PopoverTrigger>
            <PopoverContent align="start" xstyle={styles.panel}>
                <FilterGroup />
            </PopoverContent>
        </Popover>
    );
}

/** Render the nearest set of conditions as one item, its conditions in a panel. */
export function FilterSummary(properties: FilterElementProperties): JSX.Element {
    // count the set's conditions
    const locale = useLocale();
    const item = useFilterItem();
    const count = (): number => {
        const entry = item();

        return entry.kind === "set" ? countOf(entry) : 1;
    };

    return (
        <ButtonGroup
            divided
            data-slot="filter-summary"
            {...properties}
            xstyle={[styles.item, properties.xstyle]}
        >
            <Popover>
                <PopoverTrigger
                    render={(attributes) => (
                        <Button
                            {...attributes}
                            variant="secondary"
                            size="sm"
                            xstyle={styles.value}
                        />
                    )}
                    data-rule={item().id}
                >
                    <Icon icon={groupIcon} />
                    {countWords(count(), locale)}
                </PopoverTrigger>
                <PopoverContent align="start" xstyle={styles.panel}>
                    <FilterGroup removable={false} />
                </PopoverContent>
            </Popover>
            <FilterRemove />
        </ButtonGroup>
    );
}

/** Render the button that removes every condition, nothing without any. */
export function FilterClear(properties: ButtonProperties): JSX.Element {
    const control = useFilter();
    const locale = useLocale();

    return (
        <Show when={control.root().items.length > 0}>
            <Button
                data-slot="filter-clear"
                variant="ghost"
                size="sm"
                {...mergeProperties({ onClick: () => control.clear() }, properties)}
            >
                {contentOr(properties, () => locale.render(t`Clear filters`))}
            </Button>
        </Show>
    );
}

/** The properties of a group of conditions. */
export type FilterGroupProperties = FilterElementProperties & {
    /** Whether a person removes a nested group by its button, true by default. */
    readonly removable?: boolean;
};

/** Render a set of conditions below each other: the nearest set's, or every one. */
export function FilterGroup(properties: FilterGroupProperties): JSX.Element {
    // let the parts inside add to the set
    const control = useFilter();
    const item = useContext(ItemContext);
    const set = (): FilterSet => {
        const entry = item?.();

        return entry?.kind === "set" ? entry : control.root();
    };
    const isNested = untrack(() => item !== null && properties.removable !== false);
    const rest = omit(properties, "children", "removable");

    return (
        <SetContext value={set}>
            {renderPart("div", "filter-group", rest, [styles.group, isNested && styles.nested], {
                role: "group" as const,
                get children() {
                    return contentOr(properties, () => (
                        <>
                            <FilterList orientation="vertical" />
                            <div {...style.attrs(styles.footer)}>
                                <FilterAdd groups />
                                <Show when={isNested}>
                                    <FilterRemove variant="ghost" />
                                </Show>
                            </div>
                        </>
                    ));
                },
            })}
        </SetContext>
    );
}

/** Render how the nearest set's conditions combine, which a press changes, nothing for one. */
export function FilterMatch(properties: ButtonProperties): JSX.Element {
    // switch between all and any
    const control = useFilter();
    const locale = useLocale();
    const set = useContext(SetContext) ?? (() => control.root());
    const index = useContext(IndexContext);
    const isAll = (): boolean => set().match === "AND";
    const word = (): string => (isAll() ? locale.render(t`and`) : locale.render(t`or`));
    const words = (): string => {
        // say how all combine beside a list, and before a condition how it joins the others
        if (index !== null) {
            return word();
        }

        return isAll() ? locale.render(t`Match all filters`) : locale.render(t`Match any filter`);
    };

    return (
        <Show
            when={index === null || index() === 1}
            fallback={
                <span data-slot="filter-match" {...style.attrs(text.xs, styles.connective)}>
                    {index?.() === 0 ? locale.render(t`Where`) : word()}
                </span>
            }
        >
            <Show when={set().items.length > 1}>
                <Button
                    data-slot="filter-match"
                    variant={index === null ? "ghost" : "secondary"}
                    size="sm"
                    {...mergeProperties(
                        { onClick: () => control.match(set(), isAll() ? "OR" : "AND") },
                        properties,
                    )}
                    xstyle={[
                        styles.operator,
                        index !== null && styles.connective,
                        properties.xstyle,
                    ]}
                >
                    {contentOr(properties, () => words())}
                </Button>
            </Show>
        </Show>
    );
}

/** Read the condition parts are of, refusing a part outside one. */
export function useFilterItem(): () => FilterEntry {
    const item = useContext(ItemContext);
    if (item === null) {
        throw new TypeError("a part of a filter's item needs an item around it");
    }

    return item;
}

/** Count the conditions of a set, those of its own sets included. */
function countOf(set: FilterSet): number {
    return set.items.reduce((count, item) => count + (item.kind === "set" ? countOf(item) : 1), 0);
}

/** Write a count of conditions in words. */
function countWords(count: number, locale: Localization): string {
    return locale.render(t`${plural(count, { one: "# filter", other: "# filters" })}`);
}

/** Read an entry that is a condition on one field, undefined for another. */
function ruleOf(entry: FilterEntry): FilterRule | undefined {
    return entry.kind === "rule" ? entry : undefined;
}

/** Read the first of some operators, refusing a field without any. */
function present(operators: readonly FilterOperatorDefinition[]): FilterOperatorDefinition {
    const [first] = operators;
    if (first === undefined) {
        throw new TypeError("a filter's field has no operators");
    }

    return first;
}

/** List the choices a condition shows, none where it counts them. */
function listed(rule: FilterRule, choices: readonly Choice[]): readonly Choice[] {
    const chosen = choices.filter(({ value }) => rule.values.includes(value));

    return chosen.length > LISTED ? [] : chosen;
}

/** Write what a condition compares with, in words. */
function wordsOf(
    rule: FilterRule,
    choices: readonly Choice[],
    control: FilterControl,
    locale: Localization,
): string {
    // write each value as its field's kind is read, and count many
    const field = control.field(rule.field);
    const arity = control.operator(rule)?.arity;
    const words = rule.values
        .filter((value) => value !== null)
        .map((value) => wordOf(value, choices, field, arity, locale));
    if (words.length === 0) {
        return locale.render(t`Choose…`);
    } else if (arity === "range") {
        return words.join(" – ");
    }

    return words.length > LISTED
        ? locale.render(t`${locale.number(words.length)} chosen`)
        : locale.list(words, { type: "disjunction" });
}

/** Write one value of a condition, in words. */
function wordOf(
    value: JsonValue,
    choices: readonly Choice[],
    field: FilterFieldDefinition | undefined,
    arity: FilterOperatorDefinition["arity"] | undefined,
    locale: Localization,
): string {
    // write a span and a choice by their titles
    const span = FILTER_SPANS.find(({ name }) => arity === "span" && name === value);
    const choice = choices.find((each) => each.value === value);
    if (span !== undefined) {
        return locale.render(span.title);
    } else if (choice !== undefined) {
        return choice.title;
    } else if (typeof value === "number" && field?.type.kind === "time") {
        return locale.date(value, { dateStyle: "medium", timeStyle: "short" });
    } else if (typeof value === "number") {
        return locale.number(value, field?.type.kind === "number" ? field.type.format : undefined);
    }

    return typeof value === "string" ? value : JSON.stringify(value);
}
