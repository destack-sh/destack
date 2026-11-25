import { t } from "@destack/locale";
import type { Action } from "@destack/object";
import {
    createEffect,
    createSignal,
    handleOnPage,
    type JSX,
    palette,
    untrack,
    useActions,
    useLocale,
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
import { useCommandPlatform } from "../kbd/index.ts";
import type { PaletteGroup, PaletteItem } from "../palette/index.ts";
import { CommandPalette } from "../palette/index.ts";

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

    return (
        <CommandPalette
            id={properties.id}
            open={isOpen()}
            onOpenChange={(next) => void setOpen(next)}
            root={{
                id: "root",
                title: properties.title,
                groups: [
                    { id: "actions", title: locale.render(t`Actions`), items },
                    ...(properties.groups ?? []),
                ],
            }}
        />
    );
}
