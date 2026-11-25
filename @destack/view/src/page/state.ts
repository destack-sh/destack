import type { Action, ActionInput, ActionInputs, Choice, ValuesOf } from "@destack/object";
import { type JsonValue, mapValues } from "@destack/schema";
import { type Accessor, createEffect, createMemo, createSignal, untrack } from "solid-js";
import { useActions } from "./action.ts";
import { useLocale } from "./locale.ts";

/** An action, and where it runs. */
export interface ActionPlace<Inputs extends ActionInputs = ActionInputs> {
    /** The action. */
    readonly action: Action<Inputs>;
    /** Where it runs. */
    readonly element: Element | null;
    /** The values given, the rest from where it runs. */
    readonly given?: Partial<ValuesOf<Inputs>> | undefined;
    /** Whether its choices and values are followed, as they are by default. */
    readonly isFollowed?: boolean;
}

/** What an action does at its place now, which buttons and menus draw. */
export interface ActionState {
    /** Report whether it runs. */
    readonly isEnabled: Accessor<boolean>;
    /** Report whether what it turns on is on, undefined for nothing. */
    readonly isOn: Accessor<boolean | undefined>;
    /** Read the one input left that a person picks from a list, undefined for another. */
    readonly input: Accessor<ActionInput | undefined>;
    /** List what a person picks the input from. */
    readonly choices: Accessor<readonly Choice[]>;
    /** List the values the input has: its handler's, else those its rows share. */
    readonly chosen: Accessor<readonly JsonValue[]>;
    /** Report whether a choice is one of those values. */
    readonly isChosen: (choice: Choice) => boolean;
    /** Run it as a person does, with the input's value where it is given one. */
    readonly start: (value?: JsonValue) => void;
}

/** Follow what an action does at a place, nothing while there is none. */
export function useAction<Inputs extends ActionInputs>(
    place: () => ActionPlace<Inputs> | undefined,
): ActionState {
    // read whether it runs, and whether what it turns on is on
    const actions = useActions();
    const locale = useLocale();
    const isEnabled = createMemo(() => {
        // run nothing without a place
        const own = place();

        return own !== undefined && actions.has(own.action.reference, own.element);
    });
    const isOn = createMemo(() => {
        // tell nothing of one that does not run
        const own = place();

        return own !== undefined && isEnabled() ? actions.isOn(own.action, own.element) : undefined;
    });

    // read the one input left
    const input = createMemo<ActionInput | undefined>(() => {
        // take the first asked for, where it is alone and has a list
        const own = place();
        const asked =
            own !== undefined && isEnabled()
                ? actions.asked(own.action, own.element, own.given)
                : [];
        const [first, ...others] = asked;
        const isListed =
            own !== undefined &&
            first !== undefined &&
            others.length === 0 &&
            actions.isListed(own.action, first, own.element);

        return isListed ? first : undefined;
    });

    // list its choices each time they are followed
    const followed = createMemo(() => {
        // follow none without an input
        const own = place();
        const asked = input();

        return own === undefined || asked === undefined || own.isFollowed === false
            ? undefined
            : { ...own, input: asked };
    });
    const [choices, setChoices] = createSignal<readonly Choice[]>([], { ownedWrite: true });
    createEffect(followed, (own) => {
        if (own !== undefined) {
            const listed = untrack(() =>
                actions.choices(own.action, own.input, own.element, locale),
            );
            void listed.then(setChoices, reportError);
        }
    });

    // follow what the handler tells, else the rows
    const handled = createMemo(() => {
        const own = followed();

        return own === undefined ? [] : actions.chosen(own.action, own.input, own.element);
    });
    const [stored, setStored] = createSignal<readonly JsonValue[]>([], { ownedWrite: true });
    createEffect(
        () => (handled() === undefined ? followed() : undefined),
        (own) => {
            // forget those of another place
            setStored([]);

            return own === undefined
                ? undefined
                : untrack(() => actions.follow(own.action, own.input, own.element, setStored));
        },
    );
    const chosen = () => handled() ?? stored();
    const keys = createMemo(() => new Set(chosen().map((value) => JSON.stringify(value))));

    return {
        isEnabled,
        isOn,
        input,
        choices,
        chosen,
        isChosen: (choice) => keys().has(JSON.stringify(choice.value)),
        start: (value) => {
            // give the input's value beside those given
            const own = place();
            const asked = input();
            const picked: Record<string, JsonValue> =
                value === undefined || asked === undefined ? {} : { [asked.name]: value };
            if (own !== undefined) {
                actions.start(own.action, own.element, {
                    ...own.given,
                    ...mapValues<Record<string, JsonValue>, Partial<ValuesOf<Inputs>>>(
                        picked,
                        (each) => each,
                    ),
                });
            }
        },
    };
}
