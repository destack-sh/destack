import hexagonIcon from "@destack/icon/phosphor/hexagon";
import linkBreakIcon from "@destack/icon/phosphor/link-break";
import { t } from "@destack/locale";
import { type JsonValue, present } from "@destack/schema";
import {
    type Bindable,
    Color,
    Dimension,
    FontFamily,
    Reference,
    type TokenType,
} from "@destack/style";
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { TOKENS, type TokenEntry, type TokenGroup } from "@destack/theme";
import { color, radius, space, stroke } from "@destack/theme/tokens.stylex";
import { createMemo, createSignal, For, type JSX, onSettled, Show, useLocale } from "@destack/view";
import { Button } from "../button/index.ts";
import { InputGroup, InputGroupAddon } from "../input-group/index.ts";
import { groupMarker } from "../input-group/marker.stylex.ts";
import {
    Command,
    CommandEmpty,
    CommandGroup,
    CommandInput,
    CommandItem,
    CommandItemMeta,
    CommandList,
} from "../command/index.ts";
import { useFieldControl } from "../field/control.ts";
import { Icon } from "../icon/index.ts";
import { Popover, PopoverContent, PopoverTrigger } from "../popover/index.ts";
import { Binding } from "./binding.ts";
import { ColorPart } from "./color.tsx";
import { DimensionPart } from "./dimension.tsx";
import { FontFamilyPart } from "./font.tsx";
import { ValueSet, valueStyles } from "./group.tsx";
import { TokenProbe } from "./probe.ts";
import { type NamedValueInputProperties, ValueField } from "./value-field.tsx";

/** A token's button too narrow for its name and what it draws, which keeps its name. */
const NARROW_QUERY = "@container token (max-width: 9rem)";

/** The types of token whose drawn value fits beside its name, such as `16px`. */
const DRAWN_TYPES: ReadonlySet<TokenType> = new Set([
    "dimension",
    "fontWeight",
    "duration",
    "cubicBezier",
]);

/** The styles of a bindable value. */
const styles = style.create({
    path: {
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    drawn: {
        display: { default: "inline", [NARROW_QUERY]: "none" },
        marginInlineStart: "auto",
        minWidth: 0,
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
        color: color.mutedForeground,
    },
    swatch: (paint: string) => ({
        flexShrink: 0,
        width: space[4],
        height: space[4],
        borderRadius: radius[1],
        borderWidth: stroke.border,
        borderStyle: "solid",
        borderColor: color.border,
        backgroundColor: paint,
    }),
    menu: {
        width: "18rem",
        padding: 0,
    },
    bind: {
        // take no room until the field is pointed at or focused, where a pointer can point
        display: {
            default: null,
            [media.hover]: {
                default: "none",
                [style.when.ancestor(":hover", groupMarker)]: null,
                [style.when.ancestor(":focus-within", groupMarker)]: null,
                ":is([aria-expanded=true])": null,
            },
        },
    },
    pill: {
        flex: 1,
        minWidth: 0,
        containerType: "inline-size",
        containerName: "token",
        justifyContent: "start",
        gap: space[2],
        paddingInline: space[2],
    },
    over: {
        display: "grid",
        gridTemplateColumns: "minmax(0, 1fr)",
        gridTemplateAreas: '"field"',
        alignItems: "center",
    },
    field: {
        gridArea: "field",
        minWidth: 0,
    },
    slot: {
        gridArea: "field",
        justifySelf: "end",
        marginInlineEnd: space[6],
    },
});

/** The properties of a value bound to a token or set itself. */
export interface TokenBindingProperties<Value> {
    /** The type of token it binds to. */
    readonly type: TokenType;
    /** What the value is called. */
    readonly label: string | undefined;
    /** Whether it takes no input. */
    readonly disabled: boolean | undefined;
    /** The value, or a reference to its token. */
    readonly value: Bindable<Value>;
    /** Handle the value changing, or its binding. */
    readonly onValueChange: (value: Bindable<Value>) => void;
    /** Read the value a token draws now, which detaching it writes. */
    readonly detach: (probe: TokenProbe, reference: Reference) => Value;
    /** Write the value as CSS, so the menu finds the tokens that draw it, none to find none. */
    readonly css?: ((value: Value) => string) | undefined;
    /** Render the value's own input while it binds to no token, with the button to bind it. */
    readonly children: (value: () => Value | undefined, bind: JSX.Element) => JSX.Element;
    /** Lay out the bound token and the button to detach it, inside one field by default. */
    readonly bound?: ((token: JSX.Element, detach: JSX.Element) => JSX.Element) | undefined;
}

/** Render the input of a value or a reference to a token, as its schema takes them. */
export function BindableInput(properties: NamedValueInputProperties): JSX.Element {
    // read the token type, and the description of the value itself
    const binding = present(Binding.of(properties.type.schema), "a value or a token reference");
    const literal = { kind: "json", schema: binding.literal } as const;
    const value = (): unknown => properties.value;
    const name = (): string | undefined => properties.label ?? properties["aria-label"];

    // take the kit's own field of a length or a color, else the value's field beside a button to bind
    if (binding.type === "dimension") {
        return (
            <DimensionPart
                label={name()}
                disabled={properties.disabled}
                value={Dimension.bindable.parse(value() ?? Dimension.zero)}
                onValueChange={(next) => properties.onValueChange(next)}
            />
        );
    } else if (binding.type === "fontFamily") {
        return (
            <FontFamilyPart
                label={name()}
                disabled={properties.disabled}
                value={FontFamily.bindable.parse(value() ?? ["sans-serif"])}
                onValueChange={(next) => properties.onValueChange(next)}
            />
        );
    } else if (binding.type === "color") {
        return (
            <ColorPart
                label={name()}
                disabled={properties.disabled}
                value={Color.bindable.parse(value() ?? "#000000")}
                onValueChange={(next) => properties.onValueChange(next)}
            />
        );
    }

    // bind a value of several parts from its legend, as one pill while bound
    const isWhole = Binding.isWhole(binding.type);

    return (
        <TokenBinding<JsonValue | undefined>
            type={binding.type}
            label={name()}
            disabled={properties.disabled}
            value={properties.value}
            onValueChange={properties.onValueChange}
            detach={(probe, reference) => probe.json(binding.type, reference)}
            bound={
                !isWhole
                    ? undefined
                    : (token, unbind) => (
                          <ValueSet label={properties.label} aria-label={name()} actions={unbind}>
                              <InputGroup>{token}</InputGroup>
                          </ValueSet>
                      )
            }
        >
            {(own, bind) => {
                // offer to bind a value of several parts from its legend, else inside its input,
                //  the field made where it stands so it sits in the set around it
                const field = (): JSX.Element => (
                    <ValueField
                        type={literal}
                        aria-label={name()}
                        required
                        disabled={properties.disabled}
                        value={own()}
                        onValueChange={properties.onValueChange}
                    />
                );
                if (isWhole) {
                    return (
                        <ValueSet label={properties.label} aria-label={name()} actions={bind}>
                            {field()}
                        </ValueSet>
                    );
                }

                return (
                    <div {...style.attrs(styles.over, groupMarker)}>
                        <div {...style.attrs(styles.field)}>{field()}</div>
                        <span {...style.attrs(styles.slot)}>{bind}</span>
                    </div>
                );
            }}
        </TokenBinding>
    );
}

/** Render a value bound to a token as its token, else its own input, with a menu to bind it. */
export function TokenBinding<Value>(properties: TokenBindingProperties<Value>): JSX.Element {
    // tell a reference from the value itself
    const locale = useLocale();
    const [root, setRoot] = createSignal<HTMLElement>();
    let drawn: HTMLElement | undefined;
    onSettled(() => void setRoot(drawn));
    const reference = (): Reference | undefined =>
        Reference.is(properties.value) ? properties.value : undefined;
    const literal = (): Value | undefined =>
        Reference.is(properties.value) ? undefined : properties.value;

    // detach a reference into the value its token draws now
    const detach = (bound: Reference): void => {
        const element = present(root(), "a token binding's element");
        properties.onValueChange(properties.detach(new TokenProbe(element), bound));
    };

    // offer to bind the value, inside its own input
    const literalCss = (): string | undefined => {
        const own = literal();

        return own === undefined ? undefined : properties.css?.(own);
    };
    const bind = (
        <TokenMenu
            type={properties.type}
            label={properties.label}
            disabled={properties.disabled}
            value={undefined}
            literal={literalCss()}
            root={root()}
            onValueChange={properties.onValueChange}
        />
    );

    return (
        <div ref={(created) => (drawn = created)} {...style.attrs(valueStyles.group)}>
            {/* The token in place of the value, detached inside it, or the value itself */}
            <Show when={reference()} fallback={properties.children(literal, bind)}>
                {(bound) => {
                    // name the bound token, and offer to detach it
                    const token = (
                        <TokenMenu
                            type={properties.type}
                            label={properties.label}
                            disabled={properties.disabled}
                            value={bound()}
                            root={root()}
                            onValueChange={properties.onValueChange}
                        />
                    );
                    const unbind = (
                        <Button
                            variant="ghost"
                            size="sm"
                            isIconOnly
                            aria-label={locale.render(t.context("action")`Detach from token`)}
                            disabled={properties.disabled}
                            onClick={() => detach(bound())}
                        >
                            <Icon icon={linkBreakIcon} />
                        </Button>
                    );

                    return (
                        properties.bound?.(token, unbind) ?? (
                            <InputGroup>
                                {token}
                                <InputGroupAddon align="inline-end">{unbind}</InputGroupAddon>
                            </InputGroup>
                        )
                    );
                }}
            </Show>
        </div>
    );
}

/** Render the menu of the tokens of a type: as the bound token's button, or as a button to bind. */
function TokenMenu(properties: {
    /** The type of token it lists. */
    readonly type: TokenType;
    /** What the value is called. */
    readonly label: string | undefined;
    /** Whether it takes no input. */
    readonly disabled: boolean | undefined;
    /** The bound token, none while the value binds to none. */
    readonly value: Reference | undefined;
    /** The value as CSS while it binds to no token, whose matching tokens the menu lists first. */
    readonly literal?: string | undefined;
    /** The element the tokens are read at. */
    readonly root: HTMLElement | undefined;
    /** Take the token a person picks. */
    readonly onValueChange: (value: Reference) => void;
}): JSX.Element {
    // read what the bound token draws, the button labelled by its field
    const locale = useLocale();
    const field = useFieldControl();
    const [isOpen, setOpen] = createSignal(false);
    const bound = createMemo(() =>
        properties.value === undefined
            ? ""
            : (drawnOf(properties.type, [properties.value], properties.root)[0] ?? ""),
    );
    const pick = (reference: Reference): void => {
        properties.onValueChange(reference);
        setOpen(false);
    };

    return (
        <Popover open={isOpen()} onOpenChange={(next) => void setOpen(next)}>
            {/* The bound token, or a button to bind one */}
            <PopoverTrigger
                render={(attributes) => (
                    <Show
                        when={properties.value}
                        fallback={
                            <Button
                                {...attributes}
                                variant="ghost"
                                size="sm"
                                isIconOnly
                                aria-label={locale.render(t.context("action")`Bind to token`)}
                                disabled={properties.disabled}
                                xstyle={styles.bind}
                            >
                                <Icon icon={hexagonIcon} />
                            </Button>
                        }
                    >
                        {(token) => (
                            <Button
                                {...attributes}
                                variant="ghost"
                                xstyle={styles.pill}
                                disabled={properties.disabled}
                                data-slot="token"
                                id={field?.id ?? attributes.id}
                                aria-label={field === null ? properties.label : undefined}
                                aria-describedby={field?.attributes()["aria-describedby"]}
                            >
                                <TokenSwatch type={properties.type} value={token()} />
                                <span {...style.attrs(styles.path)}>
                                    {Reference.path(token()).join(".")}
                                </span>
                                <span {...style.attrs(styles.drawn)}>{bound()}</span>
                            </Button>
                        )}
                    </Show>
                )}
            />

            {/* The tokens of the type, searched */}
            <PopoverContent align="start" xstyle={styles.menu}>
                <TokenList
                    type={properties.type}
                    root={properties.root}
                    literal={properties.literal}
                    isShown={isOpen()}
                    onPick={pick}
                />
            </PopoverContent>
        </Popover>
    );
}

/** Render the tokens of a type by family, searched, each with what it draws. */
export function TokenList(properties: {
    /** The type of token it lists. */
    readonly type: TokenType;
    /** The element the tokens are read at. */
    readonly root: HTMLElement | undefined;
    /** A value as CSS, whose matching tokens it lists first, none to list none first. */
    readonly literal?: string | undefined;
    /** Whether it shows, which reads what the tokens draw. */
    readonly isShown: boolean;
    /** Take the token a person picks. */
    readonly onPick: (value: Reference) => void;
}): JSX.Element {
    // list the tokens of the type by family, and what each draws while it shows
    const locale = useLocale();
    const families = createMemo(() => TOKENS.filter(properties.type));
    const drawn = createMemo((): ReadonlyMap<Reference, string> => {
        const references = properties.isShown
            ? families().flatMap((family) =>
                  family.entries.map((entry) => Reference.to(entry.path)),
              )
            : [];
        const values = drawnOf(properties.type, references, properties.root);

        return new Map(references.map((reference, index) => [reference, values[index] ?? ""]));
    });

    // find the tokens that draw the value, each drawn here as the value is
    const matching = createMemo((): readonly TokenEntry[] => {
        // match only a value shown at an element
        const { literal, root } = properties;
        if (!properties.isShown || literal === undefined || root === undefined) {
            return [];
        }

        // draw the value and every token of the type here, then compare them
        const probe = new TokenProbe(root);
        const value = probe.literal(properties.type, literal);
        const references = families().flatMap((family) =>
            family.entries.map((entry) => Reference.to(entry.path)),
        );
        const css = probe.css(properties.type, references);
        const values = new Map(references.map((reference, index) => [reference, css[index] ?? ""]));

        return TOKENS.matching(properties.type, value, (reference) => values.get(reference) ?? "");
    });
    const groups = (): readonly TokenGroup[] => [
        ...(matching().length === 0
            ? []
            : [
                  {
                      name: locale.render(t.context("title")`Matching`),
                      description: "",
                      entries: matching(),
                  },
              ]),
        ...families(),
    ];

    return (
        <Command aria-label={locale.render(t.context("title")`Tokens`)}>
            <CommandInput placeholder={locale.render(t.context("label")`Search tokens…`)} />
            <CommandList>
                <CommandEmpty>{locale.render(t.context("title")`No tokens.`)}</CommandEmpty>
                <For each={groups()}>
                    {(family) => (
                        <CommandGroup heading={family.name}>
                            <For each={family.entries}>
                                {(entry) => {
                                    const reference = Reference.to(entry.path);

                                    return (
                                        <CommandItem
                                            value={
                                                family.entries === matching()
                                                    ? `${family.name} ${reference}`
                                                    : reference
                                            }
                                            keywords={[
                                                entry.path.join("."),
                                                entry.token.$description,
                                            ]}
                                            onSelect={() => properties.onPick(reference)}
                                        >
                                            <TokenSwatch type={properties.type} value={reference} />
                                            {entry.path.join(".")}
                                            <CommandItemMeta>
                                                {drawn().get(reference)}
                                            </CommandItemMeta>
                                        </CommandItem>
                                    );
                                }}
                            </For>
                        </CommandGroup>
                    )}
                </For>
            </CommandList>
        </Command>
    );
}

/** Read what short tokens draw at an element, such as `16px`, empty for the others. */
function drawnOf(
    type: TokenType,
    references: readonly Reference[],
    root: HTMLElement | undefined,
): readonly string[] {
    return root === undefined || !DRAWN_TYPES.has(type)
        ? references.map(() => "")
        : new TokenProbe(root).css(type, references);
}

/** Render a token's mark: its color for a color token, else the token mark. */
function TokenSwatch(properties: {
    /** The type of token. */
    readonly type: TokenType;
    /** The token. */
    readonly value: Reference;
}): JSX.Element {
    return (
        <Show when={properties.type === "color"} fallback={<Icon icon={hexagonIcon} />}>
            <span
                aria-hidden="true"
                {...style.attrs(styles.swatch(Reference.css(properties.value)))}
            />
        </Show>
    );
}
