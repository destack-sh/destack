import { Icon } from "@destack/icon";
import caretLeft from "@destack/icon/phosphor/caret-left";
import caretRight from "@destack/icon/phosphor/caret-right";
import * as style from "@destack/style";
import { color, motion, radius, space, stroke } from "@destack/theme/tokens.stylex";
import { useLocale } from "@destack/locale/solid";
import { text } from "@destack/theme/text";
import type { JSX } from "@solidjs/web";
import {
    createContext,
    createSignal,
    createUniqueId,
    omit,
    onCleanup,
    Show,
    useContext,
    type Accessor,
    type Setter,
} from "solid-js";
import { isTypeaheadKey } from "../focus/index.ts";
import type { Direction } from "@destack/locale";

/** The selector of a tree's items. */
const ITEM = "[role=treeitem]";

/** The tree of the nearest tree, null outside one. */
const TreeContext = createContext<TreeControl | null>(null);

/** The nesting depth of the items around, 1 for the top level. */
const TreeLevelContext = createContext(1);

/** The value of the nearest parent item, null at the top level. */
const TreeParentContext = createContext<string | null>(null);

/** The styles of a tree and its items. */
const styles = style.create({
    tree: {
        margin: 0,
        padding: 0,
        listStyle: "none",
        outlineStyle: "none",
    },
    item: {
        outlineStyle: "none",
    },
    indent: (level: number) => ({
        paddingInlineStart: `calc(${level - 1} * ${space[4]})`,
    }),
    row: {
        display: "flex",
        alignItems: "center",
        gap: space[1],
        paddingBlock: space[1],
        paddingInlineEnd: space[2],
        borderRadius: radius[2],
        cursor: "default",
        userSelect: "none",
        backgroundColor: { default: "transparent", ":hover": color.accent },
        transitionProperty: "background-color",
        transitionDuration: motion.durationShort,
    },
    focused: {
        outlineStyle: "solid",
        outlineWidth: stroke.ring,
        outlineOffset: `calc(-1 * ${stroke.ring})`,
        outlineColor: `color-mix(in oklab, ${color.ring} 50%, transparent)`,
    },
    selected: {
        backgroundColor: color.accent,
        color: color.accentForeground,
    },
    toggle: {
        display: "inline-flex",
        width: space[4],
        justifyContent: "center",
        flexShrink: 0,
        transitionProperty: "transform",
        transitionDuration: motion.durationShort,
    },
    expanded: {
        transform: "rotate(90deg)",
    },
    expandedBack: {
        transform: "rotate(-90deg)",
    },
    group: {
        margin: 0,
        padding: 0,
        listStyle: "none",
    },
});

/** The selected and focused items of a tree, which its items share. */
export class TreeControl {
    /** The properties of the tree root, read for its controlled selection. */
    readonly #properties: TreeProperties;
    /** The value of the item that holds the tab stop, the first item when none was focused. */
    readonly focused: Accessor<string | undefined>;
    /** The selected value when uncontrolled. */
    readonly #ownValue: Accessor<string | undefined>;
    /** Replace the selected value when uncontrolled. */
    readonly #setValue: Setter<string | undefined>;
    /** Replace the item that holds the tab stop. */
    readonly #setFocused: Setter<string | undefined>;
    /** The values of the top-level items in document order. */
    readonly #roots: Accessor<readonly string[]>;
    /** Replace the values of the top-level items. */
    readonly #setRoots: Setter<readonly string[]>;
    /** The parent and expansion of each item, by value. */
    readonly #nodes: Accessor<ReadonlyMap<string, TreeNode>>;
    /** Replace the items' parents and expansions. */
    readonly #setNodes: Setter<ReadonlyMap<string, TreeNode>>;

    /** Create the state of a tree from its root's properties. */
    constructor(properties: TreeProperties) {
        // start without a focused item or top-level items
        const [ownValue, setValue] = createSignal(properties.defaultValue);
        const [focused, setFocused] = createSignal<string | undefined>(undefined);
        const [roots, setRoots] = createSignal<readonly string[]>([], { ownedWrite: true });
        const [nodes, setNodes] = createSignal<ReadonlyMap<string, TreeNode>>(new Map(), {
            ownedWrite: true,
        });
        this.#properties = properties;
        this.#roots = roots;
        this.#setRoots = setRoots;
        this.#nodes = nodes;
        this.#setNodes = setNodes;
        this.focused = focused;
        this.#ownValue = ownValue;
        this.#setValue = setValue;
        this.#setFocused = setFocused;
    }

    /** Read the selected value, controlled or the tree's own. */
    value(): string | undefined {
        return "value" in this.#properties ? this.#properties.value : this.#ownValue();
    }

    /** Select an item and tell the change handler. */
    select(value: string): void {
        this.#setValue(value);
        this.#properties.onValueChange?.(value);
    }

    /** Add an item with its parent and expansion until the item unmounts, a top-level one among the roots. */
    register(value: string, node: TreeNode): void {
        // keep the item's place in the tree
        this.#setNodes((nodes) => new Map(nodes).set(value, node));
        onCleanup(() =>
            this.#setNodes((nodes) => {
                const remaining = new Map(nodes);
                remaining.delete(value);

                return remaining;
            }),
        );

        // keep a top-level item among the roots in document order
        if (node.parent === null) {
            this.#setRoots((roots) => [...roots, value]);
            onCleanup(() => this.#setRoots((roots) => roots.filter((entry) => entry !== value)));
        }
    }

    /** Report whether an item holds the tab stop: the focused, else the selected, else the first, or its outermost collapsed ancestor. */
    isTabStop(value: string): boolean {
        // walk up from the item asked for, stopping at each collapsed ancestor that hides it
        const nodes = this.#nodes();
        let stop = this.focused() ?? this.value() ?? this.#roots()[0];
        for (let parent = nodeParent(nodes, stop); parent !== null;) {
            if (nodes.get(parent)?.isExpanded() === false) {
                stop = parent;
            }
            parent = nodeParent(nodes, parent);
        }

        return stop === value;
    }

    /** Remember the item the focus rests on. */
    focus(value: string): void {
        this.#setFocused(value);
    }
}

/** An item's place in its tree: its parent and whether its children show. */
interface TreeNode {
    /** The value of the parent item, null for a top-level item. */
    readonly parent: string | null;
    /** Whether the item's children show. */
    readonly isExpanded: Accessor<boolean>;
}

/** Read the parent of an item, null for a top-level, unknown or absent one. */
function nodeParent(
    nodes: ReadonlyMap<string, TreeNode>,
    value: string | undefined,
): string | null {
    return value === undefined ? null : (nodes.get(value)?.parent ?? null);
}

/** The properties of a tree, the native list's attributes included. */
export interface TreeProperties extends Omit<
    JSX.HTMLAttributes<HTMLUListElement>,
    "class" | "style" | "onKeyDown"
> {
    /** The selected item's value, which makes the selection controlled. */
    readonly value?: string | undefined;
    /** The item selected at first when the selection is uncontrolled. */
    readonly defaultValue?: string;
    /** Handle another item being selected. */
    readonly onValueChange?: (value: string) => void;
    /** The StyleX styles applied after the tree's styles. */
    readonly style?: style.Styles;
}

/** The properties of a tree item, the native list item's attributes included. */
export interface TreeItemProperties extends Omit<
    JSX.LiHTMLAttributes<HTMLLIElement>,
    "class" | "style" | "value" | "onClick" | "onFocus"
> {
    /** The value the item stands for. */
    readonly value: string;
    /** The text or content of the item's row. */
    readonly label: JSX.Element;
    /** Whether the item's children show at first. */
    readonly defaultExpanded?: boolean;
    /** The StyleX styles applied after the row's styles. */
    readonly style?: style.Styles;
}

/** Read the tree of the nearest tree, refusing items outside one. */
function useTree(): TreeControl {
    const control = useContext(TreeContext);
    if (control === null) {
        throw new TypeError("tree items need a tree around them");
    }

    return control;
}

/** Render a tree of items that arrow keys, Home, End and typed letters move through and expand. */
export function Tree(properties: TreeProperties): JSX.Element {
    // share one tree with its items and read the reading direction
    const control = new TreeControl(properties);
    const locale = useLocale();
    const rest = omit(properties, "value", "defaultValue", "onValueChange", "style");

    return (
        <TreeContext value={control}>
            <ul
                role="tree"
                data-slot="tree"
                {...rest}
                onKeyDown={(event) => navigate(event, locale.direction)}
                {...style.attrs(text.footnote, styles.tree, properties.style)}
            />
        </TreeContext>
    );
}

/** Render an item of a tree, a parent with an expandable group when it has children. */
export function TreeItem(properties: TreeItemProperties): JSX.Element {
    // read the tree, the item's depth and whether it is a parent
    const control = useTree();
    const locale = useLocale();
    const level = useContext(TreeLevelContext);
    const parent = useContext(TreeParentContext);
    const rest = omit(properties, "value", "label", "defaultExpanded", "style", "children");
    const [isExpanded, setExpanded] = createSignal(properties.defaultExpanded === true);
    const isParent = (): boolean => "children" in properties;
    const isSelected = (): boolean => control.value() === properties.value;
    control.register(properties.value, { parent, isExpanded });

    // name the item by its own row
    const rowId = createUniqueId();

    return (
        <li
            role="treeitem"
            aria-labelledby={rowId}
            aria-level={level}
            aria-expanded={isParent() ? (isExpanded() ? "true" : "false") : undefined}
            aria-selected={isSelected() ? "true" : "false"}
            tabindex={control.isTabStop(properties.value) ? 0 : -1}
            data-slot="tree-item"
            data-value={properties.value}
            {...rest}
            onClick={(event) => {
                // select the clicked row's own item and toggle a parent
                event.stopPropagation();
                control.select(properties.value);
                if (isParent()) {
                    setExpanded(!isExpanded());
                }
            }}
            onFocus={(event) =>
                event.target === event.currentTarget && control.focus(properties.value)
            }
            onKeyDown={(event) => {
                // expand and collapse with the arrow keys along the reading direction
                if (event.target === event.currentTarget && isParent()) {
                    expandOrCollapse(event, isExpanded(), setExpanded, locale.direction);
                }
            }}
            {...style.attrs(styles.item)}
        >
            <TreeItemRow
                id={rowId}
                level={level}
                isParent={isParent()}
                isExpanded={isExpanded()}
                isFocused={control.focused() === properties.value}
                isSelected={isSelected()}
                style={properties.style}
            >
                {properties.label}
            </TreeItemRow>
            <Show when={isParent()}>
                <TreeGroup level={level} parent={properties.value} isExpanded={isExpanded()}>
                    {properties.children}
                </TreeGroup>
            </Show>
        </li>
    );
}

/** Render the group of a parent's children one level deeper, hidden while the parent is collapsed. */
function TreeGroup(properties: {
    readonly level: number;
    readonly parent: string;
    readonly isExpanded: boolean;
    readonly children: JSX.Element;
}): JSX.Element {
    return (
        <ul
            role="group"
            hidden={!properties.isExpanded}
            data-slot="tree-group"
            {...style.attrs(styles.group)}
        >
            <TreeLevelContext value={properties.level + 1}>
                <TreeParentContext value={properties.parent}>
                    {properties.children}
                </TreeParentContext>
            </TreeLevelContext>
        </ul>
    );
}

/** Render an item's row: indented by its level, with a chevron that turns as a parent expands. */
function TreeItemRow(properties: {
    readonly id: string;
    readonly level: number;
    readonly isParent: boolean;
    readonly isExpanded: boolean;
    readonly isFocused: boolean;
    readonly isSelected: boolean;
    readonly style: style.Styles | undefined;
    readonly children: JSX.Element;
}): JSX.Element {
    // turn the chevron along the reading direction
    const locale = useLocale();
    const isRightToLeft = (): boolean => locale.direction === "rtl";
    const turned = () => (isRightToLeft() ? styles.expandedBack : styles.expanded);

    return (
        <div
            id={properties.id}
            data-slot="tree-item-row"
            {...style.attrs(
                styles.row,
                styles.indent(properties.level),
                properties.isFocused && styles.focused,
                properties.isSelected && styles.selected,
                properties.style,
            )}
        >
            <span
                aria-hidden="true"
                {...style.attrs(styles.toggle, properties.isExpanded && turned())}
            >
                <Show when={properties.isParent}>
                    <Icon icon={isRightToLeft() ? caretLeft : caretRight} />
                </Show>
            </span>
            {properties.children}
        </div>
    );
}

/** Expand a closed parent or enter an open one, and collapse an open one, on the arrow keys along the reading direction. */
function expandOrCollapse(
    event: KeyboardEvent & { readonly currentTarget: HTMLLIElement },
    isExpanded: boolean,
    setExpanded: (isExpanded: boolean) => void,
    direction: Direction,
): void {
    // read the key that opens and the key that closes
    const opens = direction === "rtl" ? "ArrowLeft" : "ArrowRight";
    const closes = direction === "rtl" ? "ArrowRight" : "ArrowLeft";

    // expand, enter or collapse, leaving other cases to the tree
    if (event.key === opens && !isExpanded) {
        event.preventDefault();
        event.stopPropagation();
        setExpanded(true);
    } else if (event.key === opens && isExpanded) {
        event.preventDefault();
        event.stopPropagation();
        event.currentTarget.querySelector<HTMLElement>(ITEM)?.focus();
    } else if (event.key === closes && isExpanded) {
        event.preventDefault();
        event.stopPropagation();
        setExpanded(false);
    }
}

/** Move the focus among a tree's visible items, to a parent, or select with Enter and Space. */
function navigate(
    event: KeyboardEvent & { readonly currentTarget: HTMLUListElement },
    direction: Direction,
): void {
    // read the visible items and the focused one
    const items = visibleItems(event.currentTarget);
    const current =
        event.target instanceof HTMLElement ? event.target.closest<HTMLElement>(ITEM) : null;
    const index = current === null ? -1 : items.indexOf(current);
    const closes = direction === "rtl" ? "ArrowRight" : "ArrowLeft";

    // pick the target of the key
    const target =
        event.key === "ArrowDown"
            ? items[index + 1]
            : event.key === "ArrowUp"
              ? items[index - 1]
              : event.key === "Home"
                ? items[0]
                : event.key === "End"
                  ? items.at(-1)
                  : event.key === closes
                    ? (current?.parentElement?.closest<HTMLElement>(ITEM) ?? undefined)
                    : isTypeaheadKey(event)
                      ? typeahead(event.key, items, index)
                      : undefined;

    // select on Enter and Space, else focus the target
    if (event.key === "Enter" || event.key === " ") {
        event.preventDefault();
        current?.click();
    } else if (target !== undefined) {
        event.preventDefault();
        target.focus();
    }
}

/** List a tree's items outside collapsed groups, in document order. */
function visibleItems(tree: Element): HTMLElement[] {
    return [...tree.querySelectorAll<HTMLElement>(ITEM)].filter(
        (item) => item.parentElement?.closest("[role=group][hidden]") === null,
    );
}

/** Find the next item after an index whose text starts with a letter, wrapping around. */
function typeahead(
    letter: string,
    items: readonly HTMLElement[],
    index: number,
): HTMLElement | undefined {
    const ordered = [...items.slice(index + 1), ...items.slice(0, index + 1)];

    return ordered.find((item) =>
        (item.querySelector("[data-slot=tree-item-row]")?.textContent ?? "")
            .trim()
            .toLowerCase()
            .startsWith(letter.toLowerCase()),
    );
}
