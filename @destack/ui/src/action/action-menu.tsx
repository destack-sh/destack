import { t } from "@destack/locale";
import type { Action } from "@destack/object";
import {
    type ActionRegistry,
    createEffect,
    createMemo,
    createSignal,
    For,
    handleOnPage,
    isTextField,
    type JSX,
    onSettled,
    Show,
    untrack,
    useActions,
    useLocale,
} from "@destack/view";
import {
    moveItemsDown,
    moveItemsLeft,
    moveItemsRight,
    moveItemsUp,
    openItem,
    openMenu,
} from "./action.ts";
import { focusableAround } from "../focus/index.ts";
import { OpenChange } from "../layer/index.ts";
import type { Point } from "../position/index.ts";
import {
    Menu,
    MenuAction,
    MenuContent,
    MenuControl,
    MenuSeparator,
    MenuSubTrigger,
    useMenu,
} from "../menu/index.ts";

/** The actions that move items a step, which a menu lists in a submenu. */
const STEPS: ReadonlySet<Action> = new Set<Action>([
    moveItemsUp,
    moveItemsDown,
    moveItemsLeft,
    moveItemsRight,
]);

/** The place of the group of actions that move items a step. */
const STEP_GROUP = 3;

/** Render the actions on what the menu opened on: open, set, the rest, move, and remove. */
export function MenuActionList(): JSX.Element {
    // list those running as the menu opens, then as the rows of the objects it is on allow
    const actions = useActions();
    const control = useMenu();
    const locale = useLocale();
    const [read, setRead] = createSignal<readonly Action[] | undefined>(undefined, {
        ownedWrite: true,
    });
    const listed = createMemo<readonly Action[]>((last) => {
        const isOpen = control.root().isOpen();
        const rows = read();

        return isOpen ? (rows ?? actions.actionsOn(control.origin())) : (last ?? []);
    });
    createEffect(
        () => ({ isOpen: control.root().isOpen(), element: control.origin() }),
        ({ isOpen, element }) => {
            // read the rows each time it opens
            setRead(undefined);
            if (isOpen) {
                void untrack(() => actions.on(element)).then(setRead, reportError);
            }
        },
    );

    // group them, each group in the order its actions were declared
    const groups = createMemo(() => {
        // leave out the one that opens this menu
        const element = control.origin();
        const shown = listed().filter((action) => action !== openMenu);
        const byGroup = Map.groupBy(shown, (action) => groupOf(actions, action, element));

        return [...byGroup].toSorted(([one], [other]) => one - other);
    });

    return (
        <For each={groups()}>
            {([place, group], index) => (
                <>
                    <Show when={index() > 0 && place !== STEP_GROUP}>
                        <MenuSeparator />
                    </Show>
                    <Show
                        when={place === STEP_GROUP}
                        fallback={
                            <For each={group}>{(action) => <MenuAction action={action} />}</For>
                        }
                    >
                        <Menu>
                            <MenuSubTrigger>
                                {locale.render(t.context("action")`Move`)}
                            </MenuSubTrigger>
                            <MenuContent side="right" align="start">
                                <For each={group}>{(action) => <MenuAction action={action} />}</For>
                            </MenuContent>
                        </Menu>
                    </Show>
                </>
            )}
        </For>
    );
}

/** Read the place of an action's group in a menu. */
function groupOf(actions: ActionRegistry, action: Action, element: Element | null): number {
    // open first, and remove last, after moving
    if (action === openItem) {
        return 0;
    } else if (action.isDestructive) {
        return 4;
    } else if (STEPS.has(action)) {
        return STEP_GROUP;
    }

    // put what sets a property a person picks before the rest
    const [input, ...others] = untrack(() => actions.asked(action, element));
    const isPicked = input !== undefined && others.length === 0;

    return isPicked && untrack(() => actions.isListed(action, input, element)) ? 1 : 2;
}

/** Render one menu for the page: the actions on what a right click, long press or key is on. */
export function ActionMenu(): JSX.Element {
    // open at a point, on the nearest thing around
    const actions = useActions();
    const control = new MenuControl(null);
    let content: HTMLElement | undefined;
    const open = (target: Element, point: Point): boolean => {
        // leave the browser's menu where no action acts on anything but the page
        if (actions.subjectAt(target)?.part === undefined) {
            return false;
        }
        const element = focusableAround(target, target.ownerDocument.documentElement);
        const change = new OpenChange("trigger-press", undefined, { element, payload: undefined });
        control.open("first", change, point);

        return true;
    };

    // open it under what the person is on, by the key
    handleOnPage(openMenu, {
        run: ({ element }) => {
            const bounds = element?.getBoundingClientRect();
            if (element !== null && bounds !== undefined) {
                open(element, { x: bounds.left, y: bounds.bottom });
            }
        },
    });

    // mark what it is on while it is open
    createEffect(
        () => (control.isOpen() ? control.origin() : null),
        (origin) => {
            origin?.toggleAttribute("data-menu-open", true);

            return () => origin?.removeAttribute("data-menu-open");
        },
    );

    // open it as the pointer asks, unless a menu took it or the browser's serves the person
    const ask = (event: MouseEvent): void => {
        // leave a menu inside, a field's, and the one that copies selected text
        const { target } = event;
        const isLeft =
            event.defaultPrevented ||
            event.shiftKey ||
            !(target instanceof Element) ||
            isTextField(target) ||
            isInSelection(target);
        if (isLeft || !open(target, { x: event.clientX, y: event.clientY })) {
            return;
        }

        // take it from the browser, waiting for a pointer still down
        event.preventDefault();
        if (event.buttons !== 0) {
            control.release.await(event);
        }
    };
    onSettled(() => {
        // listen on the whole document until the menu goes
        const closing = new AbortController();
        content?.ownerDocument.addEventListener("contextmenu", ask, { signal: closing.signal });

        return () => closing.abort();
    });

    return (
        <Menu control={control}>
            <MenuContent ref={(element) => (content = element)} data-slot="action-menu">
                <MenuActionList />
            </MenuContent>
        </Menu>
    );
}

/** Report whether text a person selected is in an element, whose menu copies it. */
function isInSelection(element: Element): boolean {
    const selection = element.ownerDocument.getSelection();

    return selection !== null && !selection.isCollapsed && selection.containsNode(element, true);
}
