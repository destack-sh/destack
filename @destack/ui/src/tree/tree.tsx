import { Icon } from "@destack/icon";
import caretLeft from "@destack/icon/phosphor/caret-left";
import caretRight from "@destack/icon/phosphor/caret-right";
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { color, motion, radius, space, stroke } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import {
    type Accessor,
    createContext,
    createControllableSignal,
    createMemo,
    createUniqueId,
    type JSX,
    omit,
    Show,
    useContext,
    useLocale,
} from "@destack/view";
import type { Direction } from "@destack/locale";
import { Collection, CollectionBuilder } from "../collection/index.ts";
import { Focus, ListDelegate } from "../focus/index.ts";
import { Selection, type SingleSelection } from "../selection/index.ts";

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
        backgroundColor: {
            default: "transparent",
            ":hover": { default: null, [media.hover]: color.accent },
        },
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

/** The items, selection and focused item of a tree, which its items share. */
export class TreeControl {
    /** The selected value, one at most, controlled or the tree's own. */
    readonly selection: Selection;
    /** The items in the order of their elements. */
    readonly nodes: CollectionBuilder<TreeNode>;
    /** The items outside collapsed parents, in order. */
    readonly visible: Collection<TreeNode>;
    /** The visible items the arrow keys, Home, End and typed letters move through. */
    readonly delegate: ListDelegate<TreeNode>;
    /** The focused item, which holds the tab stop. */
    readonly focus: Focus<string>;
    /** The item of each value. */
    readonly #byValue: Accessor<ReadonlyMap<string, TreeNode>>;

    /** Create the state of a tree from its root's properties. */
    constructor(properties: TreeProperties) {
        // follow the controlled value or the tree's own, reporting each selected item
        const onValueChange = (next: string | undefined): void => {
            if (next !== undefined) {
                properties.onValueChange?.(next);
            }
        };
        this.selection = new Selection(
            "value" in properties
                ? {
                      get value() {
                          return properties.value;
                      },
                      onValueChange,
                  }
                : { ...defaultOf(properties.defaultValue), onValueChange },
        );

        // keep the items outside collapsed parents, found by value
        this.nodes = new CollectionBuilder((node) => node.element());
        this.#byValue = createMemo(
            () => new Map(this.nodes.items().map((node) => [node.value, node])),
        );
        this.visible = new Collection({
            sections: () => [
                { key: "visible", items: this.nodes.items().filter((node) => this.isShown(node)) },
            ],
            key: (node) => node.value,
            text: (node) =>
                node.element()?.querySelector("[data-slot=tree-item-row]")?.textContent ?? "",
        });

        // move through the visible items, the selected one or its outermost collapsed ancestor holding the tab stop
        this.delegate = new ListDelegate(this.visible, {
            orientation: "vertical",
            isLooping: false,
            isTypeahead: true,
        });
        this.focus = new Focus({
            delegate: this.delegate,
            mode: "roving",
            initial: () => this.lift(this.selection.values()[0] ?? this.visible.at(0)),
        });
    }

    /** Read the item of a value, undefined for no such item. */
    node(value: string): TreeNode | undefined {
        return this.#byValue().get(value);
    }

    /** Report whether an item shows: every parent above it expanded. */
    isShown(node: TreeNode): boolean {
        for (let parent = this.#parentOf(node); parent !== undefined;) {
            if (!parent.isExpanded()) {
                return false;
            }
            parent = this.#parentOf(parent);
        }

        return true;
    }

    /** Read the outermost collapsed parent above an item, else the item itself. */
    lift(value: string | undefined): string | undefined {
        // walk up from the item, stopping at each collapsed parent that hides it
        const node = value === undefined ? undefined : this.node(value);
        let stop = value;
        for (
            let parent = node === undefined ? undefined : this.#parentOf(node);
            parent !== undefined;
        ) {
            if (!parent.isExpanded()) {
                stop = parent.value;
            }
            parent = this.#parentOf(parent);
        }

        return stop;
    }

    /** Select an item and tell the change handler. */
    select(value: string): void {
        this.selection.select(value);
    }

    /** Read the parent of an item, undefined for a top-level one. */
    #parentOf(node: TreeNode): TreeNode | undefined {
        return node.parent === null ? undefined : this.node(node.parent);
    }
}

/** An item's place in its tree: its value, parent, expansion and element. */
interface TreeNode {
    /** The value the item stands for. */
    readonly value: string;
    /** The value of the parent item, null for a top-level item. */
    readonly parent: string | null;
    /** Whether the item's children show. */
    readonly isExpanded: Accessor<boolean>;
    /** The item's element, undefined until it renders. */
    readonly element: Accessor<HTMLElement | undefined>;
    /** Select the item and toggle a parent, as a click does. */
    readonly activate: () => void;
}

/** Read the default of a single selection, none when absent. */
function defaultOf(value: string | undefined): Pick<SingleSelection, "defaultValue"> {
    return value === undefined ? {} : { defaultValue: value };
}

/** The properties of a tree, the native list's attributes included. */
export interface TreeProperties extends Omit<
    JSX.HTMLAttributes<HTMLUListElement>,
    "class" | "onKeyDown"
> {
    /** The selected item's value, which makes the selection controlled. */
    readonly value?: string | undefined;
    /** The item selected at first when the selection is uncontrolled. */
    readonly defaultValue?: string;
    /** Handle another item being selected. */
    readonly onValueChange?: (value: string) => void;
    /** The StyleX styles applied after the tree's styles. */
    readonly xstyle?: style.Styles;
}

/** The properties of a tree item, the native list item's attributes included. */
export interface TreeItemProperties extends Omit<
    JSX.LiHTMLAttributes<HTMLLIElement>,
    "class" | "value" | "onClick" | "onFocus"
> {
    /** The value the item stands for. */
    readonly value: string;
    /** The text or content of the item's row. */
    readonly label: JSX.Element;
    /** Whether the item's children show, which makes the expansion controlled. */
    readonly expanded?: boolean;
    /** Whether the item's children show at first while uncontrolled. */
    readonly defaultExpanded?: boolean;
    /** Handle the person expanding or collapsing the item. */
    readonly onExpandedChange?: (expanded: boolean) => void;
    /** The StyleX styles applied after the row's styles. */
    readonly xstyle?: style.Styles;
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
    const rest = omit(properties, "value", "defaultValue", "onValueChange", "xstyle", "style");

    return (
        <TreeContext value={control}>
            <ul
                role="tree"
                data-slot="tree"
                {...rest}
                onKeyDown={(event) => navigate(event, control, locale.direction)}
                onFocusOut={(event) => control.focus.focusOut(event)}
                {...style.attributes(
                    [text.footnote, styles.tree, properties.xstyle],
                    properties.style,
                )}
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
    const rest = omit(
        properties,
        "value",
        "label",
        "expanded",
        "defaultExpanded",
        "onExpandedChange",
        "xstyle",
        "style",
        "children",
    );
    const [isExpanded, setExpanded] = createControllableSignal({
        isControlled: () => properties.expanded !== undefined,
        value: () => properties.expanded === true,
        defaultValue: properties.defaultExpanded === true,
        onChange: (expanded) => properties.onExpandedChange?.(expanded),
    });
    const isParent = (): boolean => "children" in properties;
    const isSelected = (): boolean => control.selection.isSelected(properties.value);
    const activate = (): void => {
        // select the item and toggle a parent
        control.select(properties.value);
        if (isParent()) {
            setExpanded(!isExpanded());
        }
    };
    let element: HTMLLIElement | undefined;
    control.nodes.add({
        value: properties.value,
        parent,
        isExpanded,
        element: () => element,
        activate,
    });
    control.focus.bind(
        () => properties.value,
        () => element,
    );

    // label the item by its own row
    const rowId = createUniqueId();

    return (
        <li
            role="treeitem"
            aria-labelledby={rowId}
            aria-level={level}
            aria-expanded={isParent() ? (isExpanded() ? "true" : "false") : undefined}
            aria-selected={isSelected() ? "true" : "false"}
            tabindex={control.focus.isActive(properties.value) ? 0 : -1}
            data-slot="tree-item"
            data-state={isParent() ? (isExpanded() ? "open" : "closed") : undefined}
            data-value={properties.value}
            {...rest}
            ref={(item) => (element = item)}
            onClick={(event) => {
                // select the clicked row's own item and toggle a parent
                event.stopPropagation();
                activate();
            }}
            onFocus={(event) => {
                // follow the focus onto the item's own element
                if (event.target === event.currentTarget) {
                    control.focus.focusIn(properties.value);
                }
            }}
            onKeyDown={(event) => {
                // expand, enter and collapse with the arrow keys along the reading direction
                if (event.target === event.currentTarget && isParent()) {
                    expandOrCollapse(
                        event,
                        control,
                        properties.value,
                        isExpanded(),
                        setExpanded,
                        locale.direction,
                    );
                }
            }}
            {...style.attrs(styles.item)}
        >
            <TreeItemRow
                id={rowId}
                level={level}
                isParent={isParent()}
                isExpanded={isExpanded()}
                isFocused={control.focus.isFocused() && control.focus.isActive(properties.value)}
                isSelected={isSelected()}
                xstyle={properties.xstyle}
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
    readonly xstyle: style.Styles | undefined;
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
            {...style.attributes([
                styles.row,
                styles.indent(properties.level),
                properties.isFocused && styles.focused,
                properties.isSelected && styles.selected,
                properties.xstyle,
            ])}
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
    event: KeyboardEvent,
    control: TreeControl,
    value: string,
    isExpanded: boolean,
    setExpanded: (isExpanded: boolean) => void,
    direction: Direction,
): void {
    // read the key that opens and the key that closes
    const opens = direction === "rtl" ? "ArrowLeft" : "ArrowRight";
    const closes = direction === "rtl" ? "ArrowRight" : "ArrowLeft";
    const child = control.delegate.after(value);

    // expand, enter or collapse, leaving other cases to the tree
    if (event.key === opens && !isExpanded) {
        event.preventDefault();
        event.stopPropagation();
        setExpanded(true);
    } else if (event.key === opens && isExpanded && child !== undefined) {
        event.preventDefault();
        event.stopPropagation();
        control.focus.focus(child);
    } else if (event.key === closes && isExpanded) {
        event.preventDefault();
        event.stopPropagation();
        setExpanded(false);
    }
}

/** Move the focus among a tree's visible items, to a parent, or choose the focused item with Enter and Space. */
function navigate(event: KeyboardEvent, control: TreeControl, direction: Direction): void {
    // read the focused item and the key that closes toward its parent
    const active = control.focus.current();
    const node = active === undefined ? undefined : control.node(active);
    const closes = direction === "rtl" ? "ArrowRight" : "ArrowLeft";

    // choose the focused item, step out to its parent, or move along the visible items
    if ((event.key === "Enter" || event.key === " ") && node !== undefined) {
        event.preventDefault();
        node.activate();
    } else if (event.key === closes && node?.parent !== undefined && node.parent !== null) {
        event.preventDefault();
        control.focus.focus(node.parent);
    } else {
        control.focus.move(event, direction);
    }
}
