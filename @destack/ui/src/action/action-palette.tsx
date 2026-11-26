import { t } from "@destack/locale";
import type { Action, ObjectType } from "@destack/object";
import { ObjectReference } from "@destack/sync";
import {
    createEffect,
    createMemo,
    createSignal,
    handleOnPage,
    type JSX,
    palette,
    untrack,
    useActions,
    useLocale,
    useSearch,
} from "@destack/view";
import {
    clearSelection,
    moveItemsDown,
    moveItemsLeft,
    moveItemsRight,
    moveItemsUp,
    nextItem,
    previousItem,
    selectAll,
    toggleSelection,
} from "./action.ts";
import { ChoiceIcon } from "./choice-icon.tsx";
import { titleOf } from "./search-title.ts";
import { useCommandPlatform } from "../kbd/index.ts";
import type { PaletteGroup, PaletteItem, PalettePage } from "../palette/index.ts";
import { CommandPalette } from "../palette/index.ts";

/** The list of no entries, the same each time. */
const NONE: readonly PaletteItem[] = [];

/** The most objects it finds, as many as a person reads. */
const FOUND = 20;

/** The actions that step through a list, which people run by their keys. */
const STEPS: ReadonlySet<Action> = new Set<Action>([
    nextItem,
    previousItem,
    toggleSelection,
    selectAll,
    clearSelection,
    moveItemsUp,
    moveItemsDown,
    moveItemsLeft,
    moveItemsRight,
]);

/** The properties of a palette of actions. */
export interface ActionPaletteProperties {
    /** The key the palette remembers its entries' uses under, unique on the page. */
    readonly id: string;
    /** The title of the root page, such as the app's name. */
    readonly title: string;
    /** The objects it finds, live and as the person may see them, none by default. */
    readonly objects?: {
        /** Their types. */
        readonly types: readonly ObjectType[];
        /** Whether a text that finds nothing is corrected, as it is by default. */
        readonly typos?: "correct" | "keep";
        /** Open one a person picks. */
        readonly onOpen: (reference: ObjectReference) => void;
    };
    /** The groups after the actions, such as what a person opens. */
    readonly groups?: readonly PaletteGroup[];
}

/** Render the palette of what a person can do where they are, which its action opens. */
export function ActionPalette(properties: ActionPaletteProperties): JSX.Element {
    // open over where the person is
    const actions = useActions();
    const locale = useLocale();
    const isCommand = useCommandPlatform();
    const [place, setPlace] = createSignal<Element | null>(null, { ownedWrite: true });
    const [isOpen, setOpen] = createSignal(false, { ownedWrite: true });
    const [listed, setListed] = createSignal<readonly Action[]>([], { ownedWrite: true });
    handleOnPage(palette, {
        run: ({ element }) => {
            setPlace(element);
            setOpen(true);
        },
    });
    createEffect(
        () => ({ isShown: isOpen(), element: place() }),
        ({ isShown, element }) => {
            // read the rows of the objects there, whose methods it lists too
            if (isShown) {
                void untrack(() => actions.at(element)).then(setListed, reportError);
            }
        },
    );

    // list each with its keys, running it where the person was
    const itemOf = (action: Action): PaletteItem => {
        const shortcut = actions.keybindings(action, isCommand())[0];
        const title = locale.render(action.title);

        return {
            id: `${action.package.id}/${action.name}`,
            title,
            ...(action.category === undefined ? {} : { subtitle: locale.render(action.category) }),
            ...(shortcut === undefined ? {} : { shortcut }),
            actions: [{ id: "run", title, run: () => actions.start(action, place()) }],
        };
    };

    // put the steps of a list last
    const items = (): readonly PaletteItem[] =>
        listed()
            .filter((action) => action !== palette)
            .toSorted((one, two) => Number(STEPS.has(one)) - Number(STEPS.has(two)))
            .map((action) => itemOf(action));

    // find the objects while a person types, each opened where it is picked
    const [text, setText] = createSignal("", { ownedWrite: true });
    const found = useSearch(
        () =>
            isOpen() && properties.objects !== undefined && text().trim() !== ""
                ? text()
                : undefined,
        {
            get types() {
                return properties.objects?.types ?? [];
            },
            limit: FOUND,
            get typos() {
                return properties.objects?.typos ?? "correct";
            },
        },
    );
    const objects = createMemo((): readonly PaletteItem[] => {
        // keep one empty list while nothing is found, so the palette ranks nothing anew
        const choices = found.choices() ?? [];
        if (choices.length === 0) {
            return NONE;
        }

        return choices.map((choice) => ({
            id: JSON.stringify(choice.value),
            title: choice.title,
            ...(choice.excerpt === undefined ? {} : { subtitle: choice.excerpt }),
            icon: () => <ChoiceIcon choice={choice} />,
            actions: [
                {
                    id: "open",
                    title: locale.render(t`Open`),
                    run: () => properties.objects?.onOpen(ObjectReference.parse(choice.value)),
                },
            ],
        }));
    });

    // keep one page, its groups changing with the owner's, so a title alone ranks nothing anew
    const shared: readonly PaletteGroup[] = [
        { id: "actions", title: locale.render(t`Actions`), items },
        {
            id: "objects",
            get title() {
                return titleOf(found, locale) ?? locale.render(t`Results`);
            },
            items: objects,
            text: found.text,
            ranking: "owner",
            isPending: found.isPending,
        },
    ];
    const groups = createMemo(() => [...shared, ...(properties.groups ?? [])]);
    const root: PalettePage = {
        id: "root",
        get title() {
            return properties.title;
        },
        get groups() {
            return groups();
        },
    };

    return (
        <CommandPalette
            id={properties.id}
            open={isOpen()}
            onOpenChange={(next) => void setOpen(next)}
            onSearchChange={(typed) => void setText(typed)}
            root={root}
        />
    );
}
