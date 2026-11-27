import { type Direction, t } from "@destack/locale";
import type { Choice } from "@destack/object";
import { Mention } from "@destack/object/markdown";
import * as style from "@destack/style";
import { Subject } from "@destack/sync";
import { color, radius, shadow, size, space, stroke } from "@destack/theme/tokens.stylex";
import {
    type Accessor,
    createEffect,
    createSignal,
    For,
    type JSX,
    onCleanup,
    Show,
    untrack,
    useLocale,
} from "@destack/view";
import { ChoiceIcon } from "../action/choice-icon.tsx";
import { AutocompleteContext, AutocompleteControl } from "../autocomplete/index.ts";
import { ListBox, ListBoxControl, ListBoxEmpty, ListBoxItem } from "../list-box/index.ts";
import { Position } from "../position/index.ts";
import { type ComposerControl, useComposer } from "./composer.tsx";

/** The text before the caret that asks for a mention: an @ at a word's start, then its search. */
const ASKING = /(?:^|\s)@([^\s@]*)$/u;

/** The styles of a composer's mention list. */
const styles = style.create({
    content: {
        width: `calc(64 * ${space[1]})`,
        maxHeight: `calc(8 * ${size[3]})`,
        overflowY: "auto",
        inset: "auto",
        margin: 0,
        padding: space[1],
        borderWidth: stroke.border,
        borderStyle: "solid",
        borderColor: color.border,
        borderRadius: radius[4],
        backgroundColor: color.popover,
        color: color.foreground,
        boxShadow: shadow.overlay,
    },
    item: {
        display: "flex",
        alignItems: "center",
        gap: space[2],
    },
    description: {
        marginInlineStart: "auto",
        color: color.mutedForeground,
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
});

/** The properties of a composer's mention list. */
export interface ComposerMentionProperties {
    /** The people and objects the search found, each valued by its subject. */
    readonly choices: readonly Choice[];
    /** Find the choices of what a person types after an @, empty for a bare @. */
    readonly onSearch: (text: string) => void;
    /** Write the text a choice puts in place of its @ search, its Markdown mention by default. */
    readonly onMention?: (choice: Choice) => string;
}

/** The @ search of a composer's text and the list of people and objects it shows. */
export class ComposerMentionControl {
    /** The search after the @, which drives the list. */
    readonly autocomplete: AutocompleteControl;
    /** The list of choices. */
    readonly list: ListBoxControl;
    /** The search after the @ before the caret, undefined while none asks for a mention. */
    readonly search: Accessor<string | undefined>;
    /** The properties of the mention list. */
    readonly #properties: ComposerMentionProperties;
    /** The composer whose text it reads. */
    readonly #composer: ComposerControl;
    /** Replace the search. */
    readonly #setSearch: (search: string | undefined) => void;
    /** Where the @ of the asked mention stands in the text. */
    #start: number;
    /** Where the @ a person closed the list on stands, which stays closed until another. */
    #closed: number | undefined;
    /** The list element. */
    #content: HTMLElement | undefined;
    /** Stop placing the list beside the text. */
    #unplace: () => void;

    /** Follow the @ searches of a composer's text. */
    constructor(properties: ComposerMentionProperties, composer: ComposerControl) {
        // drive the list by the search, leaving the owner to filter its choices
        const [search, setSearch] = createSignal<string | undefined>(undefined);
        this.search = search;
        this.#setSearch = (next) => setSearch(() => next);
        this.autocomplete = new AutocompleteControl({
            get value() {
                return search() ?? "";
            },
            filter: null,
        });
        this.list = new ListBoxControl({ autocomplete: this.autocomplete });
        this.#properties = properties;
        this.#composer = composer;
        this.#start = 0;
        this.#closed = undefined;
        this.#content = undefined;
        this.#unplace = () => undefined;
    }

    /** Set the list element. */
    setContent(element: HTMLElement): void {
        this.#content = element;
    }

    /** Read the @ search before the caret, asking the owner for its choices. */
    read(input: HTMLTextAreaElement): void {
        // find an @ at a word's start before a collapsed caret
        const caret = input.selectionStart;
        const before = input.value.slice(0, caret);
        const asked = input.selectionEnd === caret ? ASKING.exec(before) : null;
        const search = asked?.[1];
        const start = caret - (search?.length ?? 0) - 1;

        // ask for the choices of a new search, unless the person closed the list on this @
        if (search === undefined || start === this.#closed) {
            this.#setSearch(undefined);
        } else {
            this.#closed = undefined;
            this.#start = start;
            if (search !== untrack(this.search)) {
                this.#setSearch(search);
                this.#properties.onSearch(search);
                this.list.focus.reset();
            }
        }
    }

    /** Close the list, keeping it closed for the @ it was open on. */
    close(): void {
        this.#closed = untrack(this.search) === undefined ? undefined : this.#start;
        this.#setSearch(undefined);
    }

    /** Steer the open list by a key pressed in the text, reporting whether the list took it. */
    press(event: KeyboardEvent, direction: Direction): boolean {
        // leave every key to the text while the list is closed
        if (untrack(this.search) === undefined || event.isComposing) {
            return false;
        }

        // close on Escape, choose on Tab as on Enter, and move through the list
        if (event.key === "Escape") {
            event.preventDefault();
            this.close();

            return true;
        } else if (event.key === "Tab" && !event.shiftKey) {
            const active = this.list.focus.current();
            if (active === undefined) {
                return false;
            }
            event.preventDefault();
            this.list.choose(active);

            return true;
        }

        return this.autocomplete.steer(event, direction);
    }

    /** Put a choice's mention in place of its @ search, and tell the text's owner. */
    insert(choice: Choice): void {
        // write the mention and a space after it, the caret after both
        const input = this.#composer.input;
        if (input === undefined) {
            return;
        }
        const written = this.#properties.onMention?.(choice) ?? markdownOf(choice);
        const caret = input.selectionStart;
        input.setRangeText(`${written} `, this.#start, caret, "end");

        // close the list and tell the owner of the text as typing does
        this.#setSearch(undefined);
        input.dispatchEvent(new InputEvent("input", { bubbles: true, inputType: "insertText" }));
        input.focus();
    }

    /** Show the list beside the text while a search asks for a mention, else hide it. */
    sync(isOpen: boolean): void {
        // leave a list or text not yet on screen
        const content = this.#content;
        const input = this.#composer.input;
        if (content === undefined || input === undefined) {
            return;
        }

        // attach the list to the text before showing it, and place it once it has a size
        this.#unplace();
        if (isOpen) {
            const detach = Position.attach(content, input, this.list.id);
            content.showPopover({ source: input });
            const unplace = Position.place(content, input);
            this.#unplace = () => {
                detach();
                unplace();
                this.#unplace = () => undefined;
            };
        } else {
            content.hidePopover();
        }
    }
}

/** Render the people and objects an @ in a composer's text finds, inserting a mention. */
export function ComposerMention(properties: ComposerMentionProperties): JSX.Element {
    // follow the composer's text and show the list while an @ asks
    const composer = useComposer();
    const locale = useLocale();
    const control = new ComposerMentionControl(properties, composer);
    composer.mention(control, () => locale.direction);
    createEffect(
        () => control.search() !== undefined,
        (isOpen) => control.sync(isOpen),
    );
    onCleanup(() => control.sync(false));

    return (
        <AutocompleteContext value={control.autocomplete}>
            <ListBox
                control={control.list}
                popover="manual"
                data-slot="composer-mention"
                aria-label={locale.render(t.context("label")`Mention`)}
                ref={(element: HTMLElement) => control.setContent(element)}
                xstyle={[...Position.beside("top", "start"), styles.content]}
            >
                <For each={properties.choices}>
                    {(choice, index) => (
                        <ListBoxItem
                            value={String(index())}
                            textValue={choice.title}
                            onSelect={() => control.insert(choice)}
                            xstyle={styles.item}
                        >
                            <ChoiceIcon choice={choice} />
                            {choice.title}
                            <Show when={choice.description}>
                                {(description) => (
                                    <span {...style.attrs(styles.description)}>
                                        {description()}
                                    </span>
                                )}
                            </Show>
                        </ListBoxItem>
                    )}
                </For>
                <ListBoxEmpty>{locale.render(t.context("title")`No results.`)}</ListBoxEmpty>
            </ListBox>
        </AutocompleteContext>
    );
}

/** Write a choice's Markdown mention: its title linked to its subject. */
function markdownOf(choice: Choice): string {
    return Mention.markdown(Subject.parse(choice.value), choice.title);
}
