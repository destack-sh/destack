import type { IconBodies } from "@destack/icon";
import caretRight from "@destack/icon/phosphor/caret-right";
import { Shortcut, t } from "@destack/locale";
import * as style from "@destack/style";
import { color, radius, shadow, space, stroke, swatch, width } from "@destack/theme/tokens.stylex";
import { createMemo, For, type JSX, onSettled, Show, useLocale } from "@destack/view";
import {
    Command,
    CommandBody,
    CommandDetail,
    CommandDialog,
    CommandEmpty,
    CommandFooter,
    CommandFooterLocation,
    CommandGroup,
    CommandHint,
    CommandInput,
    CommandItem,
    CommandItemDescription,
    CommandItemMeta,
    CommandList,
    CommandShortcut,
    useCommand,
} from "../command/index.ts";
import { Badge } from "../badge/index.ts";
import { Icon } from "../icon/index.ts";
import { useListBox } from "../list-box/index.ts";
import { type PaletteItem, PaletteControl, type PalettePage } from "./control.ts";

/** The styles of a palette's actions panel and its entries' marks. */
const styles = style.create({
    panel: {
        // stay inside the palette however many actions there are, the list scrolling
        position: "absolute",
        display: "flex",
        maxHeight: `calc(100% - ${space[8]} - 2 * ${space[2]})`,
        insetBlockEnd: `calc(${space[8]} + ${space[2]})`,
        insetInlineEnd: space[3],
        zIndex: 1,
        width: width.popover,
        borderWidth: stroke.border,
        borderStyle: "solid",
        borderColor: color.border,
        borderRadius: radius[3],
        boxShadow: shadow.overlay,
        overflow: "hidden",
    },
    destructive: {
        color: swatch.destructiveText,
    },
    blank: {
        flexShrink: 0,
        width: "1em",
    },
    drill: {
        color: color.mutedForeground,
    },
});

/** The properties of a command palette. */
export interface CommandPaletteProperties {
    /** The key the palette remembers its entries' uses under, unique on the page. */
    readonly id: string;
    /** The root page: its groups of entries. */
    readonly root: PalettePage;
    /** Whether it is open. */
    readonly open: boolean;
    /** Handle it opening or closing. */
    readonly onOpenChange: (open: boolean) => void;
}

/** Render a command palette its caller opens: search, pages, recents, details and actions. */
export function CommandPalette(properties: CommandPaletteProperties): JSX.Element {
    return (
        <CommandDialog
            open={properties.open}
            onOpenChange={(open) => properties.onOpenChange(open)}
            shouldFilter={false}
            loop
        >
            <PaletteContent
                id={properties.id}
                root={properties.root}
                close={() => properties.onOpenChange(false)}
            />
        </CommandDialog>
    );
}

/** Render a palette's search, entries, detail, footer and actions panel inside its command. */
function PaletteContent(properties: {
    /** The key the palette remembers its entries' uses under. */
    readonly id: string;
    /** The root page. */
    readonly root: PalettePage;
    /** Close the palette. */
    readonly close: () => void;
}): JSX.Element {
    // fill the command's search and pages with the palette's entries
    const locale = useLocale();
    const command = useCommand();
    const list = useListBox();
    let search: HTMLInputElement | undefined;
    const control = new PaletteControl({
        command,
        root: () => properties.root,
        key: properties.id,
        close: properties.close,
    });
    onSettled(() => control.restore());

    // rank the shown page's entries and read the highlighted one
    const results = createMemo(() => control.results());
    const entries = createMemo(() => [
        ...results().recent,
        ...results().groups.flatMap((result) => result.items),
    ]);
    const highlighted = (): PaletteItem | undefined => {
        const value = list.activeValue();

        return entries().find((item) => item.id === value);
    };

    // open the actions on Command or Control with K, run the second action with Enter, drill with Tab
    const keys = (event: KeyboardEvent) => {
        // read the highlighted entry, its second action and the action whose keys were pressed
        const item = highlighted();
        const isCommand = Shortcut.isCommand(navigator.userAgent);
        const second = item?.actions[1];
        const shortcut = item?.actions.find(
            (action) =>
                action.shortcut !== undefined &&
                Shortcut.matches(action.shortcut, event, isCommand),
        );
        if (control.acting() !== undefined && event.key === "Escape") {
            event.preventDefault();
            control.act(undefined);
        } else if (item === undefined) {
            return;
        } else if (Shortcut.matches("mod+k", event, isCommand)) {
            event.preventDefault();
            control.act(control.acting() === undefined ? item : undefined);
        } else if (second !== undefined && Shortcut.matches("mod+enter", event, isCommand)) {
            event.preventDefault();
            void control.run(item, second);
        } else if (event.key === "Tab" && !event.shiftKey && item.page !== undefined) {
            event.preventDefault();
            control.drill(item);
        } else if (shortcut !== undefined) {
            event.preventDefault();
            void control.run(item, shortcut);
        }
    };

    return (
        <>
            <CommandInput
                ref={(element) => (search = element)}
                placeholder={
                    control.page().placeholder ?? locale.render(t`Search or run a command`)
                }
                onKeyDown={keys}
            />
            <CommandBody>
                <CommandList>
                    <CommandEmpty>{locale.render(t`No results.`)}</CommandEmpty>
                    <Show when={results().recent.length > 0}>
                        <CommandGroup heading={locale.render(t`Recent`)}>
                            <For each={results().recent}>
                                {(item) => <PaletteEntry item={item} control={control} />}
                            </For>
                        </CommandGroup>
                    </Show>
                    <For each={results().groups}>
                        {(result) => (
                            <CommandGroup heading={result.group.title}>
                                <For each={result.items}>
                                    {(item) => <PaletteEntry item={item} control={control} />}
                                </For>
                            </CommandGroup>
                        )}
                    </For>
                </CommandList>
                <Show when={entries().some((item) => item.detail !== undefined)}>
                    <CommandDetail>
                        {(value) =>
                            entries()
                                .find((item) => item.id === value)
                                ?.detail?.()
                        }
                    </CommandDetail>
                </Show>
            </CommandBody>
            <CommandFooter>
                <CommandFooterLocation>{control.page().title}</CommandFooterLocation>
                <Show when={highlighted()}>
                    {(item) => (
                        <>
                            <Show when={item().actions[0]}>
                                {(action) => (
                                    <CommandHint keys="enter">{action().title}</CommandHint>
                                )}
                            </Show>
                            <Show when={item().page !== undefined}>
                                <CommandHint keys="tab">{locale.render(t`Show`)}</CommandHint>
                            </Show>
                            <CommandHint keys="mod+k">{locale.render(t`Actions`)}</CommandHint>
                        </>
                    )}
                </Show>
            </CommandFooter>
            <Show when={control.acting()}>
                {(item) => (
                    <ActionsPanel
                        item={item()}
                        control={control}
                        onClose={() => {
                            control.act(undefined);
                            search?.focus();
                        }}
                    />
                )}
            </Show>
        </>
    );
}

/** Render an entry: its icon, title, subtitle, accessory and whether it opens a page. */
function PaletteEntry(properties: {
    /** The entry. */
    readonly item: PaletteItem;
    /** The palette, which runs it. */
    readonly control: PaletteControl;
}): JSX.Element {
    const item = properties.item;

    return (
        <CommandItem
            value={item.id}
            onSelect={() => {
                // open the entry's page when it has no action, else run its first action
                if (item.actions.length === 0) {
                    properties.control.drill(item);
                } else {
                    void properties.control.run(item);
                }
            }}
        >
            <Mark
                icon={item.icon}
                isAligned={properties.control
                    .page()
                    .groups.some((group) => group.items().some((each) => each.icon !== undefined))}
            />
            {item.title}
            <Show when={item.subtitle}>
                {(subtitle) => <CommandItemDescription>{subtitle()}</CommandItemDescription>}
            </Show>
            <CommandItemMeta>
                <For each={item.accessories ?? []}>
                    {(accessory) => (
                        <Show when={accessory.color} fallback={accessory.text}>
                            {(tint) => <Badge color={tint()}>{accessory.text}</Badge>}
                        </Show>
                    )}
                </For>
                <Show when={item.shortcut}>{(keys) => <CommandShortcut keys={keys()} />}</Show>
                <Show when={item.page !== undefined}>
                    <Icon icon={caretRight} xstyle={styles.drill} />
                </Show>
            </CommandItemMeta>
        </CommandItem>
    );
}

/** Render an entry's icon, its room among entries with icons, so every title starts on one edge. */
function Mark(properties: {
    /** The icon, none for an entry without one. */
    readonly icon: IconBodies | undefined;
    /** Whether another entry beside it has an icon. */
    readonly isAligned: boolean;
}): JSX.Element {
    return (
        <Show
            when={properties.icon}
            fallback={
                <Show when={properties.isAligned}>
                    <span aria-hidden="true" {...style.attrs(styles.blank)} />
                </Show>
            }
        >
            {(icon) => <Icon icon={icon()} />}
        </Show>
    );
}

/** Render an entry's actions in a panel over the palette's corner. */
function ActionsPanel(properties: {
    /** The entry whose actions the panel lists. */
    readonly item: PaletteItem;
    /** The palette, which runs them. */
    readonly control: PaletteControl;
    /** Close the panel and return to the palette's search. */
    readonly onClose: () => void;
}): JSX.Element {
    // take the focus into the panel's search once it shows
    const locale = useLocale();
    let search: HTMLInputElement | undefined;
    onSettled(() => search?.focus());

    return (
        <div data-slot="palette-actions" {...style.attrs(styles.panel)}>
            <Command aria-label={locale.render(t`Actions`)} loop>
                <CommandInput
                    ref={(element) => (search = element)}
                    placeholder={locale.render(t`Search actions`)}
                    onKeyDown={(event) => {
                        // close the panel on Escape or Command K, leaving the palette open
                        const isCommand = Shortcut.isCommand(navigator.userAgent);
                        if (event.key === "Escape" || Shortcut.matches("mod+k", event, isCommand)) {
                            event.preventDefault();
                            properties.onClose();
                        }
                    }}
                />
                <CommandList>
                    <CommandEmpty>{locale.render(t`No results.`)}</CommandEmpty>
                    <CommandGroup heading={properties.item.title}>
                        <For each={properties.item.actions}>
                            {(action) => (
                                <CommandItem
                                    value={action.id}
                                    keywords={[action.title]}
                                    xstyle={action.isDestructive === true && styles.destructive}
                                    onSelect={() =>
                                        void properties.control.run(properties.item, action)
                                    }
                                >
                                    <Mark
                                        icon={action.icon}
                                        isAligned={properties.item.actions.some(
                                            (each) => each.icon !== undefined,
                                        )}
                                    />
                                    {action.title}
                                    <Show when={action.shortcut}>
                                        {(keys) => <CommandShortcut keys={keys()} />}
                                    </Show>
                                </CommandItem>
                            )}
                        </For>
                    </CommandGroup>
                </CommandList>
            </Command>
        </div>
    );
}
