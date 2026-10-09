import * as style from "@destack/style";
import {
    color,
    motion,
    radius,
    shadow,
    size,
    space,
    stroke,
    weight,
} from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import {
    type Accessor,
    createContext,
    createControllableSignal,
    createUniqueId,
    type JSX,
    merge,
    omit,
    useContext,
    useLocale,
} from "@destack/view";
import {
    type ElementPartProperties,
    type PartAttributes,
    type Render,
    rendered,
    renderPart,
} from "../part/index.ts";
import { ListState, type Orientation } from "../focus/index.ts";

/** The orientation and activation of tabs that set neither. */
const DEFAULTS: Required<Pick<TabsProperties, "orientation" | "activationMode">> = {
    orientation: "horizontal",
    activationMode: "automatic",
};

/** The tabs of the nearest tabs root, null outside one. */
const TabsContext = createContext<TabsControl | null>(null);

/** The styles of tabs and their elements. */
const styles = style.create({
    tabs: {
        display: "flex",
        gap: space[2],
    },
    list: {
        display: "inline-flex",
        alignItems: "center",
        justifyContent: "center",
        width: "fit-content",
        padding: stroke.ring,
        borderRadius: radius[4],
        backgroundColor: color.muted,
        color: color.mutedForeground,
    },
    trigger: {
        display: "inline-flex",
        flex: 1,
        alignItems: "center",
        justifyContent: "center",
        gap: space[2],
        height: size[2],
        paddingInline: space[3],
        borderWidth: stroke.border,
        borderColor: "transparent",
        borderRadius: radius[3],
        fontWeight: weight.medium,
        whiteSpace: "nowrap",
        cursor: "pointer",
        transitionProperty: "color, background-color, box-shadow",
        transitionDuration: motion.durationShort,
        transitionTimingFunction: motion.easingStandard,
        outlineStyle: { default: "none", ":focus-visible": "solid" },
        outlineWidth: stroke.ring,
        outlineColor: `color-mix(in oklab, ${color.ring} 50%, transparent)`,
        opacity: { default: 1, ":disabled": 0.5 },
        pointerEvents: { default: "auto", ":disabled": "none" },
    },
    selected: {
        backgroundColor: color.background,
        color: color.foreground,
        boxShadow: shadow.raised,
    },
    content: {
        flex: 1,
        outlineStyle: { default: "none", ":focus-visible": "solid" },
        outlineWidth: stroke.ring,
        outlineColor: `color-mix(in oklab, ${color.ring} 50%, transparent)`,
    },
});

/** The direction of each orientation's list above or beside the panels. */
const orientations = style.create({
    horizontal: { flexDirection: "column" },
    vertical: { flexDirection: "row" },
});

/** The direction of each orientation's tabs within the list. */
const lists = style.create({
    horizontal: { flexDirection: "row" },
    vertical: { flexDirection: "column", alignItems: "stretch" },
});

/** Whether moving the focus to a tab selects it, or Enter and Space do. */
export type TabsActivationMode = "automatic" | "manual";

/** The selected tab, ids and keyboard model of a set of tabs, which its list, tabs and panels share. */
export class TabsControl {
    /** The properties of the tabs root, read for its controlled state and keyboard model. */
    readonly properties: Required<Pick<TabsProperties, "orientation" | "activationMode">> &
        TabsProperties;
    /** The selected value, controlled or the root's own. */
    readonly value: Accessor<string | undefined>;
    /** The tabs in document order and the one holding the tab stop. */
    readonly list: ListState;
    /** The prefix of every id of the tabs and panels. */
    readonly #prefix: string;
    /** Replace the selected value and tell the change handler. */
    readonly #setValue: (value: string | undefined) => void;

    /** Create the state of a set of tabs from its root's properties. */
    constructor(
        properties: Required<Pick<TabsProperties, "orientation" | "activationMode">> &
            TabsProperties,
    ) {
        // start from the default value with no tabs yet
        const [value, setValue] = createControllableSignal({
            isControlled: () => properties.value !== undefined,
            value: () => properties.value,
            defaultValue: properties.defaultValue,
            onChange: (next) => {
                // tell the change handler of each selected tab
                if (next !== undefined) {
                    properties.onValueChange?.(next);
                }
            },
        });
        this.properties = properties;
        this.value = value;
        this.list = new ListState({
            get orientation() {
                return properties.orientation;
            },
            isLooping: true,
            isTypeahead: false,
            initial: value,
        });
        this.#prefix = createUniqueId();
        this.#setValue = setValue;
    }

    /** Select a tab and tell the change handler. */
    select(value: string): void {
        this.#setValue(value);
    }

    /** Build the id of a tab. */
    tabId(value: string): string {
        return `${this.#prefix}-tab-${value.replaceAll(/\s/gu, "-")}`;
    }

    /** Build the id of a tab's panel. */
    panelId(value: string): string {
        return `${this.#prefix}-panel-${value.replaceAll(/\s/gu, "-")}`;
    }
}

/** The properties of a tabs root, the native element's attributes included. */
export interface TabsProperties extends Omit<JSX.HTMLAttributes<HTMLDivElement>, "class"> {
    /** The selected tab's value, which makes the selection controlled. */
    readonly value?: string;
    /** The tab selected at first when the selection is uncontrolled. */
    readonly defaultValue?: string;
    /** Handle another tab being selected. */
    readonly onValueChange?: (value: string) => void;
    /** The direction the tabs line up and the arrow keys move in, horizontal by default. */
    readonly orientation?: Exclude<Orientation, "both">;
    /** Whether focusing a tab selects it or Enter and Space do, automatic by default. */
    readonly activationMode?: TabsActivationMode;
    /** The StyleX styles applied after the root's styles. */
    readonly xstyle?: style.Styles;
}

/** The properties of an element of a set of tabs, the native element's attributes included. */
export type TabsElementProperties<Attributes> = Omit<Attributes, "class"> & ElementPartProperties;

/** The properties of a tab or a tab's panel. */
export type TabsValueProperties<Attributes> = TabsElementProperties<Omit<Attributes, "value">> & {
    /** The value the tab and its panel stand for. */
    readonly value: string;
};

/** Read the tabs of the nearest tabs root, refusing elements outside one. */
export function useTabs(): TabsControl {
    const control = useContext(TabsContext);
    if (control === null) {
        throw new TypeError("tab elements need a tabs root around them");
    }

    return control;
}

/** Hold the selected tab of a list of tabs and their panels. */
export function Tabs(properties: TabsProperties): JSX.Element {
    // share one set of tabs with the list, tabs and panels
    const tabs = merge(DEFAULTS, properties);
    const rest = omit(
        tabs,
        "value",
        "defaultValue",
        "onValueChange",
        "orientation",
        "activationMode",
        "xstyle",
        "style",
    );
    const control = new TabsControl(tabs);

    return (
        <TabsContext value={control}>
            <div
                data-slot="tabs"
                data-orientation={tabs.orientation}
                {...rest}
                {...style.attributes(
                    [styles.tabs, orientations[tabs.orientation], tabs.xstyle],
                    tabs.style,
                )}
            />
        </TabsContext>
    );
}

/** Render the tab list, which arrow keys, Home and End move through. */
export function TabsList(
    properties: TabsElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    // move the focus among the list's tabs, selecting them when activation is automatic
    const control = useTabs();
    const locale = useLocale();
    const move = (event: KeyboardEvent) => {
        // move among the list's tabs, selecting the one moved to when activation is automatic
        const target = control.list.focus.move(event, locale.direction);
        if (target !== undefined && control.properties.activationMode === "automatic") {
            control.select(target);
        }
    };

    return renderPart(
        "div",
        "tabs-list",
        properties,
        () => [styles.list, lists[control.properties.orientation]],
        {
            role: "tablist",
            get "data-orientation"() {
                return control.properties.orientation;
            },
            get "aria-orientation"() {
                return control.properties.orientation;
            },
            onKeyDown: move,
            onFocusOut: (event) => control.list.focus.focusOut(event),
        },
    );
}

/** The properties of a tab, the native button's attributes included. */
export type TabsTriggerProperties = TabsValueProperties<
    Omit<JSX.ButtonHTMLAttributes<HTMLButtonElement>, "onClick">
> & {
    /** Render another element with the tab's attributes, the native button by default. */
    readonly render?: Render;
};

/** Render a tab that selects its panel on click, or on focus when activation is automatic. */
export function TabsTrigger(properties: TabsTriggerProperties): JSX.Element {
    // join the tabs and read whether this one is selected
    const control = useTabs();
    let element: HTMLElement | undefined;
    control.list.add({
        key: properties.value,
        text: () => properties.value,
        isDisabled: () => properties.disabled === true,
        element: () => element,
    });
    const rest = omit(properties, "value", "xstyle", "style", "render");
    const isSelected = (): boolean => control.value() === properties.value;
    const part: PartAttributes = merge(
        {
            role: "tab" as const,
            get id() {
                return control.tabId(properties.value);
            },
            get "aria-controls"() {
                return control.panelId(properties.value);
            },
            get "aria-selected"() {
                return isSelected() ? "true" : "false";
            },
            get tabindex() {
                return control.list.focus.isActive(properties.value) ? 0 : -1;
            },
            ref: (target: HTMLElement) => {
                element = target;
            },
            "data-slot": "tabs-trigger",
            get "data-state"() {
                return isSelected() ? "active" : "inactive";
            },
            get "data-value"() {
                return properties.value;
            },
            get "data-orientation"() {
                return control.properties.orientation;
            },
            onClick: () => control.select(properties.value),
            onFocus: () => control.list.focus.focusIn(properties.value),
        },
        () =>
            style.attributes(
                [text.footnote, styles.trigger, isSelected() && styles.selected, properties.xstyle],
                properties.style,
            ),
    );

    return rendered(properties.render, part, rest, () => (
        <button type="button" {...part} {...rest} />
    ));
}

/** Render the panel of a tab, hidden while another tab is selected. */
export function TabsContent(
    properties: TabsValueProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    const control = useTabs();
    const rest = omit(properties, "value", "xstyle", "style");

    return (
        <div
            role="tabpanel"
            id={control.panelId(properties.value)}
            aria-labelledby={control.tabId(properties.value)}
            tabindex={0}
            hidden={control.value() !== properties.value}
            data-slot="tabs-content"
            data-state={control.value() === properties.value ? "active" : "inactive"}
            data-orientation={control.properties.orientation}
            {...rest}
            {...style.attributes([styles.content, properties.xstyle], properties.style)}
        />
    );
}
