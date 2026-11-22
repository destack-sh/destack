import type { Direction } from "@destack/locale";
import {
    type Accessor,
    createContext,
    createSignal,
    type JSX,
    omit,
    onCleanup,
    untrack,
    useContext,
    useLocale,
} from "@destack/view";
import type { Focus } from "../focus/index.ts";
import { Input, type InputProperties } from "../input/index.ts";

/** The keys a search field takes from its collection. */
const MOVES: ReadonlySet<string> = new Set([
    "ArrowDown",
    "ArrowUp",
    "ArrowLeft",
    "ArrowRight",
    "PageDown",
    "PageUp",
    "Home",
    "End",
]);

/** The search of the nearest autocomplete, null outside one. */
export const AutocompleteContext = createContext<AutocompleteControl | null>(null);

/** A collection a search field drives: its element, its focused key and the action of each key. */
export interface AutocompleteTarget {
    /** The id of the collection's element, which the search field controls. */
    readonly id: string;
    /** The focused key, virtual behind the search field's `aria-activedescendant`. */
    readonly focus: Focus<string>;
    /** Report whether the arrow keys move along the search field's text, as in a vertical list. */
    readonly isVertical: boolean;
    /** Run the action of a key's item, as Enter does. */
    readonly act: (key: string) => void;
}

/** The typed text of an autocomplete, on its root as Base UI's or on its field as cmdk's. */
export interface AutocompleteValueProperties {
    /** The typed text, which makes it controlled. */
    readonly value?: string | undefined;
    /** The typed text at first while uncontrolled, empty by default. */
    readonly defaultValue?: string | undefined;
    /** Handle each change of the typed text. */
    readonly onValueChange?: ((value: string) => void) | undefined;
}

/** The typed text of an autocomplete and how it filters. */
export interface AutocompleteProperties extends AutocompleteValueProperties {
    /** Report whether an item matches the search, null where the owner filters the items. */
    readonly filter?:
        | ((value: string, search: string, keywords: readonly string[]) => boolean)
        | null
        | undefined;
}

/** The attributes a search field takes from its autocomplete. */
export interface AutocompleteInputAttributes {
    /** The combobox role. */
    readonly role: "combobox";
    /** Whether the collection shows, always for an autocomplete. */
    readonly "aria-expanded": "true";
    /** The id of the collection's element. */
    readonly "aria-controls": string | undefined;
    /** The list autocomplete of a search that filters a collection. */
    readonly "aria-autocomplete": "list";
    /** The id of the focused item's element. */
    readonly "aria-activedescendant": string | undefined;
    /** No browser completion. */
    readonly autocomplete: "off";
    /** No spell checking. */
    readonly spellcheck: false;
    /** The search. */
    readonly value: string;
    /** Take what the person types as the search. */
    readonly onInput: (event: InputEvent & { readonly currentTarget: HTMLInputElement }) => void;
    /** Move the collection's focus and run its action from the keyboard. */
    readonly onKeyDown: (event: KeyboardEvent) => void;
}

/** The search field driving one collection's focused key. */
export class AutocompleteControl {
    /** The typed text, controlled or the autocomplete's own. */
    readonly value: Accessor<string>;
    /** The collection the search field drives, undefined until one mounts. */
    readonly target: Accessor<AutocompleteTarget | undefined>;
    /** The properties the autocomplete follows. */
    readonly #properties: AutocompleteProperties;
    /** Replace the typed text and tell the owner. */
    readonly #setValue: (value: string) => void;
    /** Replace the field whose properties set the typed text in place of the root's. */
    readonly #setField: (field: AutocompleteValueProperties | undefined) => void;
    /** Replace the collection the search field drives. */
    readonly #setTarget: (target: AutocompleteTarget | undefined) => void;

    /** Follow the search of an autocomplete's properties. */
    constructor(properties: AutocompleteProperties) {
        // follow the field's or the root's controlled text, else the autocomplete's own
        const [field, setField] = createSignal<AutocompleteValueProperties | undefined>(undefined, {
            ownedWrite: true,
        });
        const source = (): AutocompleteValueProperties => field() ?? properties;
        const initial = untrack(() => properties.defaultValue);
        const [typed, setTyped] = createSignal<string | undefined>(undefined);
        const value = (): string =>
            source().value ?? typed() ?? source().defaultValue ?? initial ?? "";
        const [target, setTarget] = createSignal<AutocompleteTarget | undefined>(undefined, {
            ownedWrite: true,
        });
        this.value = value;
        this.target = target;
        this.#properties = properties;
        this.#setValue = (next) => {
            // store an uncontrolled text, and tell the owner either way
            if (source().value === undefined) {
                setTyped(next);
            }
            source().onValueChange?.(next);
        };
        this.#setField = setField;
        this.#setTarget = setTarget;
    }

    /** Take the typed text from a field's properties until the field unmounts, as cmdk's input. */
    follow(field: AutocompleteValueProperties): void {
        this.#setField(field);
        onCleanup(() => this.#setField(undefined));
    }

    /** Drive a collection until it unmounts. */
    connect(target: AutocompleteTarget): void {
        this.#setTarget(target);
        onCleanup(() => this.#setTarget(undefined));
    }

    /** Replace the typed text, focusing the first match again. */
    type(value: string): void {
        this.#setValue(value);
        this.target()?.focus.reset();
    }

    /** Report whether an item matches the search. */
    matches(value: string, keywords: readonly string[]): boolean {
        // show every item without a search or filtering
        const search = this.value().trim();
        const filter = this.#properties.filter;
        if (filter === null || search === "") {
            return true;
        }

        // match by the owner's filter, else by the item's words
        if (filter !== undefined) {
            return filter(value, search, keywords);
        }
        const lowered = search.toLowerCase();

        return [value, ...keywords].some((word) => word.toLowerCase().includes(lowered));
    }

    /** Move the collection's focus on the keys its layout takes. */
    steer(event: KeyboardEvent, direction: Direction): boolean {
        // leave keys to the search field without a collection
        const target = this.target();
        if (target === undefined) {
            return false;
        }

        // run the focused item's action on Enter
        const active = target.focus.current();
        if (event.key === "Enter") {
            event.preventDefault();
            if (active !== undefined) {
                target.act(active);
            }

            return true;
        }
        // leave the caret's own keys to the text unless the collection moves through them
        else if (!MOVES.has(event.key) || (target.isVertical && isCaretKey(event.key))) {
            return false;
        }

        // move the focus, stopping the key from moving the caret
        event.preventDefault();
        target.focus.move(event, direction);

        return true;
    }

    /** Read the attributes of the search field that drives the collection. */
    input(direction: () => Direction): AutocompleteInputAttributes {
        const target = this.target;
        const value = this.value;

        return {
            role: "combobox",
            "aria-expanded": "true",
            get "aria-controls"() {
                return target()?.id;
            },
            "aria-autocomplete": "list",
            get "aria-activedescendant"() {
                return target()?.focus.descendant();
            },
            autocomplete: "off",
            spellcheck: false,
            get value() {
                return value();
            },
            onInput: (event) => this.type(event.currentTarget.value),
            onKeyDown: (event) => this.steer(event, direction()),
        };
    }
}

/** Read the nearest autocomplete, refusing parts outside one. */
export function useAutocomplete(): AutocompleteControl {
    const control = useContext(AutocompleteContext);
    if (control === null) {
        throw new TypeError("an autocomplete part needs an autocomplete around it");
    }

    return control;
}

/** Keep the search that drives the one collection inside. */
export function Autocomplete(
    properties: AutocompleteProperties & { readonly children?: JSX.Element },
): JSX.Element {
    return (
        <AutocompleteContext value={new AutocompleteControl(properties)}>
            {properties.children}
        </AutocompleteContext>
    );
}

/** The properties of an autocomplete's search field, the input's included. */
export type AutocompleteInputProperties = Omit<
    InputProperties,
    "value" | "onInput" | "onKeyDown" | "role"
>;

/** Render the search field of the nearest autocomplete. */
export function AutocompleteInput(properties: AutocompleteInputProperties): JSX.Element {
    // take the search field's attributes from the autocomplete
    const control = useAutocomplete();
    const locale = useLocale();
    const rest = omit(properties, "type");

    return (
        <Input
            type={properties.type ?? "search"}
            data-slot="autocomplete-input"
            {...control.input(() => locale.direction)}
            {...rest}
        />
    );
}

/** Report whether a key moves the caret along a line of text. */
function isCaretKey(key: string): boolean {
    return key === "ArrowLeft" || key === "ArrowRight";
}
