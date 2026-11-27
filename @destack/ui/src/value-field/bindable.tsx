import hexagonIcon from "@destack/icon/phosphor/hexagon";
import linkBreakIcon from "@destack/icon/phosphor/link-break";
import { t } from "@destack/locale";
import { type JsonValue, present } from "@destack/schema";
import { type Bindable, Reference, type TokenType } from "@destack/style";
import * as style from "@destack/style";
import { TOKENS } from "@destack/theme";
import { color, radius, space, stroke } from "@destack/theme/tokens.stylex";
import { createMemo, createSignal, For, type JSX, onSettled, Show, useLocale } from "@destack/view";
import { Button } from "../button/index.ts";
import {
    Command,
    CommandEmpty,
    CommandGroup,
    CommandInput,
    CommandItem,
    CommandItemMeta,
    CommandList,
} from "../command/index.ts";
import { Icon } from "../icon/index.ts";
import { Popover, PopoverContent, PopoverTrigger } from "../popover/index.ts";
import { Binding } from "./binding.ts";
import { ValueGroup, valueStyles } from "./group.tsx";
import { TokenProbe } from "./probe.ts";
import { type NamedValueFieldProperties, ValueField } from "./value-field.tsx";

/** The types of token whose drawn value fits beside its name, such as `16px`. */
const DRAWN_TYPES: ReadonlySet<TokenType> = new Set([
    "dimension",
    "fontWeight",
    "duration",
    "cubicBezier",
]);

/** The styles of a bindable value. */
const styles = style.create({
    token: {
        justifyContent: "start",
        minWidth: 0,
    },
    path: {
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    drawn: {
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
        background: paint,
    }),
    menu: {
        width: "18rem",
        padding: 0,
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
    /** Render the input of the value itself, while it binds to no token. */
    readonly children: (value: () => Value | undefined) => JSX.Element;
}

/** Render the input of a value or a reference to a token, as its schema takes them. */
export function BindableInput(properties: NamedValueFieldProperties): JSX.Element {
    // read the token type, and the description of the value itself
    const binding = present(Binding.of(properties.type.schema), "a value or a token reference");
    const literal = { kind: "json", schema: binding.literal } as const;

    return (
        <TokenBinding<JsonValue | undefined>
            type={binding.type}
            label={properties["aria-label"]}
            disabled={properties.disabled}
            value={properties.value}
            onValueChange={properties.onValueChange}
            detach={(probe, reference) => probe.json(binding.type, reference)}
        >
            {(own) => (
                <ValueField
                    type={literal}
                    aria-label={properties["aria-label"]}
                    required
                    disabled={properties.disabled}
                    value={own()}
                    onValueChange={properties.onValueChange}
                />
            )}
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

    return (
        <ValueGroup aria-label={properties.label}>
            <div ref={(created) => (drawn = created)} {...style.attrs(valueStyles.item)}>
                {/* The token, or the value itself */}
                <Show when={reference()} fallback={properties.children(literal)}>
                    {(bound) => (
                        <TokenMenu
                            type={properties.type}
                            label={properties.label}
                            disabled={properties.disabled}
                            value={bound()}
                            root={root()}
                            onValueChange={properties.onValueChange}
                        />
                    )}
                </Show>

                {/* Bind, or detach */}
                <Show
                    when={reference()}
                    fallback={
                        <TokenMenu
                            type={properties.type}
                            label={properties.label}
                            disabled={properties.disabled}
                            value={undefined}
                            root={root()}
                            onValueChange={properties.onValueChange}
                        />
                    }
                >
                    {(bound) => (
                        <Button
                            variant="ghost"
                            size="sm"
                            isIconOnly
                            disabled={properties.disabled}
                            aria-label={locale.render(t.context("action")`Detach from token`)}
                            onClick={() => detach(bound())}
                        >
                            <Icon icon={linkBreakIcon} />
                        </Button>
                    )}
                </Show>
            </div>
        </ValueGroup>
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
    /** The element the tokens are read at. */
    readonly root: HTMLElement | undefined;
    /** Take the token a person picks. */
    readonly onValueChange: (value: Reference) => void;
}): JSX.Element {
    // list the tokens of the type, by family
    const locale = useLocale();
    const [isOpen, setOpen] = createSignal(false);
    const families = createMemo(() =>
        TOKENS.families
            .map((family) => ({
                ...family,
                entries: family.entries.filter((entry) => entry.token.$type === properties.type),
            }))
            .filter((family) => family.entries.length > 0),
    );

    // read what each short token draws while the menu is open, and the bound one's
    const read = (references: readonly Reference[]): readonly string[] => {
        const element = properties.root;

        return element === undefined || !DRAWN_TYPES.has(properties.type)
            ? references.map(() => "")
            : new TokenProbe(element).css(properties.type, references);
    };
    const drawn = createMemo((): ReadonlyMap<Reference, string> => {
        const references = isOpen()
            ? families().flatMap((family) =>
                  family.entries.map((entry) => Reference.to(entry.path)),
              )
            : [];
        const values = read(references);

        return new Map(references.map((reference, index) => [reference, values[index] ?? ""]));
    });
    const bound = createMemo(() =>
        properties.value === undefined ? "" : (read([properties.value])[0] ?? ""),
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
                                disabled={properties.disabled}
                                aria-label={locale.render(t.context("action")`Bind to token`)}
                            >
                                <Icon icon={hexagonIcon} />
                            </Button>
                        }
                    >
                        {(token) => (
                            <Button
                                {...attributes}
                                variant="outline"
                                xstyle={styles.token}
                                disabled={properties.disabled}
                                data-slot="token"
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
                <Command aria-label={locale.render(t.context("title")`Tokens`)}>
                    <CommandInput placeholder={locale.render(t.context("label")`Search tokens…`)} />
                    <CommandList>
                        <CommandEmpty>{locale.render(t.context("title")`No tokens.`)}</CommandEmpty>
                        <For each={families()}>
                            {(family) => (
                                <CommandGroup heading={family.name}>
                                    <For each={family.entries}>
                                        {(entry) => {
                                            const reference = Reference.to(entry.path);

                                            return (
                                                <CommandItem
                                                    value={reference}
                                                    keywords={[
                                                        entry.path.join("."),
                                                        entry.token.$description,
                                                    ]}
                                                    onSelect={() => pick(reference)}
                                                >
                                                    <TokenSwatch
                                                        type={properties.type}
                                                        value={reference}
                                                    />
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
            </PopoverContent>
        </Popover>
    );
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
