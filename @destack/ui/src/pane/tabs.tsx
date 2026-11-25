import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { color } from "@destack/theme/tokens.stylex";
import { createContext, For, type JSX, omit, Show, untrack, useContext } from "@destack/view";
import {
    ContextMenuContent,
    ContextMenuSeparator,
    ContextMenuTrigger,
} from "../context-menu/index.ts";
import { Menu, MenuControl } from "../menu/index.ts";
import type { PartAttributes } from "../part/index.ts";
import { Overflow, OverflowMenu } from "../overflow/index.ts";
import { SortableContainer, SortableItem } from "../sortable/index.ts";
import {
    Tabs,
    TabsClose,
    TabsItem,
    TabsMenuItems,
    useTabs,
    type TabsItemProperties,
    TabsList,
    type TabsListProperties,
    tabsListVariants,
    type TabsProperties,
    TabsTrigger,
    type TabsVariant,
} from "../tabs/index.ts";
import { usePane } from "./control.ts";
import { TAB_TYPE } from "./drag.ts";
import { type PaneItem, PaneLayout } from "./layout.ts";
import { PaneMenuItems } from "./menu.tsx";
import { PaneBar } from "./pane.tsx";

/** The styles of a pane's tabs. */
const styles = style.create({
    tabs: {
        flexShrink: 0,
        gap: 0,
    },
    // step back in the panes the person is not in
    quiet: {
        color: { default: color.mutedForeground, [media.forcedColors]: null },
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
        /** What the menu of a tab offers, the standard items by default and no menu for null. */
        readonly menu?: JSX.Element;
    };

/** The menu of the nearest pane's tabs, null without one. */
const PaneMenuContext = createContext<MenuControl | null>(null);

/** Render the items of a pane as tabs, which move within it and to others. */
export function PaneTabs(properties: PaneTabsProperties): JSX.Element {
    // select and close in the layout
    const { pane, group } = usePane();
    const rest = omit(properties, "children", "end", "menu", "variant", "sizing");
    const variant = (): TabsVariant => properties.variant ?? "card";

    // share one menu among the tabs
    const hasMenu = untrack(() => !("menu" in properties) || properties.menu !== null);
    const menu = new MenuControl(null);

    return (
        <Tabs
            xstyle={styles.tabs}
            {...rest}
            idPrefix={group.prefix}
            value={PaneLayout.selected(pane())?.id ?? ""}
            onValueChange={(item) => group.change((layout) => PaneLayout.select(layout, item))}
            onClose={(item) => group.close(item)}
            onMove={(item, index) =>
                group.move({
                    id: item,
                    type: TAB_TYPE,
                    place: { kind: "item", pane: untrack(pane).id, index },
                    isCopied: false,
                })
            }
            onReopen={group.closed().length === 0 ? undefined : () => group.reopen()}
        >
            <TabsBinding />
            <PaneMenuContext value={hasMenu ? menu : null}>
                <PaneBar
                    render={(bar) => (
                        <Overflow {...bar} xstyle={tabsListVariants({ variant: variant() })}>
                            <SortableContainer
                                id={untrack(() => pane().id)}
                                orientation="horizontal"
                                accepts={[TAB_TYPE]}
                                render={(list) => (
                                    <TabsList
                                        {...list}
                                        variant={variant()}
                                        sizing={properties.sizing ?? "shrink"}
                                    />
                                )}
                            >
                                <For each={pane().items} keyed={(item) => item.id}>
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
            </PaneMenuContext>
            <Show when={hasMenu}>
                <Menu control={menu}>
                    <ContextMenuContent>
                        {"menu" in properties ? (
                            properties.menu
                        ) : (
                            <>
                                <TabsMenuItems />
                                <ContextMenuSeparator />
                                <PaneMenuItems />
                            </>
                        )}
                    </ContextMenuContent>
                </Menu>
            </Show>
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
    const menu = useContext(PaneMenuContext);
    const id = untrack(() => properties.item.id);

    // step back while another pane is active, and open the pane's menu
    const isQuiet = (): boolean =>
        group.layout().active !== pane().id && PaneLayout.panes(group.layout()).length > 1;
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
            render={(part) =>
                menu === null ? (
                    item(part)
                ) : (
                    <ContextMenuTrigger {...part} control={menu} render={item} />
                )
            }
        >
            {"children" in properties ? (
                properties.children
            ) : (
                <>
                    <TabsTrigger>
                        {group.options.label?.(properties.item) ?? properties.item.value}
                    </TabsTrigger>
                    <TabsClose />
                </>
            )}
        </SortableItem>
    );
}
