import { Icon } from "../icon/index.ts";
import { Toggle } from "../toggle/index.ts";
import { Button } from "../button/index.ts";
import caretDownIcon from "@destack/icon/phosphor/caret-down";
import lockIcon from "@destack/icon/phosphor/lock-simple";
import { t } from "@destack/locale";
import {
    type Bindable,
    Dimension,
    Reference,
    Size,
    type SizeLimit,
    SIZING_KINDS,
    Sizing,
    SizingKind,
} from "@destack/style";
import * as style from "@destack/style";
import { color, space } from "@destack/theme/tokens.stylex";
import { createSignal, For, type JSX, Show, useLocale } from "@destack/view";
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuCheckboxItem,
    DropdownMenuRadioGroup,
    DropdownMenuRadioItem,
    DropdownMenuSeparator,
    DropdownMenuTrigger,
} from "../dropdown-menu/index.ts";
import { FieldGroup } from "../field/field.tsx";
import { InputGroup, InputGroupAddon, InputGroupText } from "../input-group/index.ts";
import { DimensionPart } from "./dimension.tsx";
import { choiceType, ValuePart, ValueSet } from "./group.tsx";
import { type NamedValueInputProperties, readValue } from "./value-field.tsx";

/** No limit. */
const NO_LIMIT: SizeLimit = { kind: "none" };

/** The length a side takes as it turns fixed, as design tools start a frame. */
const START_LENGTH: Dimension = { value: 100, unit: "px" };

/** The two sides of a box, each with its mark, its name and its least and greatest lengths. */
const SIDES = {
    width: {
        mark: "W",
        title: t.context("label")`Width`,
        limits: [
            { key: "minWidth", mark: "Min", title: t.context("label")`Min width` },
            { key: "maxWidth", mark: "Max", title: t.context("label")`Max width` },
        ],
    },
    height: {
        mark: "H",
        title: t.context("label")`Height`,
        limits: [
            { key: "minHeight", mark: "Min", title: t.context("label")`Min height` },
            { key: "maxHeight", mark: "Max", title: t.context("label")`Max height` },
        ],
    },
} as const;

/** A side of a box. */
type Side = keyof typeof SIDES;

/** A least or greatest length of a side. */
type Limit = (typeof SIDES)[Side]["limits"][number];

/** A limit of a size, by its key. */
type LimitKey = Limit["key"];

/** The styles of a size's input. */
const styles = style.create({
    keyword: {
        flex: 1,
        minWidth: 0,
        color: color.mutedForeground,
    },
    mark: {
        minWidth: space[4],
        color: color.mutedForeground,
    },
});

/** Render the input of a size: width and height, each with its sizing in a menu, and limits. */
export function SizeInput(properties: NamedValueInputProperties): JSX.Element {
    // read the size, the one CSS starts from where none is given
    const locale = useLocale();
    const size = (): Size => readValue(properties.value, Size) ?? Size.initial;
    const change = (next: Partial<Size>): void => properties.onValueChange({ ...size(), ...next });
    const [isLocked, setLocked] = createSignal(false);

    // keep the ratio while locked, where both sides are fixed lengths in one unit
    const resize = (side: Side, sizing: Sizing): void => {
        // read the other side, and its ratio to this one before the change
        const other: Side = side === "width" ? "height" : "width";
        const opposite = size()[other];
        const ratio = ratioOf(size()[side], opposite);

        // scale the other side by the ratio while locked
        const scaled =
            isLocked() &&
            ratio !== undefined &&
            sizing.kind === "fixed" &&
            !Reference.is(sizing.length)
                ? {
                      kind: "fixed" as const,
                      length: { ...sizing.length, value: sizing.length.value * ratio },
                  }
                : opposite;
        change({ [side]: sizing, [other]: scaled });
    };

    return (
        <ValueSet
            label={properties.label}
            aria-label={properties["aria-label"]}
            actions={
                <Toggle
                    size="sm"
                    isIconOnly
                    aria-label={locale.render(t.context("label")`Keep ratio`)}
                    disabled={properties.disabled}
                    pressed={isLocked()}
                    onPressedChange={setLocked}
                >
                    <Icon icon={lockIcon} />
                </Toggle>
            }
        >
            {/* Width and height */}
            <FieldGroup columns={2}>
                <For each={["width", "height"] as const}>
                    {(side) => (
                        <SizingPart
                            side={side}
                            size={size()}
                            disabled={properties.disabled}
                            onSizingChange={(sizing) => resize(side, sizing)}
                            onLimitChange={(key, next) => change({ [key]: next })}
                        />
                    )}
                </For>
            </FieldGroup>

            {/* The limits set, least and greatest for each side */}
            <For each={["width", "height"] as const}>
                {(side) => (
                    <Show when={hasLimit(size(), side)}>
                        <FieldGroup columns={2}>
                            <For each={SIDES[side].limits}>
                                {(limit) => (
                                    <LimitPart
                                        limit={limit}
                                        size={size()}
                                        disabled={properties.disabled}
                                        onLimitChange={(key, next) => change({ [key]: next })}
                                    />
                                )}
                            </For>
                        </FieldGroup>
                    </Show>
                )}
            </For>
        </ValueSet>
    );
}

/** The kinds of one sizing, titled. */
const SIZING_TYPE = choiceType(SizingKind.options, SIZING_KINDS);

/** The length a sizing takes as it turns fixed. */
const FIXED_LENGTH: Dimension = { value: 100, unit: "px" };

/** Render the input of one sizing, such as a width: its kind, and its length where fixed. */
export function SizingInput(properties: NamedValueInputProperties): JSX.Element {
    // read the sizing, as the layout decides where none is given
    const locale = useLocale();
    const sizing = (): Sizing => readValue(properties.value, Sizing) ?? { kind: "auto" };
    const length = (): Bindable<Dimension> | undefined => {
        const current = sizing();

        return current.kind === "fixed" ? current.length : undefined;
    };
    const name = (): string | undefined => properties.label ?? properties["aria-label"];

    return (
        <ValueSet label={properties.label} aria-label={properties["aria-label"]}>
            <FieldGroup columns={2}>
                <ValuePart
                    type={SIZING_TYPE}
                    schema={SizingKind}
                    label={t.context("label")`Sizing`}
                    value={sizing().kind}
                    disabled={properties.disabled}
                    onValueChange={(kind) =>
                        properties.onValueChange(
                            kind === "fixed"
                                ? { kind, length: length() ?? FIXED_LENGTH }
                                : { kind },
                        )
                    }
                />
                <Show when={length()}>
                    {(fixed) => (
                        <DimensionPart
                            label={name() ?? locale.render(t.context("label")`Length`)}
                            disabled={properties.disabled}
                            value={fixed()}
                            onValueChange={(next) =>
                                properties.onValueChange({ kind: "fixed", length: next })
                            }
                        />
                    )}
                </Show>
            </FieldGroup>
        </ValueSet>
    );
}

/** Render one side's sizing: its length, or its kind, with a menu of kinds and limits inside. */
function SizingPart(properties: {
    /** The side. */
    readonly side: Side;
    /** The whole size. */
    readonly size: Size;
    /** Whether it takes no input. */
    readonly disabled: boolean | undefined;
    /** Handle the side's sizing changing. */
    readonly onSizingChange: (sizing: Sizing) => void;
    /** Handle a limit being added or removed. */
    readonly onLimitChange: (key: LimitKey, limit: SizeLimit) => void;
}): JSX.Element {
    // read the side's sizing, and name it in the person's language
    const locale = useLocale();
    const side = (): (typeof SIDES)[Side] => SIDES[properties.side];
    const sizing = (): Sizing => properties.size[properties.side];
    const length = (): Bindable<Dimension> | undefined => {
        const current = sizing();

        return current.kind === "fixed" ? current.length : undefined;
    };
    const label = (): string => locale.render(side().title);
    const menu = (
        <SizingMenu
            side={properties.side}
            size={properties.size}
            disabled={properties.disabled}
            onSizingChange={properties.onSizingChange}
            onLimitChange={properties.onLimitChange}
        />
    );

    return (
        <Show
            when={length()}
            fallback={
                <InputGroup aria-label={label()}>
                    <InputGroupAddon align="inline-start">
                        <span {...style.attrs(styles.mark)}>{side().mark}</span>
                    </InputGroupAddon>
                    <InputGroupText xstyle={styles.keyword}>
                        {locale.render(SIZING_KINDS[sizing().kind].title)}
                    </InputGroupText>
                    <InputGroupAddon align="inline-end">{menu}</InputGroupAddon>
                </InputGroup>
            }
        >
            {(fixed) => (
                <DimensionPart
                    label={label()}
                    start={side().mark}
                    end={menu}
                    disabled={properties.disabled}
                    value={fixed()}
                    onValueChange={(next) =>
                        properties.onSizingChange({ kind: "fixed", length: next })
                    }
                />
            )}
        </Show>
    );
}

/** Render the menu of a side's sizing: fixed, hug or fill, then its least and greatest lengths. */
function SizingMenu(properties: {
    /** The side. */
    readonly side: Side;
    /** The whole size. */
    readonly size: Size;
    /** Whether it takes no input. */
    readonly disabled: boolean | undefined;
    /** Handle the side's sizing changing. */
    readonly onSizingChange: (sizing: Sizing) => void;
    /** Handle a limit being added or removed. */
    readonly onLimitChange: (key: LimitKey, limit: SizeLimit) => void;
}): JSX.Element {
    // keep the length across a change of kind, starting a fixed side at a frame's length
    const locale = useLocale();
    const side = (): (typeof SIDES)[Side] => SIDES[properties.side];
    const sizing = (): Sizing => properties.size[properties.side];
    const choose = (key: string): void => {
        const kind = SizingKind.safeParse(key).data;
        const current = sizing();
        if (kind === "fixed") {
            properties.onSizingChange({
                kind,
                length: current.kind === "fixed" ? current.length : START_LENGTH,
            });
        } else if (kind !== undefined) {
            properties.onSizingChange({ kind });
        }
    };
    const toggle = (key: LimitKey): void =>
        properties.onLimitChange(
            key,
            properties.size[key].kind === "none"
                ? { kind: "length", length: START_LENGTH }
                : NO_LIMIT,
        );

    return (
        <DropdownMenu>
            <DropdownMenuTrigger
                render={(attributes) => (
                    <Button
                        {...attributes}
                        variant="ghost"
                        size="sm"
                        isIconOnly
                        aria-label={locale.render(
                            t.context("label")`${locale.render(side().title)} sizing`,
                        )}
                        disabled={properties.disabled}
                    >
                        <Icon icon={caretDownIcon} />
                    </Button>
                )}
            />
            <DropdownMenuContent align="end">
                <DropdownMenuRadioGroup value={sizing().kind} onValueChange={choose}>
                    <For each={SizingKind.options}>
                        {(kind) => (
                            <DropdownMenuRadioItem value={kind}>
                                {locale.render(SIZING_KINDS[kind].title)}
                            </DropdownMenuRadioItem>
                        )}
                    </For>
                </DropdownMenuRadioGroup>
                <DropdownMenuSeparator />
                <For each={side().limits}>
                    {(limit) => (
                        <DropdownMenuCheckboxItem
                            checked={properties.size[limit.key].kind === "length"}
                            onCheckedChange={() => toggle(limit.key)}
                        >
                            {locale.render(limit.title)}
                        </DropdownMenuCheckboxItem>
                    )}
                </For>
            </DropdownMenuContent>
        </DropdownMenu>
    );
}

/** Render one limit of a size: its length while set, else an empty field that sets none. */
function LimitPart(properties: {
    /** The limit: its key, mark and name. */
    readonly limit: Limit;
    /** The whole size. */
    readonly size: Size;
    /** Whether it takes no input. */
    readonly disabled: boolean | undefined;
    /** Handle the limit changing. */
    readonly onLimitChange: (key: LimitKey, limit: SizeLimit) => void;
}): JSX.Element {
    const locale = useLocale();
    const length = (): Bindable<Dimension> | undefined => {
        const set = properties.size[properties.limit.key];

        return set.kind === "length" ? set.length : undefined;
    };

    return (
        <Show when={length()} fallback={<span />}>
            {(set) => (
                <DimensionPart
                    label={locale.render(properties.limit.title)}
                    start={properties.limit.mark}
                    disabled={properties.disabled}
                    value={set()}
                    onValueChange={(next) =>
                        properties.onLimitChange(properties.limit.key, {
                            kind: "length",
                            length: next,
                        })
                    }
                />
            )}
        </Show>
    );
}

/** Tell whether a side has a least or greatest length set. */
function hasLimit(size: Size, side: Side): boolean {
    return SIDES[side].limits.some((limit) => size[limit.key].kind === "length");
}

/** Read the ratio of one side's length to the other's, where both are lengths in one unit. */
function ratioOf(side: Sizing, other: Sizing): number | undefined {
    // take two fixed lengths of a unit, neither bound to a token nor zero
    if (side.kind !== "fixed" || other.kind !== "fixed") {
        return undefined;
    } else if (Reference.is(side.length) || Reference.is(other.length)) {
        return undefined;
    } else if (side.length.unit !== other.length.unit || side.length.value === 0) {
        return undefined;
    }

    return other.length.value / side.length.value;
}
