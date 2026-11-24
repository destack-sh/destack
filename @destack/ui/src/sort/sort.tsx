import ascendingIcon from "@destack/icon/phosphor/arrow-up";
import descendingIcon from "@destack/icon/phosphor/arrow-down";
import plusIcon from "@destack/icon/phosphor/plus";
import sortIcon from "@destack/icon/phosphor/arrows-down-up";
import handleIcon from "@destack/icon/phosphor/dots-six-vertical";
import xIcon from "@destack/icon/phosphor/x";
import { plural, t } from "@destack/locale";
import * as style from "@destack/style";
import { color, radius, size, space, stroke, weight } from "@destack/theme/tokens.stylex";
import {
    createContext,
    createEffect,
    createMemo,
    createSignal,
    flush,
    For,
    type JSX,
    omit,
    Show,
    useContext,
    useLocale,
} from "@destack/view";
import { Button, type ButtonProperties } from "../button/index.ts";
import { ButtonGroup } from "../button-group/index.ts";
import {
    Command,
    CommandEmpty,
    CommandGroup,
    CommandInput,
    CommandItem,
    CommandList,
} from "../command/index.ts";
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuTrigger,
} from "../dropdown-menu/index.ts";
import { Icon } from "../icon/index.ts";
import { useJoin } from "../join/index.ts";
import { type ElementPartProperties, mergeProperties, renderPart } from "../part/index.ts";
import { Popover, PopoverContent, PopoverTrigger } from "../popover/index.ts";
import {
    Sortable,
    SortableContainer,
    SortableHandle,
    type SortableHandleProperties,
    SortableItem,
} from "../sortable/index.ts";
import { ValueIcon } from "../value-field/index.ts";
import { SortContext, SortControl, type SortOptions, useSort } from "./control.ts";

/** The styles of a sort and its parts. */
const styles = style.create({
    sort: {
        display: "flex",
        flexWrap: "wrap",
        alignItems: "center",
        gap: space[2],
        minWidth: 0,
    },
    list: {
        display: "flex",
        flexWrap: "wrap",
        gap: space[2],
        minWidth: 0,
    },
    vertical: {
        flexDirection: "column",
        alignItems: "flex-start",
    },
    wide: {
        flexBasis: "100%",
    },
    handle: {
        alignSelf: "stretch",
        height: size[1],
        width: size[1],
        borderWidth: stroke.border,
        borderColor: "transparent",
        borderRadius: radius[3],
        backgroundColor: color.muted,
    },
    panel: {
        display: "grid",
        justifyItems: "start",
        gap: space[2],
        width: "max-content",
        maxWidth: `calc(100vw - 2 * ${space[1]})`,
        padding: space[3],
    },
    footer: {
        display: "flex",
        flexWrap: "wrap",
        gap: space[1],
    },
    label: {
        fontWeight: weight.regular,
    },
    menu: {
        padding: 0,
    },
});

/** The field the parts of an item are of, null outside one. */
const ItemContext = createContext<string | null>(null);

/** The properties of an element of a sort, with its own attributes. */
export type SortElementProperties = Omit<JSX.HTMLAttributes<HTMLDivElement>, "class"> &
    ElementPartProperties;

/** The properties of a sort, with its element's attributes. */
export type SortProperties = SortOptions & Omit<SortElementProperties, keyof SortOptions>;

/** Render an order a person edits: its parts, or those of a usual sort. */
export function Sort(properties: SortProperties): JSX.Element {
    // share the keys with the parts
    const control = new SortControl(properties);
    const locale = useLocale();
    const rest = omit(properties, "fields", "value", "defaultValue", "onValueChange", "children");

    return (
        <SortContext value={control}>
            {renderPart("div", "sort", rest, styles.sort, {
                role: "group" as const,
                "aria-label": locale.render(t`Sort`),
                get children() {
                    return properties.children ?? <SortMenu />;
                },
            })}
        </SortContext>
    );
}

/** The properties of a list of the fields rows sort by. */
export interface SortListProperties {
    /** The direction its items lay out along, horizontal by default. */
    readonly orientation?: "horizontal" | "vertical";
    /** Render a field, the usual parts by default. */
    readonly children?: (field: string) => JSX.Element;
}

/** Render the fields rows sort by, dragged into another order. */
export function SortList(properties: SortListProperties): JSX.Element {
    // follow each field by its name
    const control = useSort();
    const locale = useLocale();
    const names = createMemo(() => control.keys().map(({ field }) => field));
    const orientation = () => properties.orientation ?? "horizontal";

    return (
        <Show when={names().length > 0}>
            <Sortable
                onMove={(move) => control.move(move.id, move.next)}
                xstyle={orientation() === "vertical" && styles.wide}
            >
                <SortableContainer
                    id="sort"
                    label={locale.render(t`Sort`)}
                    orientation={orientation()}
                    xstyle={[styles.list, orientation() === "vertical" && styles.vertical]}
                >
                    <For each={names()}>
                        {(name) => (
                            <ItemContext value={name}>
                                {properties.children?.(name) ?? <SortItem />}
                            </ItemContext>
                        )}
                    </For>
                </SortableContainer>
            </Sortable>
        </Show>
    );
}

/** Render the nearest field rows sort by, joined into one control: its parts, or the usual ones. */
export function SortItem(properties: SortElementProperties): JSX.Element {
    // let a drag move it among others
    const control = useSort();
    const name = useSortItem();
    const rest = omit(properties, "children");

    return (
        <SortableItem id={name} label={control.field(name)?.label ?? name}>
            <ButtonGroup
                divided
                data-slot="sort-item"
                aria-label={control.field(name)?.label}
                {...rest}
            >
                {properties.children ?? (
                    <>
                        <SortHandle />
                        <SortField />
                        <SortDirection />
                        <SortRemove />
                    </>
                )}
            </ButtonGroup>
        </SortableItem>
    );
}

/** Render the button that drags the nearest field, nothing while it is the only one. */
export function SortHandle(properties: SortableHandleProperties): JSX.Element {
    const control = useSort();
    const join = useJoin();

    return (
        <Show when={control.keys().length > 1}>
            <SortableHandle
                data-slot="sort-handle"
                {...properties}
                xstyle={[styles.handle, join(), properties.xstyle]}
            >
                {properties.children ?? <Icon icon={handleIcon} />}
            </SortableHandle>
        </Show>
    );
}

/** Render the nearest field, which a menu changes to another. */
export function SortField(properties: ButtonProperties): JSX.Element {
    // list the fields rows do not sort by yet
    const control = useSort();
    const name = useSortItem();
    const others = createMemo(() => control.others());
    const rest = omit(properties, "children");

    return (
        <DropdownMenu>
            <DropdownMenuTrigger
                render={(attributes) => (
                    <Button
                        {...attributes}
                        variant="secondary"
                        size="sm"
                        {...rest}
                        xstyle={[styles.label, properties.xstyle]}
                    />
                )}
                disabled={others().length === 0}
                data-slot="sort-field"
            >
                {properties.children ?? <FieldLabel name={name} />}
            </DropdownMenuTrigger>
            <DropdownMenuContent align="start">
                <For each={others()}>
                    {(other) => (
                        <DropdownMenuItem onSelect={() => control.replace(name, other)}>
                            <FieldLabel name={other} />
                        </DropdownMenuItem>
                    )}
                </For>
            </DropdownMenuContent>
        </DropdownMenu>
    );
}

/** Render a field's icon and what it is called. */
function FieldLabel(properties: {
    /** The field. */
    readonly name: string;
}): JSX.Element {
    const control = useSort();
    const field = () => control.field(properties.name);

    return (
        <>
            <Show when={field()?.icon ?? field()?.type}>
                <ValueIcon kind={field()?.type?.kind ?? "text"} icon={field()?.icon} />
            </Show>
            {field()?.label ?? properties.name}
        </>
    );
}

/** Render the direction of the nearest field, which a press turns around. */
export function SortDirection(properties: ButtonProperties): JSX.Element {
    // read the direction and what it is called
    const control = useSort();
    const locale = useLocale();
    const name = useSortItem();
    const direction = () => control.direction(name) ?? "asc";

    return (
        <Button
            data-slot="sort-direction"
            variant="secondary"
            size="sm"
            {...mergeProperties(
                { onClick: () => control.set(name, direction() === "asc" ? "desc" : "asc") },
                properties,
            )}
            xstyle={[styles.label, properties.xstyle]}
        >
            {properties.children ?? (
                <>
                    <Icon icon={direction() === "asc" ? ascendingIcon : descendingIcon} />
                    {control.words(name, direction(), locale)}
                </>
            )}
        </Button>
    );
}

/** Render the button that sorts by the nearest field no more, the focus left on the next. */
export function SortRemove(properties: ButtonProperties): JSX.Element {
    // find where it is, as it goes away with the field
    const control = useSort();
    const locale = useLocale();
    const name = useSortItem();
    const remove = ({ currentTarget }: { readonly currentTarget: HTMLElement }): void => {
        // list the buttons before and after
        const root = currentTarget.closest("[data-slot=sort]");
        const buttons = (): HTMLElement[] => [
            ...(root?.querySelectorAll<HTMLElement>("[data-slot=sort-remove]") ?? []),
        ];
        const place = buttons().indexOf(currentTarget);
        control.remove(name);
        flush();

        // focus the next field's button, else the one before, else the menu
        const others = buttons();
        const menu = root?.querySelector<HTMLElement>(
            "[data-slot=sort-menu], [data-slot=sort-add]",
        );
        (others[place] ?? others.at(-1) ?? menu)?.focus();
    };

    return (
        <Button
            data-slot="sort-remove"
            variant="secondary"
            size="icon-sm"
            aria-label={locale.render(t`Remove sort`)}
            {...mergeProperties({ onClick: remove }, properties)}
        >
            {properties.children ?? <Icon icon={xIcon} />}
        </Button>
    );
}

/** Render the button that adds a field, nothing once rows sort by each. */
export function SortAdd(properties: ButtonProperties): JSX.Element {
    // list the fields left
    const control = useSort();
    const locale = useLocale();
    const [isOpen, setOpen] = createSignal(false, { ownedWrite: true });
    const rest = omit(properties, "children");
    const others = createMemo(() => control.others());
    let trigger: HTMLElement | undefined;
    const add = (name: string): void => {
        // close the menu, and hand the focus to the field's direction
        const root = trigger?.closest("[data-slot=sort]");
        setOpen(false);
        control.set(name);
        flush();
        [...(root?.querySelectorAll<HTMLElement>("[data-slot=sort-direction]") ?? [])]
            .at(-1)
            ?.focus();
    };

    return (
        <Show when={others().length > 0 || isOpen()}>
            <Popover open={isOpen()} onOpenChange={(next) => void setOpen(next)}>
                <PopoverTrigger
                    render={(attributes) => (
                        <Button {...attributes} variant="ghost" size="sm" {...rest} />
                    )}
                    ref={(element: HTMLElement) => (trigger = element)}
                    data-slot="sort-add"
                >
                    {properties.children ?? (
                        <>
                            <Icon icon={plusIcon} />
                            {locale.render(t`Sort`)}
                        </>
                    )}
                </PopoverTrigger>
                <PopoverContent align="start" xstyle={styles.menu}>
                    <FieldCommand onSelect={add} />
                </PopoverContent>
            </Popover>
        </Show>
    );
}

/** Render the fields rows do not sort by yet, searched. */
function FieldCommand(properties: {
    /** Take the field a person chooses. */
    readonly onSelect: (name: string) => void;
}): JSX.Element {
    // list the fields left
    const control = useSort();
    const locale = useLocale();
    const others = createMemo(() => control.others());

    return (
        <Command aria-label={locale.render(t`Sort`)}>
            <CommandInput placeholder={locale.render(t`Sort by…`)} />
            <CommandList>
                <CommandEmpty>{locale.render(t`No results.`)}</CommandEmpty>
                <CommandGroup>
                    <For each={others()}>
                        {(name) => (
                            <CommandItem
                                value={name}
                                keywords={[control.field(name)?.label ?? name]}
                                onSelect={() => properties.onSelect(name)}
                            >
                                <FieldLabel name={name} />
                            </CommandItem>
                        )}
                    </For>
                </CommandGroup>
            </CommandList>
        </Command>
    );
}

/** Render the button of an order: its fields to choose from, then a panel to change it in. */
export function SortMenu(properties: ButtonProperties): JSX.Element {
    // open on the fields to choose from, and on the panel once there is one
    const control = useSort();
    const locale = useLocale();
    const rest = omit(properties, "children");
    const [isOpen, setOpen] = createSignal(false, { ownedWrite: true });
    let content: HTMLElement | undefined;

    // show the one field and its direction, or how many there are
    const only = () => (control.keys().length === 1 ? control.keys()[0] : undefined);

    // hand the focus to the field's direction in the panel
    const start = (name: string): void => {
        control.set(name);
        flush();
        content?.querySelector<HTMLElement>("[data-slot=sort-direction]")?.focus();
    };

    // close the panel with its last field
    createEffect(
        () => control.keys().length,
        (count, before) => {
            if (count === 0 && (before ?? 0) > 0) {
                setOpen(false);
            }
        },
    );

    return (
        <Popover open={isOpen()} onOpenChange={(next) => void setOpen(next)}>
            <PopoverTrigger
                render={(attributes) => (
                    <Button
                        {...attributes}
                        variant={control.keys().length > 0 ? "secondary" : "ghost"}
                        size="sm"
                        {...rest}
                    />
                )}
                data-slot="sort-menu"
            >
                {properties.children ?? (
                    <Show
                        when={only()}
                        fallback={
                            <>
                                <Icon icon={sortIcon} />
                                {control.keys().length === 0
                                    ? locale.render(t`Sort`)
                                    : locale.render(
                                          t`${plural(control.keys().length, { one: "# sort", other: "# sorts" })}`,
                                      )}
                            </>
                        }
                    >
                        {(key) => (
                            <>
                                <Icon
                                    icon={
                                        key().direction === "asc" ? ascendingIcon : descendingIcon
                                    }
                                />
                                {control.field(key().field)?.label ?? key().field}
                            </>
                        )}
                    </Show>
                )}
            </PopoverTrigger>
            <PopoverContent
                ref={(element: HTMLElement) => (content = element)}
                align="start"
                xstyle={control.keys().length > 0 ? styles.panel : styles.menu}
            >
                <Show when={control.keys().length > 0} fallback={<FieldCommand onSelect={start} />}>
                    <SortList orientation="vertical" />
                    <div {...style.attrs(styles.footer)}>
                        <SortAdd />
                        <SortClear />
                    </div>
                </Show>
            </PopoverContent>
        </Popover>
    );
}

/** Render the button that sorts by nothing, nothing while rows do. */
export function SortClear(properties: ButtonProperties): JSX.Element {
    const control = useSort();
    const locale = useLocale();

    return (
        <Show when={control.keys().length > 0}>
            <Button
                data-slot="sort-clear"
                variant="ghost"
                size="sm"
                {...mergeProperties({ onClick: () => control.clear() }, properties)}
            >
                {properties.children ?? locale.render(t`Clear sort`)}
            </Button>
        </Show>
    );
}

/** Read the field the parts of an item are of, refusing a part outside one. */
export function useSortItem(): string {
    const name = useContext(ItemContext);
    if (name === null) {
        throw new TypeError("a part of a sort's item needs an item around it");
    }

    return name;
}
