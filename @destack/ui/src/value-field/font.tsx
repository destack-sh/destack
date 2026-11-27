import { t } from "@destack/locale";
import { Choice } from "@destack/object";
import {
    type Bindable,
    FONT_STYLES,
    FONT_WEIGHTS,
    FontFamily,
    FontFeatures,
    FontStyle,
    FontVariations,
    FontWeight,
} from "@destack/style";
import * as style from "@destack/style";
import { createMemo, createSignal, For, type JSX, Show, useLocale } from "@destack/view";
import caretIcon from "@destack/icon/phosphor/caret-down";
import { present } from "@destack/schema";
import { space } from "@destack/theme/tokens.stylex";
import { Button } from "../button/index.ts";
import {
    Command,
    CommandEmpty,
    CommandGroup,
    CommandInput,
    CommandItem,
    CommandList,
} from "../command/index.ts";
import { Icon } from "../icon/index.ts";
import { InputGroup, InputGroupAddon } from "../input-group/index.ts";
import { Popover, PopoverContent, PopoverTrigger } from "../popover/index.ts";
import { TokenBinding } from "./bindable.tsx";
import { choiceType } from "./group.tsx";
import { TagsParts } from "./tags.tsx";
import { TypefaceCatalog, useTypeface } from "./typeface.ts";
import { type NamedValueInputProperties, readValue, ValueField } from "./value-field.tsx";

/** The styles of the inputs of fonts. */
const styles = style.create({
    trigger: {
        flex: 1,
        minWidth: 0,
        justifyContent: "space-between",
        paddingInline: space[3],
    },
    name: {
        minWidth: 0,
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    menu: {
        width: "16rem",
        padding: 0,
    },
    face: (family: string) => ({
        fontFamily: family,
    }),
});

/** The fonts an input starts at, the browser's sans serif. */
const START_FAMILY: FontFamily = ["sans-serif"];

/** The weight an input starts at, a regular one. */
const START_WEIGHT: FontWeight = 400;

/** The type of a font weight, a choice its input lists as the family around draws. */
const WEIGHT_TYPE = { kind: "choice", choices: [] } as const;

/** The type of a font style, a choice of its values. */
const STYLE_TYPE = choiceType(FontStyle.options, FONT_STYLES);

/** Render the input of a font family: a picker of fonts, each in its own face. */
export function FontFamilyInput(properties: NamedValueInputProperties): JSX.Element {
    return (
        <FontPicker
            label={properties["aria-label"]}
            disabled={properties.disabled}
            value={readValue(properties.value, FontFamily) ?? START_FAMILY}
            onValueChange={(next) => properties.onValueChange(next)}
        />
    );
}

/** Render the fonts of a value, or a reference to a font family token in their place. */
export function FontFamilyPart(properties: {
    /** What the fonts are called. */
    readonly label: string | undefined;
    /** Whether it takes no input. */
    readonly disabled: boolean | undefined;
    /** The fonts, or their token. */
    readonly value: Bindable<FontFamily>;
    /** Handle the fonts or their token changing. */
    readonly onValueChange: (value: Bindable<FontFamily>) => void;
}): JSX.Element {
    return (
        <TokenBinding
            type="fontFamily"
            label={properties.label}
            disabled={properties.disabled}
            value={properties.value}
            onValueChange={properties.onValueChange}
            detach={(probe, reference) => probe.fontFamily(reference)}
            css={(families) => FontFamily.css(families)}
        >
            {(families, bind) => (
                <FontPicker
                    label={properties.label}
                    disabled={properties.disabled}
                    value={families() ?? START_FAMILY}
                    end={bind}
                    onValueChange={properties.onValueChange}
                />
            )}
        </TokenBinding>
    );
}

/** Render a picker of fonts: the first one in its own face, a searched list of fonts to pick. */
export function FontPicker(properties: {
    /** What the fonts are called. */
    readonly label: string | undefined;
    /** Whether it takes no input. */
    readonly disabled: boolean | undefined;
    /** The fonts, the first one first. */
    readonly value: FontFamily;
    /** What follows the name inside the field, such as a button to bind a token. */
    readonly end?: JSX.Element;
    /** Handle another font being picked. */
    readonly onValueChange: (value: FontFamily) => void;
}): JSX.Element {
    // pick a typeface, keeping the value's generic families after it, or the fonts a person types
    const locale = useLocale();
    const catalog = new TypefaceCatalog(document);
    const [isOpen, setOpen] = createSignal(false);
    const [typed, setTyped] = createSignal("");
    const first = (): string => present(properties.value[0], "a font family's first font");
    const pick = (fonts: FontFamily): void => {
        properties.onValueChange(fonts);
        setOpen(false);
    };
    const generic = new Set(catalog.generic().map((typeface) => typeface.family));
    const fallbacks = (family: string): FontFamily =>
        generic.has(family)
            ? [family]
            : [family, ...properties.value.filter((name) => generic.has(name))];

    // list the document's typefaces, the device's once asked, then the generic families
    const groups = createMemo(() =>
        isOpen()
            ? [
                  { title: t.context("title")`Document`, typefaces: catalog.document() },
                  { title: t.context("title")`Installed`, typefaces: catalog.local() },
                  { title: t.context("title")`Generic`, typefaces: catalog.generic() },
              ].filter((group) => group.typefaces.length > 0)
            : [],
    );
    const isNew = (): boolean => {
        const text = typed().trim().toLowerCase();

        return (
            text.length > 0 &&
            !groups().some((group) =>
                group.typefaces.some((typeface) => typeface.family.toLowerCase() === text),
            )
        );
    };

    return (
        <InputGroup>
            <Popover open={isOpen()} onOpenChange={(next) => void setOpen(next)}>
                <PopoverTrigger
                    render={(attributes) => (
                        <Button
                            {...attributes}
                            variant="ghost"
                            disabled={properties.disabled}
                            aria-label={properties.label}
                            xstyle={styles.trigger}
                        >
                            <span
                                {...style.attrs(
                                    styles.name,
                                    styles.face(FontFamily.css(properties.value)),
                                )}
                            >
                                {first()}
                            </span>
                            <Icon icon={caretIcon} />
                        </Button>
                    )}
                />
                <PopoverContent align="start" xstyle={styles.menu}>
                    <Command aria-label={locale.render(t.context("title")`Fonts`)}>
                        <CommandInput
                            placeholder={locale.render(t.context("label")`Search fonts…`)}
                            onValueChange={setTyped}
                        />
                        <CommandList>
                            <CommandEmpty>
                                {locale.render(t.context("title")`No fonts.`)}
                            </CommandEmpty>
                            <For each={groups()}>
                                {(group) => (
                                    <CommandGroup heading={locale.render(group.title)}>
                                        <For each={group.typefaces}>
                                            {(typeface) => (
                                                <CommandItem
                                                    value={typeface.family}
                                                    onSelect={() =>
                                                        pick(fallbacks(typeface.family))
                                                    }
                                                >
                                                    <span
                                                        {...style.attrs(
                                                            styles.face(
                                                                FontFamily.css([typeface.family]),
                                                            ),
                                                        )}
                                                    >
                                                        {typeface.family}
                                                    </span>
                                                </CommandItem>
                                            )}
                                        </For>
                                    </CommandGroup>
                                )}
                            </For>
                            <Show when={catalog.isLocalListed && catalog.local().length === 0}>
                                <CommandItem
                                    value={locale.render(t.context("action")`Show installed fonts`)}
                                    onSelect={() => void catalog.listLocal()}
                                >
                                    {locale.render(t.context("action")`Show installed fonts`)}
                                </CommandItem>
                            </Show>
                            <Show when={isNew()}>
                                <CommandItem
                                    value={typed()}
                                    onSelect={() => pick(FontFamily.read(typed()))}
                                >
                                    {locale.render(t.context("action")`Use ${typed().trim()}`)}
                                </CommandItem>
                            </Show>
                        </CommandList>
                    </Command>
                </PopoverContent>
            </Popover>
            <Show when={properties.end}>
                {(end) => <InputGroupAddon align="inline-end">{end()}</InputGroupAddon>}
            </Show>
        </InputGroup>
    );
}

/** Render the input of a font weight: the weights its family draws, and any other it is given. */
export function FontWeightInput(properties: NamedValueInputProperties): JSX.Element {
    // read the weight, a regular one where none is given
    const locale = useLocale();
    const weight = (): FontWeight => readValue(properties.value, FontWeight) ?? START_WEIGHT;
    const typeface = useTypeface();

    // list the weights the family around draws, else every named one, and the weight given
    const choices = (): Choice[] => {
        // name each weight the family draws
        const drawn = typeface()?.weights("normal");
        const named = Object.values(FONT_WEIGHTS)
            .filter((each) => drawn === undefined || drawn.includes(each.weight))
            .map((each) => Choice.of({ value: String(each.weight), title: each.title }, locale));

        // add the weight given where no name fits it
        const isNamed = named.some((each) => each.value === String(weight()));

        return isNamed ? named : [...named, Choice.of({ value: String(weight()) }, locale)];
    };
    const give = (key: unknown): void => {
        const next = FontWeight.safeParse(Number(key)).data;
        if (next !== undefined) {
            properties.onValueChange(next);
        }
    };

    return (
        <ValueField
            type={WEIGHT_TYPE}
            choices={choices()}
            aria-label={properties["aria-label"]}
            aria-describedby={properties["aria-describedby"]}
            required
            disabled={properties.disabled}
            value={String(weight())}
            onValueChange={give}
        />
    );
}

/** Render the input of a font style: upright, italic or oblique. */
export function FontStyleInput(properties: NamedValueInputProperties): JSX.Element {
    return <ValueField {...properties} type={STYLE_TYPE} required />;
}

/** Render the input of OpenType features: each feature's tag and setting. */
export function FontFeaturesInput(properties: NamedValueInputProperties): JSX.Element {
    // read the features, none where none are given
    const features = (): FontFeatures => readValue(properties.value, FontFeatures) ?? {};

    return (
        <TagsParts
            label={properties["aria-label"]}
            disabled={properties.disabled === true}
            value={features()}
            schema={FontFeatures}
            start={1}
            step={1}
            onValueChange={(next) => properties.onValueChange(next)}
        />
    );
}

/** Render the input of variable font axes: each axis's tag and value. */
export function FontVariationsInput(properties: NamedValueInputProperties): JSX.Element {
    // read the axes, none where none are given
    const variations = (): FontVariations => readValue(properties.value, FontVariations) ?? {};

    return (
        <TagsParts
            label={properties["aria-label"]}
            disabled={properties.disabled === true}
            value={variations()}
            schema={FontVariations}
            start={0}
            step={0.1}
            onValueChange={(next) => properties.onValueChange(next)}
        />
    );
}
