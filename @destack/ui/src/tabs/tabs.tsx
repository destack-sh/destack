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
import { useLocale } from "@destack/locale/solid";
import { text } from "@destack/theme/text";
import type { JSX } from "@solidjs/web";
import {
    createContext,
    createSignal,
    createUniqueId,
    merge,
    omit,
    onCleanup,
    useContext,
    type Accessor,
    type Setter,
} from "solid-js";
import { directionOf, itemsOf, moveFocus, type Orientation } from "../focus/index.ts";

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
        borderStyle: "solid",
        borderWidth: stroke.border,
        borderColor: "transparent",
        borderRadius: radius[3],
        backgroundColor: "transparent",
        color: "inherit",
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
    /** The values of the tabs in document order. */
    readonly values: Accessor<readonly string[]>;
    /** The prefix of every id of the tabs and panels. */
    readonly #prefix: string;
    /** The selected value when uncontrolled. */
    readonly #ownValue: Accessor<string | undefined>;
    /** Replace the selected value when uncontrolled. */
    readonly #setValue: Setter<string | undefined>;
    /** Replace the values of the tabs. */
    readonly #setValues: Setter<readonly string[]>;

    /** Create the state of a set of tabs from its root's properties. */
    constructor(
        properties: Required<Pick<TabsProperties, "orientation" | "activationMode">> &
            TabsProperties,
    ) {
        // start from the default value with no tabs yet
        const [ownValue, setValue] = createSignal(properties.defaultValue);
        const [values, setValues] = createSignal<readonly string[]>([], { ownedWrite: true });
        this.properties = properties;
        this.values = values;
        this.#prefix = createUniqueId();
        this.#ownValue = ownValue;
        this.#setValue = setValue;
        this.#setValues = setValues;
    }

    /** Read the selected value, controlled or the root's own. */
    value(): string | undefined {
        return this.properties.value ?? this.#ownValue();
    }

    /** Add a tab's value until the tab unmounts. */
    register(value: string): void {
        this.#setValues((values) => [...values, value]);
        onCleanup(() => this.#setValues((values) => values.filter((entry) => entry !== value)));
    }

    /** Select a tab and tell the change handler. */
    select(value: string): void {
        this.#setValue(value);
        this.properties.onValueChange?.(value);
    }

    /** Report whether a tab takes the tab stop: the selected one, else the first. */
    isTabStop(value: string): boolean {
        return (this.value() ?? this.values()[0]) === value;
    }

    /** Name the id of a tab. */
    tabId(value: string): string {
        return `${this.#prefix}-tab-${value.replaceAll(/\s/gu, "-")}`;
    }

    /** Name the id of a tab's panel. */
    panelId(value: string): string {
        return `${this.#prefix}-panel-${value.replaceAll(/\s/gu, "-")}`;
    }
}

/** The properties of a tabs root, the native element's attributes included. */
export interface TabsProperties extends Omit<
    JSX.HTMLAttributes<HTMLDivElement>,
    "class" | "style"
> {
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
    readonly style?: style.Styles;
}

/** The properties of an element of a set of tabs, the native element's attributes included. */
export type TabsElementProperties<Attributes> = Omit<Attributes, "class" | "style"> & {
    /** The StyleX styles applied after the element's styles. */
    readonly style?: style.Styles;
};

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
        "style",
    );
    const control = new TabsControl(tabs);

    return (
        <TabsContext value={control}>
            <div
                data-slot="tabs"
                data-orientation={tabs.orientation}
                {...rest}
                {...style.attrs(styles.tabs, orientations[tabs.orientation], tabs.style)}
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
    const rest = omit(properties, "style");
    const move = (event: KeyboardEvent & { readonly currentTarget: HTMLDivElement }) => {
        // move among the list's tabs
        const tabs = itemsOf(event.currentTarget, "[role=tab]");
        const target = moveFocus(
            event,
            tabs,
            control.properties.orientation,
            directionOf(locale.tag),
        );
        const value = target?.dataset["value"];
        if (value !== undefined && control.properties.activationMode === "automatic") {
            control.select(value);
        }
    };

    return (
        <div
            role="tablist"
            data-slot="tabs-list"
            aria-orientation={control.properties.orientation}
            {...rest}
            onKeyDown={move}
            {...style.attrs(styles.list, lists[control.properties.orientation], properties.style)}
        />
    );
}

/** Render a tab that selects its panel on click, or on focus when activation is automatic. */
export function TabsTrigger(
    properties: TabsValueProperties<Omit<JSX.ButtonHTMLAttributes<HTMLButtonElement>, "onClick">>,
): JSX.Element {
    // join the tabs and read whether this one is selected
    const control = useTabs();
    control.register(properties.value);
    const rest = omit(properties, "value", "style");
    const isSelected = (): boolean => control.value() === properties.value;

    return (
        <button
            type="button"
            role="tab"
            id={control.tabId(properties.value)}
            aria-controls={control.panelId(properties.value)}
            aria-selected={isSelected() ? "true" : "false"}
            tabindex={control.isTabStop(properties.value) ? 0 : -1}
            data-slot="tabs-trigger"
            data-state={isSelected() ? "active" : "inactive"}
            data-value={properties.value}
            {...rest}
            onClick={() => control.select(properties.value)}
            {...style.attrs(
                text.footnote,
                styles.trigger,
                isSelected() && styles.selected,
                properties.style,
            )}
        />
    );
}

/** Render the panel of a tab, hidden while another tab is selected. */
export function TabsContent(
    properties: TabsValueProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    const control = useTabs();
    const rest = omit(properties, "value", "style");

    return (
        <div
            role="tabpanel"
            id={control.panelId(properties.value)}
            aria-labelledby={control.tabId(properties.value)}
            tabindex={0}
            hidden={control.value() !== properties.value}
            data-slot="tabs-content"
            {...rest}
            {...style.attrs(styles.content, properties.style)}
        />
    );
}
