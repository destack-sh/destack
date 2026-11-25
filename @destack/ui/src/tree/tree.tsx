import { state } from "@destack/theme/state";
import type { ObjectReference } from "@destack/sync";
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { color, radius, space, stroke, size } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import {
    type Accessor,
    type ActionRegistry,
    createContext,
    createControllableSignal,
    createMemo,
    createUniqueId,
    type JSX,
    omit,
    Show,
    useActions,
    useContext,
    useLocale,
} from "@destack/view";
import type { Direction } from "@destack/locale";
import { provideItems } from "../action/index.ts";
import { useCommandPlatform } from "../kbd/index.ts";
import { Collection, CollectionBuilder } from "../collection/index.ts";
import { DisclosureCaret, indent, indicator } from "../disclosure/index.ts";
import { Focus, ListDelegate } from "../focus/index.ts";
import { contentOr, type ElementPartProperties, renderPart, type Render } from "../part/index.ts";
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
        outlineStyle: "none",
    },
    item: {
        outlineStyle: "none",
    },
    row: {
        position: "relative",
        display: "flex",
        alignItems: "center",
        minHeight: size[2],
        gap: space[1],
        paddingBlock: space[1],
        paddingInlineEnd: space[2],
        borderRadius: radius[2],
        cursor: "default",
        userSelect: "none",
    },
    // keep the fill under the menu open on the item
    hover: {
        backgroundColor: {
            default: "transparent",
            ":is([data-menu-open] > *)": color.hover,
            ":hover": { default: null, [media.hover]: color.hover },
        },
    },
    focused: {
        outlineStyle: "solid",
        outlineWidth: stroke.ring,
        outlineOffset: `calc(-1 * ${stroke.ring})`,
        outlineColor: color.ring,
    },
    toggle: {
        justifyContent: "center",
        width: space[4],
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
    /** The focused item, which has the tab stop. */
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

        // list the items outside collapsed parents, found by value
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

        // move through the visible items, the selected one or its outermost collapsed ancestor with the tab stop
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

    /** List the item a person is on at an element, none outside one. */
    targets(element: Element | null): readonly string[] {
        const item = element?.closest<HTMLElement>("[role=treeitem][data-value]") ?? null;
        const value = item?.dataset["value"];

        return value === undefined || this.node(value) === undefined ? [] : [value];
    }

    /** Select an item and tell the change handler. */
    select(value: string): void {
        this.selection.add(value);
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
export interface TreeProperties extends Omit<JSX.HTMLAttributes<HTMLUListElement>, "class"> {
    /** The selected item's value, which makes the selection controlled. */
    readonly value?: string | undefined;
    /** The item selected at first when the selection is uncontrolled. */
    readonly defaultValue?: string;
    /** Handle another item being selected. */
    readonly onValueChange?: (value: string) => void;
    /** Read the object an item stands for, which actions on its type take, absent for none. */
    readonly reference?: (value: string) => ObjectReference | undefined;
    /** The StyleX styles applied after the tree's styles. */
    readonly xstyle?: style.Styles;
    /** Render another element with its attributes, its own by default. */
    readonly render?: Render;
}

/** The properties of a tree item, the native list item's attributes included. */
export interface TreeItemProperties extends Omit<
    JSX.LiHTMLAttributes<HTMLLIElement>,
    "class" | "value"
> {
    /** The value the item stands for. */
    readonly value: string;
    /** The label of a standard row, whose items are the children, parts of its own when absent. */
    readonly label?: JSX.Element;
    /** Whether the item's children show, which makes the expansion controlled. */
    readonly expanded?: boolean;
    /** Whether the item's children show at first while uncontrolled. */
    readonly defaultExpanded?: boolean;
    /** Handle the person expanding or collapsing the item. */
    readonly onExpandedChange?: (expanded: boolean) => void;
    /** The StyleX styles applied after a standard row's styles. */
    readonly xstyle?: style.Styles;
    /** Render another element with its attributes, its own by default. */
    readonly render?: Render;
}

/** Read the tree of the nearest tree, refusing items outside one. */
export function useTree(): TreeControl {
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
    const actions = useActions();
    const isCommand = useCommandPlatform();
    const rest = omit(properties, "value", "defaultValue", "onValueChange", "reference");

    return (
        <TreeContext value={control}>
            {renderPart("ul", "tree", rest, [text.sm, styles.tree], {
                role: "tree" as const,
                ref: provideItems({
                    targets: (element) => control.targets(element),
                    reference: (value) => properties.reference?.(value),
                }),
                onKeyDown: (event) =>
                    navigate(event, control, locale.direction, actions, isCommand()),
                onFocusOut: (event) => control.focus.focusOut(event),
            })}
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
    const groups = new CollectionBuilder<() => HTMLElement | undefined>((group) => group());
    const isParent = (): boolean =>
        properties.label === undefined ? groups.items().length > 0 : "children" in properties;
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

    // share the item with its row and group, which labels it and which makes it a parent
    const rowId = createUniqueId();
    const item: TreeItemControl = {
        value: properties.value,
        rowId,
        level,
        isParent,
        isExpanded,
        isSelected,
        isFocused: () => control.focus.isFocused() && control.focus.isActive(properties.value),
        addGroup: (group) => void groups.add(group),
    };

    return renderPart("li", "tree-item", rest, styles.item, {
        role: "treeitem" as const,
        "aria-labelledby": rowId,
        "aria-level": level,
        get "aria-expanded"() {
            return isParent() ? (isExpanded() ? "true" : "false") : undefined;
        },
        get "aria-selected"() {
            return isSelected() ? "true" : "false";
        },
        get tabindex() {
            return control.focus.isActive(properties.value) ? 0 : -1;
        },
        get "data-open"() {
            return isParent() && isExpanded() ? "" : undefined;
        },
        get "data-closed"() {
            return isParent() && !isExpanded() ? "" : undefined;
        },
        get "data-value"() {
            return properties.value;
        },
        ref: (created) => (element = created),
        onClick: (event) => {
            // select the clicked row's own item and toggle a parent
            event.stopPropagation();
            activate();
        },
        onFocus: (event) => {
            // follow the focus onto the item's own element
            if (event.target === event.currentTarget) {
                control.focus.focusIn(properties.value);
            }
        },
        onKeyDown: (event) => {
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
        },
        get children() {
            return (
                <TreeItemContext value={item}>
                    {properties.label === undefined ? (
                        properties.children
                    ) : (
                        <>
                            <TreeItemRow xstyle={properties.xstyle}>
                                <TreeItemToggle />
                                {properties.label}
                            </TreeItemRow>
                            <Show when={"children" in properties}>
                                <TreeItemGroup>{properties.children}</TreeItemGroup>
                            </Show>
                        </>
                    )}
                </TreeItemContext>
            );
        },
    });
}

/** An item of a tree as its parts read it. */
export interface TreeItemControl {
    /** The value the item stands for. */
    readonly value: string;
    /** The id of its row, which labels it. */
    readonly rowId: string;
    /** The item's depth, 1 at the top level. */
    readonly level: number;
    /** Whether items are under it. */
    readonly isParent: Accessor<boolean>;
    /** Whether the items under it show. */
    readonly isExpanded: Accessor<boolean>;
    /** Whether it is the one selected. */
    readonly isSelected: Accessor<boolean>;
    /** Whether it has the focus. */
    readonly isFocused: Accessor<boolean>;
    /** Add the group of the items under it, until it unmounts. */
    readonly addGroup: (element: () => HTMLElement | undefined) => void;
}

/** The nearest item of a tree, null outside one. */
const TreeItemContext = createContext<TreeItemControl | null>(null);

/** Read the nearest item of a tree, refusing a part outside one. */
export function useTreeItem(): TreeItemControl {
    const item = useContext(TreeItemContext);
    if (item === null) {
        throw new TypeError("a tree item part needs a tree item around it");
    }

    return item;
}

/** The properties of a part of an item's row, the native element's attributes included. */
export type TreeItemElementProperties<Target extends HTMLElement = HTMLSpanElement> = Omit<
    JSX.HTMLAttributes<Target>,
    "class"
> &
    ElementPartProperties;

/** Render the row of an item: its guide, toggle, and what it stands for. */
export function TreeItemRow(properties: TreeItemElementProperties<HTMLDivElement>): JSX.Element {
    const { rowId, level, isFocused, isSelected } = useTreeItem();

    return renderPart(
        "div",
        "tree-item-row",
        properties,
        () => [
            styles.row,
            styles.hover,
            indent.by("0px", level - 1),
            isFocused() && styles.focused,
            isSelected() && state.selected,
        ],
        { id: rowId },
    );
}

/** Render the items under an item, one level deeper, while it is expanded. */
export function TreeItemGroup(
    properties: TreeItemElementProperties<HTMLUListElement>,
): JSX.Element {
    // make the item a parent
    const { value, level, isExpanded, addGroup } = useTreeItem();
    const rest = omit(properties, "children");
    let element: HTMLElement | undefined;
    addGroup(() => element);

    return renderPart("ul", "tree-group", rest, null, {
        ref: (created: HTMLElement) => void (element = created),
        role: "group" as const,
        get hidden() {
            return !isExpanded();
        },
        get children() {
            return (
                <TreeLevelContext value={level + 1}>
                    <TreeParentContext value={value}>{properties.children}</TreeParentContext>
                </TreeLevelContext>
            );
        },
    });
}

/** Render the caret that turns as a parent expands, or the room of one. */
export function TreeItemToggle(properties: TreeItemElementProperties): JSX.Element {
    // turn with the item's expansion
    const { isParent, isExpanded } = useTreeItem();
    const rest = omit(properties, "children");

    return renderPart(
        "span",
        "tree-item-toggle",
        rest,
        () => [styles.toggle, indicator.base, isParent() && isExpanded() && indicator.open],
        {
            "aria-hidden": "true" as const,
            get children() {
                return (
                    <Show when={isParent()}>
                        {contentOr(properties, () => (
                            <DisclosureCaret />
                        ))}
                    </Show>
                );
            },
        },
    );
}

/** Render the lines of the items an item is under. */
export function TreeItemGuide(properties: TreeItemElementProperties): JSX.Element {
    const { level } = useTreeItem();

    return (
        <Show when={level > 1}>
            {renderPart(
                "span",
                "tree-item-guide",
                properties,
                [indent.guide, indent.guideAt("0px", level - 1)],
                { "aria-hidden": "true" as const },
            )}
        </Show>
    );
}

/** Expand or enter a parent, or collapse an open one. */
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

/** Move the focus among a tree's visible items, or to a parent. */
function navigate(
    event: KeyboardEvent,
    control: TreeControl,
    direction: Direction,
    actions: ActionRegistry,
    isCommand: boolean,
): void {
    // read the focused item and the key that closes toward its parent
    const active = control.focus.current();
    const node = active === undefined ? undefined : control.node(active);
    const closes = direction === "rtl" ? "ArrowRight" : "ArrowLeft";

    // choose the focused item, step out to its parent, run an action, or move along the visible items
    if (event.defaultPrevented) {
        return;
    } else if ((event.key === "Enter" || event.key === " ") && node !== undefined) {
        event.preventDefault();
        node.activate();
    } else if (event.key === closes && node?.parent !== undefined && node.parent !== null) {
        event.preventDefault();
        control.focus.focus(node.parent);
    } else if (!actions.press(event, isCommand)) {
        control.focus.move(event, direction);
    }
}
