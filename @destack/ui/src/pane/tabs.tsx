import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { color } from "@destack/theme/tokens.stylex";
import { For, type JSX, omit, untrack } from "@destack/view";
import { contentOr, mergeProperties, type PartAttributes } from "../part/index.ts";
import { Overflow, OverflowMenu } from "../overflow/index.ts";
import { SortableContainer, SortableItem } from "../sortable/index.ts";
import {
    Tabs,
    TabsClose,
    TabsItem,
    useTabs,
    type TabsItemProperties,
    TabsList,
    type TabsListProperties,
    tabsListVariants,
    type TabsProperties,
    TabsTitle,
    TabsTrigger,
    type TabsVariant,
} from "../tabs/index.ts";
import { usePane } from "./control.ts";
import { PANE_TYPE, TAB_TYPE } from "./drag.ts";
import { type PaneItem, PaneLayout } from "./layout.ts";
import { PaneBar } from "./pane.tsx";

/** The styles of a pane's tabs. */
const styles = style.create({
    tabs: {
        flexShrink: 0,
        gap: 0,
    },
    // step back as the selected tab of a pane the person is not in
    quiet: {
        color: { default: color.mutedForeground, [media.forcedColors]: "HighlightText" },
    },
});

/** The properties of a pane's tabs. */
export type PaneTabsProperties = Omit<
    TabsProperties,
    "value" | "defaultValue" | "onValueChange" | "onClose" | "idPrefix" | "children"
> &
    Pick<TabsListProperties, "variant" | "sizing"> & {
        /** Render an item's tab, the standard one by default. */
        readonly children?: (item: () => PaneItem) => JSX.Element;
        /** What comes after the tabs, such as a button that opens another. */
        readonly end?: JSX.Element;
    };

/** Render the items of a pane as tabs, which move within it and to others. */
export function PaneTabs(properties: PaneTabsProperties): JSX.Element {
    // select and close in the layout
    const { pane, group } = usePane();
    const rest = omit(properties, "children", "end", "variant", "sizing");
    const variant = (): TabsVariant => properties.variant ?? "card";

    // take what the pane takes among its tabs, besides panes
    const types = (): readonly string[] =>
        [TAB_TYPE, ...(group.entry(pane().id)?.accepts() ?? [])].filter(
            (type, index, all) => type !== PANE_TYPE && all.indexOf(type) === index,
        );

    // list the tabs of every pane where only the active one shows
    const items = (): readonly PaneItem[] => {
        if (!group.isSingle()) {
            return pane().items;
        }

        return group.layout().active === pane().id
            ? PaneLayout.panes(group.layout()).flatMap((each) => each.items)
            : [];
    };

    return (
        <Tabs
            xstyle={styles.tabs}
            {...rest}
            idPrefix={group.prefix}
            value={PaneLayout.selected(pane())?.id ?? ""}
            onValueChange={(item) => group.select(item)}
            reference={(item) => group.options.reference?.(item)}
            onClose={(item) => group.close(item)}
            onMove={(item, index) =>
                group.move({
                    id: item,
                    type: TAB_TYPE,
                    place: { kind: "item", pane: untrack(pane).id, index },
                    isCopied: false,
                    items: [],
                })
            }
            onReopen={group.closed().length === 0 ? undefined : () => group.reopen()}
        >
            <TabsBinding />
            <PaneBar
                render={(bar) => (
                    <Overflow {...bar} xstyle={tabsListVariants({ variant: variant() })}>
                        <SortableContainer
                            id={untrack(() => pane().id)}
                            orientation="horizontal"
                            accepts={types()}
                            height={group.placement(pane().id)?.height ?? 0}
                            render={(list) => (
                                <TabsList
                                    {...list}
                                    variant={variant()}
                                    sizing={properties.sizing ?? "shrink"}
                                />
                            )}
                        >
                            <For each={items()} keyed={(item) => item.id}>
                                {(item) =>
                                    properties.children === undefined ? (
                                        <PaneTab item={item()} />
                                    ) : (
                                        properties.children(item)
                                    )
                                }
                            </For>
                        </SortableContainer>
                        <OverflowMenu />
                        {properties.end}
                    </Overflow>
                )}
            />
        </Tabs>
    );
}

/** Give a pane its tabs. */
function TabsBinding(): JSX.Element {
    // read both, as it is in both
    const { setTabs } = usePane();
    setTabs(useTabs());

    return null;
}

/** The properties of a pane's tab. */
export type PaneTabProperties = Omit<TabsItemProperties, "value"> & {
    /** The item. */
    readonly item: PaneItem;
};

/** Render the tab of an item, with its value and a close button by default. */
export function PaneTab(properties: PaneTabProperties): JSX.Element {
    // drag the tab by itself
    const rest = omit(properties, "item", "children");
    const { pane, group } = usePane();
    const id = untrack(() => properties.item.id);

    // step back as the selected tab while another pane is active
    const isQuiet = (): boolean =>
        group.layout().active !== pane().id &&
        PaneLayout.panes(group.layout()).length > 1 &&
        PaneLayout.selected(pane())?.id === id;
    const item = (part: PartAttributes): JSX.Element => (
        <TabsItem
            {...part}
            {...rest}
            value={id}
            xstyle={[isQuiet() && styles.quiet, properties.xstyle]}
        />
    );

    return (
        <SortableItem
            id={id}
            type={TAB_TYPE}
            disabled={group.isLocked()}
            activator="item"
            sensors={["pointer"]}
            render={item}
        >
            {contentOr(properties, () => (
                <>
                    <PaneLink item={properties.item}>
                        <TabsTitle>
                            {group.options.label?.(properties.item) ?? properties.item.value}
                        </TabsTitle>
                    </PaneLink>
                    <TabsClose />
                </>
            ))}
        </SortableItem>
    );
}

/** Render the words of a tab, a link where its item has an address. */
function PaneLink(properties: {
    /** The item. */
    readonly item: PaneItem;
    /** The words. */
    readonly children: JSX.Element;
}): JSX.Element {
    // leave a press with a modifier to the browser, which opens the address elsewhere
    const { group } = usePane();
    const hasAddress = untrack(() => group.options.href !== undefined);
    const link = (part: PartAttributes): JSX.Element => (
        <a
            {...mergeProperties(part, {
                get href() {
                    return group.options.href?.(properties.item);
                },
                type: undefined,
                draggable: "false" as const,
                onKeyDown: (event: KeyboardEvent & { readonly currentTarget: HTMLElement }) => {
                    if (event.key === " ") {
                        event.preventDefault();
                        event.currentTarget.click();
                    }
                },
            })}
            onClick={(event) => {
                if (!event.metaKey && !event.ctrlKey && !event.shiftKey) {
                    event.preventDefault();
                    group.select(properties.item.id);
                }
            }}
        />
    );

    return hasAddress ? (
        <TabsTrigger render={link}>{properties.children}</TabsTrigger>
    ) : (
        <TabsTrigger>{properties.children}</TabsTrigger>
    );
}
