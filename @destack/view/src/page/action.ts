import { isServer } from "@solidjs/web";
import { DeclarationReference } from "@destack/package/declare";
import { type Accessor, createSignal, onCleanup, type Setter, useContext } from "solid-js";
import { ObjectReference } from "@destack/sync";
import { type Localization, Shortcut, t } from "@destack/locale";
import {
    type Action,
    type ActionCall,
    ActionInput,
    type ActionInputs,
    type ActionResolution,
    type ActionValues,
    Choice,
    defineAction,
    type ObjectType,
    Place,
    type PlaceVariable,
    selected,
    type ValuesOf,
} from "@destack/object";
import type { ObjectClient } from "@destack/object/client";
import { FamilyError } from "@destack/error";
import { JsonValue, mapValues, schema } from "@destack/schema";
import { ActionRegistryContext } from "./context.ts";

/** The roles of widgets whose keys type text, after ARIA. */
const FIELD_ROLES: ReadonlySet<string> = new Set(["textbox", "searchbox", "combobox"]);

/** The elements Enter and Space activate. */
const PRESSED = "button, a[href], summary";

/** The most objects an input's choices list. */
const CHOICES = 200;

/** The elements that run actions where the person was before they reached them. */
const RUNNERS = "[data-action], [role=menu], [role=dialog]";

/** Undo the person's newest change across the page's scopes. */
export const undo = defineAction({
    name: "undo",
    title: t`Undo`,
    category: t`Edit`,
    keybinding: "mod+z",
});

/** Redo the person's newest undone change across the page's scopes. */
export const redo = defineAction({
    name: "redo",
    title: t`Redo`,
    category: t`Edit`,
    keybinding: "mod+shift+z",
});

/** Open the command palette over the page, from anywhere in it. */
export const palette = defineAction({
    name: "palette",
    title: t`Command Palette`,
    keybinding: "mod+k",
    focus: "any",
});

/** One run of a handled action: where the person was, and its inputs' values. */
export interface ActionEvent<Inputs extends ActionInputs = ActionInputs> {
    /** The element the person was at, null for the page itself. */
    readonly element: Element | null;
    /** The inputs' values, by name. */
    readonly values: ValuesOf<Inputs>;
}

/** What runs an action, and while when. */
export interface ActionHandler<Inputs extends ActionInputs = ActionInputs> {
    /** Run the action where the person was, with its inputs' values. */
    run(event: ActionEvent<Inputs>): Promise<void> | void;
    /** List the values a person picks an input from where they are, such as a board's columns. */
    readonly choices?: (input: string, element: Element | null) => readonly Choice[];
    /** Report whether the handler runs where the person is, read reactively, as menus validate. */
    readonly isEnabled?: (element: Element | null) => boolean;
    /** Report whether what it turns on is on, absent for what turns nothing on. */
    readonly isOn?: (element: Element | null) => boolean;
}

/** A handler with the element it handles actions in, absent for the whole page. */
interface Handling {
    /** The handler. */
    readonly handler: ActionHandler;
    /** The element, which actions in it bubble to. */
    readonly element: Element | undefined;
}

/** A place with the element it is of, absent for the whole page. */
interface Provision {
    /** Read the place now. */
    readonly place: Accessor<Place>;
    /** The element, whose actions find the place. */
    readonly element: Element | undefined;
}

/** What a page calls methods with: its clients, and the space it is in. */
export interface ActionHost {
    /** The clients of the scopes the page opens, by scope. */
    readonly clients: ReadonlyMap<string, ObjectClient>;
    /** The space the page is in. */
    readonly scope: string;
}

/** What a person is asked before an action runs: the inputs left, and to confirm it. */
export interface ActionQuestion {
    /** The action. */
    readonly action: Action;
    /** The values so far. */
    readonly values: ActionValues;
    /** The inputs the person still gives. */
    readonly asked: readonly ActionInput[];
    /** Where the person was. */
    readonly element: Element | null;
}

/** Ask a person before an action runs, answering what they give, absent once they cancel. */
export type ActionAsk = (question: ActionQuestion) => Promise<ActionValues | undefined>;

/** Tell a person an action failed, with the reason its error declares, absent for none. */
export type ActionFail = (action: Action, reason: string | undefined) => void;

/** The actions a running page's components handle, and the objects the person selects in it. */
export class ActionRegistry {
    /** The handled actions, by setting key. */
    readonly #actions = new Map<string, Action>();
    /** The handlers of each action, by setting key, the latest last. */
    readonly #handlers = new Map<string, Handling[]>();
    /** The places of the page's parts, the latest last. */
    #provisions: readonly Provision[] = [];
    /** Read the objects the person selected in the page, absent while no component tracks them. */
    #selection: (() => readonly ObjectReference[]) | undefined;
    /** The actions calling methods that the page offers, by setting key. */
    readonly #offered = new Map<string, Action>();
    /** What the page calls methods with, absent on a page without clients. */
    #host: ActionHost | undefined;
    /** Where the person last was, null for the page itself. */
    readonly #last: Accessor<Element | null>;
    /** Replace where the person last was. */
    readonly #setLast: Setter<Element | null>;
    /** The parts of the page asking the person, the latest last. */
    #asks: readonly ActionAsk[] = [];
    /** The parts of the page telling the person of failures, the latest last. */
    #fails: readonly ActionFail[] = [];
    /** Read the person's key combinations by setting key, null for an unbound one. */
    readonly #keybindings: Accessor<Readonly<Record<string, Shortcut | null>>>;
    /** Count the changes to the handlers, which readers of the actions follow. */
    readonly #revision: Accessor<number>;
    /** Replace the count of changes. */
    readonly #setRevision: Setter<number>;

    /** Create a page's registry without handlers, its keys the person's, else the declared ones. */
    constructor(keybindings: Accessor<Readonly<Record<string, Shortcut | null>>> = () => ({})) {
        // follow the changes to the handlers
        this.#keybindings = keybindings;
        const [revision, setRevision] = createSignal(0, { ownedWrite: true });
        this.#revision = revision;
        this.#setRevision = setRevision;

        // follow where the person is
        const [last, setLast] = createSignal<Element | null>(null, { ownedWrite: true });
        this.#last = last;
        this.#setLast = setLast;
    }

    /** Handle an action in an element, else the whole page, and return what stops handling it. */
    handle<Inputs extends ActionInputs>(
        action: Action<Inputs>,
        handler: ActionHandler<Inputs>,
        element?: Element,
    ): () => void {
        // add the handler to the action's, its values read by the action's inputs
        const key = DeclarationReference.key(action.reference);
        const stored: Handling = {
            element,
            handler: {
                ...handler,
                run: ({ element: origin, values }) =>
                    handler.run({
                        element: origin,
                        values: mapValues<ActionValues, ValuesOf<Inputs>>(values, (value) => value),
                    }),
            },
        };
        this.#actions.set(key, action);
        this.#handlers.set(key, [...(this.#handlers.get(key) ?? []), stored]);
        this.#changed();

        return () => {
            const left = (this.#handlers.get(key) ?? []).filter((each) => each !== stored);
            if (left.length === 0) {
                this.#actions.delete(key);
                this.#handlers.delete(key);
            } else {
                this.#handlers.set(key, left);
            }
            this.#changed();
        };
    }

    /** Report whether an action runs now: an enabled handler of it applies to the focus. */
    has(reference: DeclarationReference, focus: Element | null): boolean {
        const action = this.find(reference);

        return action !== undefined && this.actions(focus).includes(action);
    }

    /** List the actions that run for a focus, the innermost first, then those on what is around. */
    actions(focus: Element | null): Action[] {
        // put an element's handled actions before those of the elements around it, the page's last
        const handled = [...this.#actions].flatMap(([key, action]) => {
            const chosen = this.#chosen(key, focus);

            return chosen === undefined ? [] : [{ action, depth: depthOf(chosen.element) }];
        });
        const ordered = handled.toSorted((one, other) => other.depth - one.depth);

        // add the calls of methods on objects around, and those on the page's space
        const places = this.places(focus);
        const offered = [...this.#offered.values()].filter(
            ({ call }) =>
                call !== undefined &&
                this.#client(call) !== undefined &&
                call.inputs.every(
                    (input) => input.name !== call.target || input.shown(places) !== undefined,
                ),
        );

        return [...ordered.map(({ action }) => action), ...offered];
    }

    /** List every action at an element, each titled method of the objects around too. */
    async at(element: Element | null): Promise<Action[]> {
        // read the rows of the nearest objects of each type the page opens
        const places = this.places(element);
        const running = this.actions(element);
        const types = new Set(
            [...(this.#host?.clients.values() ?? [])].flatMap(({ objects }) => objects),
        );
        const lists: Action[] = [];
        const listed = new Set<ObjectType>();
        for (const type of types) {
            const shown = ActionInput.of("target", selected(type)).shown(places);
            const references =
                shown === undefined ? [] : schema.array(ObjectReference).parse(shown);
            const rows = await Promise.all(
                references.map(async (each) =>
                    this.#host?.clients.get(each.scope)?.find(type, each),
                ),
            );
            const known = rows.filter((row) => row !== undefined);
            const client = this.#host?.clients.get(references[0]?.scope ?? "");
            if (known.length > 0 && client !== undefined) {
                listed.add(type);
                lists.push(...type.actions(known, running, client.context().moment));
            }
        }

        // put the rest of the running actions first, those on objects as their rows allow
        const others = running.filter(
            ({ call }) => call?.target === undefined || !listed.has(call.object),
        );

        return [...others, ...lists];
    }

    /** Report whether what an action turns on is on at an element, absent for nothing. */
    isOn(action: Action, element: Element | null): boolean | undefined {
        const key = DeclarationReference.key(action.reference);

        return this.#chosen(key, element)?.handler.isOn?.(element);
    }

    /** List the inputs an action still asks for at an element, beside those given. */
    asked<Inputs extends ActionInputs>(
        action: Action<Inputs>,
        element: Element | null,
        given: Partial<ValuesOf<Inputs>> = {},
    ): readonly ActionInput[] {
        return this.#resolve(action, element, JsonValue.object(given)).asked;
    }

    /** Write what an action does at an element: its sentence, its title while an input is asked. */
    sentence<Inputs extends ActionInputs>(
        action: Action<Inputs>,
        element: Element | null,
        localization: Localization,
        given: Partial<ValuesOf<Inputs>> = {},
    ): string {
        // write the title without a sentence, or while an input is missing
        const { values, asked } = this.#resolve(action, element, JsonValue.object(given));
        if (action.summary === undefined || asked.some(({ presence }) => presence === "required")) {
            return localization.render(action.title);
        }

        // count objects, title a choice, and write any other value as it is
        const written = action.inputs.map((input): [string, string | number] => {
            const value = values[input.name];
            if (input.value.kind === "object") {
                return [input.name, Array.isArray(value) ? value.length : 1];
            } else if (input.value.kind === "choice") {
                const choice = input.value.choices.find((each) => each.value === value);

                return [
                    input.name,
                    choice === undefined ? "" : Choice.of(choice, localization).title,
                ];
            }

            return [
                input.name,
                typeof value === "string" || typeof value === "number" ? value : "",
            ];
        });

        return localization.render(action.summary, Object.fromEntries(written));
    }

    /** Report whether a person picks an input from a list at an element. */
    isListed(action: Action, input: ActionInput, element: Element | null): boolean {
        const key = DeclarationReference.key(action.reference);
        const { kind } = input.value;

        return (
            kind === "object" ||
            kind === "choice" ||
            this.#chosen(key, element)?.handler.choices !== undefined
        );
    }

    /** List the values a person picks an input from. */
    async choices(
        action: Action,
        input: ActionInput,
        element: Element | null,
        localization: Localization,
    ): Promise<readonly Choice[]> {
        // take what the handler lists, else the choices the input declares
        const chosen = this.#chosen(DeclarationReference.key(action.reference), element);
        const listed = chosen?.handler.choices?.(input.name, element) ?? [];
        const { value } = input;
        if (listed.length > 0) {
            return listed;
        } else if (value.kind === "choice") {
            return value.choices.map((each) => Choice.of(each, localization));
        } else if (value.kind !== "object") {
            return [];
        }

        // list the objects of the type that meet its condition, beside the one the action is on
        const { type, where = {} } = value;
        const client = this.#clientOf(type, this.#scopeAt(action, element));
        if (client === undefined) {
            return [];
        }
        const live = client.queryType(type).findMany({ where, limit: CHOICES }).subscribe();
        const rows = await live.read();
        await live.close();

        // read how each looks at this moment
        const { moment } = client.context();

        return rows.map((each) => {
            // read the row
            const row = JsonValue.object(each);
            const id = schema.string().parse(row["id"]);

            return {
                value: type.policy.reference(client.scope, id),
                ...type.appearance(row, moment, localization),
            };
        });
    }

    /** Find where an action runs for an element: there, else where the person last was. */
    placeOf(action: Action, element: Element): Element | null {
        const last = this.#last();
        const place = last?.isConnected === true ? last : null;

        return this.has(action.reference, element) ? element : place;
    }

    /** Offer an action calling a method wherever it applies, until it stops. */
    offer(action: Action): () => void {
        // offer it until it stops
        const key = DeclarationReference.key(action.reference);
        this.#offered.set(key, action);
        this.#changed();

        return () => {
            this.#offered.delete(key);
            this.#changed();
        };
    }

    /** Call methods with a page's clients, in its space. */
    attach(host: ActionHost): void {
        this.#host = host;
        this.#changed();
    }

    /** Ask the person through a part of the page, the latest, until it stops. */
    asks(ask: ActionAsk): () => void {
        this.#asks = [...this.#asks, ask];

        return () => {
            this.#asks = this.#asks.filter((each) => each !== ask);
        };
    }

    /** Tell the person of failures through a part of the page, the latest, until it stops. */
    fails(fail: ActionFail): () => void {
        this.#fails = [...this.#fails, fail];

        return () => {
            this.#fails = this.#fails.filter((each) => each !== fail);
        };
    }

    /** Find an action the page handles or offers, absent for one it does neither with. */
    find(reference: DeclarationReference): Action | undefined {
        const key = DeclarationReference.key(reference);

        return this.#actions.get(key) ?? this.#offered.get(key);
    }

    /** Give a place to the actions in an element, else the whole page, until it stops. */
    provide(place: Accessor<Place>, element?: Element): () => void {
        const provision: Provision = { place, element };
        this.#provisions = [...this.#provisions, provision];

        return () => {
            this.#provisions = this.#provisions.filter((each) => each !== provision);
        };
    }

    /** List the places around an element, the nearest first, the page's own last. */
    places(element: Element | null): Place[] {
        // take the page's places, and those of the parts around the element
        const around = this.#provisions.flatMap(({ place, element: of }) => {
            const isAround = of === undefined || (element !== null && of.contains(element));

            return isAround ? [{ place: place(), depth: depthOf(of) }] : [];
        });

        // put the innermost first, and a later place of one part before an earlier
        return around
            .toReversed()
            .toSorted((one, other) => other.depth - one.depth)
            .map(({ place }) => place);
    }

    /** Read the key combination that runs an action: the person's, else its declared one. */
    keybinding(action: Action): Shortcut | undefined {
        const key = DeclarationReference.key(action.reference);
        const keybindings = this.#keybindings();

        return key in keybindings ? (keybindings[key] ?? undefined) : action.keybinding;
    }

    /** Run the innermost action a key press binds, of a part where one asks. */
    press(event: KeyboardEvent, isCommand: boolean, part?: Element): boolean {
        // find the action, leaving what something took, a button's own, and a field's unless bound anywhere
        const focus = event.target instanceof Element ? event.target : null;
        const isField = isTextField(event.target);
        const isTaken = event.defaultPrevented || isPressing(event);
        const action = isTaken
            ? undefined
            : this.actions(focus).find((each) => {
                  // match its keys, within the part where one asks
                  const keys = this.keybinding(each);
                  const key = DeclarationReference.key(each.reference);
                  const chosen = this.#chosen(key, focus)?.element;
                  const isOwn =
                      part === undefined || (chosen !== undefined && part.contains(chosen));

                  return (
                      isOwn &&
                      keys !== undefined &&
                      (each.focus === "any" || !isField) &&
                      Shortcut.matches(keys, event, isCommand)
                  );
              });
        if (action === undefined) {
            return false;
        }

        // take the press and run the action there
        event.preventDefault();
        this.start(action, focus);

        return true;
    }

    /** Run the actions key presses bind anywhere on the page until aborted. */
    listen(signal: AbortSignal): void {
        const isCommand = Shortcut.isCommand(navigator.userAgent);
        window.addEventListener("keydown", (event) => this.press(event, isCommand), { signal });

        // remember where the person last was, apart from the buttons and menus running actions there
        window.addEventListener(
            "focusin",
            ({ target }) => {
                if (target instanceof Element && target.closest(RUNNERS) === null) {
                    this.#setLast(target);
                }
            },
            { signal },
        );
    }

    /** Run an action as a person does, telling them where it fails. */
    start<Inputs extends ActionInputs>(
        action: Action<Inputs>,
        element: Element | null,
        given: Partial<ValuesOf<Inputs>> = {},
    ): void {
        void this.run(action, element, given).catch((error: unknown) =>
            this.#failed(action, error),
        );
    }

    /**
     * Run an action where a person is, asking what neither places nor what is given answer.
     *
     * Answer the result of each call, none for a handled action, absent once the person cancels.
     */
    async run<Inputs extends ActionInputs>(
        action: Action<Inputs>,
        element: Element | null,
        given: Partial<ValuesOf<Inputs>> = {},
    ): Promise<readonly unknown[] | undefined> {
        // find the handler the element chooses, else the client calling the method
        const key = DeclarationReference.key(action.reference);
        const handler = this.#chosen(key, element)?.handler;
        const { call } = action;
        const client = call === undefined ? undefined : this.#client(call);
        if (handler === undefined && (call === undefined || client === undefined)) {
            throw new TypeError(`the page runs no action '${key}' now`);
        }

        // resolve the inputs, and ask for those left and before removing anything for good
        const own = JsonValue.object(given);
        const first = this.#resolve(action, element, own);
        const answers =
            first.asked.length > 0 || call?.isDestructive === true
                ? await this.#answers({ action, values: first.values, asked: first.asked, element })
                : {};
        if (answers === undefined) {
            return undefined;
        }
        const resolved = this.#resolve(action, element, { ...own, ...answers });

        // call its method in the space of each target, or hand the handler what its inputs take
        if (call !== undefined && client !== undefined && handler === undefined) {
            return this.#perform(action, call, client, resolved);
        }
        const refused = action.inputs.find((input) => {
            const value = resolved.values[input.name];

            return value !== undefined && !input.takes(value);
        });
        if (refused !== undefined) {
            throw new TypeError(`input '${refused.name}' of action '${key}' is invalid`);
        }
        await handler?.run({ element, values: resolved.values });

        return [];
    }

    /** Resolve an action's inputs at an element from what is given and the places around. */
    #resolve(action: Action, element: Element | null, given: ActionValues): ActionResolution {
        const places = this.places(element);

        return action.call === undefined
            ? resolveInputs(action.inputs, places, given)
            : action.call.resolve({ places }, given);
    }

    /** Ask the person through the page, refusing to run without a part that asks. */
    #answers(question: ActionQuestion): Promise<ActionValues | undefined> {
        // ask through the latest part
        const [missing] = question.asked;
        const ask = this.#asks.at(-1);
        if (ask === undefined) {
            const key = DeclarationReference.key(question.action.reference);
            throw new TypeError(
                missing === undefined
                    ? `action '${key}' removes for good, and no part of the page asks to confirm it`
                    : `input '${missing.name}' of action '${key}' is missing, and no part of the page asks for it`,
            );
        }

        return ask(question);
    }

    /** Call an action's method for the targets of each space, with that space's client. */
    async #perform(
        action: Action,
        call: ActionCall,
        client: ObjectClient,
        resolved: ActionResolution,
    ): Promise<readonly unknown[]> {
        // call a method on a scope at once
        const { target } = call;
        if (target === undefined) {
            return this.#predicted(action, client, resolved);
        }

        // call it for each space's targets together
        const given = resolved.values[target];
        const references = schema
            .array(ObjectReference)
            .parse(Array.isArray(given) ? given : [given]);
        const spaces = Map.groupBy(references, (reference) => reference.scope);
        const results = await Promise.all(
            [...spaces].map(([scope, targets]) => {
                // take the space's client, refusing a space the page does not open
                const own = this.#host?.clients.get(scope);
                if (own === undefined) {
                    throw new TypeError(`the page opens no space '${scope}'`);
                }
                const values = { ...resolved.values, [target]: targets };

                return this.#predicted(action, own, { ...resolved, values });
            }),
        );

        return results.flat();
    }

    /** Perform a call, answering its prediction, and tell the person once it is refused. */
    #predicted(
        action: Action,
        client: ObjectClient,
        resolved: ActionResolution,
    ): Promise<readonly unknown[]> {
        // tell of a refusal after the prediction, whose own failure the caller takes
        const { predicted, confirmed } = client.perform(action, resolved);
        const refuse = (error: unknown): void => {
            // leave a mutation a closing client still queues
            if (!(error instanceof DOMException && error.name === "AbortError")) {
                this.#failed(action, error);
            }
        };
        void predicted.then(
            () => confirmed.catch(refuse),
            () => undefined,
        );

        return predicted;
    }

    /** Tell the person an action failed, and report a failure nothing declares. */
    #failed(action: Action, error: unknown): void {
        // tell them through the latest part
        const fail = this.#fails.at(-1);
        const reason = error instanceof FamilyError ? error.message : undefined;
        fail?.(action, reason);

        // report what no declared error explains, or nobody was told
        if (fail === undefined || reason === undefined) {
            globalThis.reportError(error);
        }
    }

    /** Find the client with the type an action calls: the page's space's, else another's. */
    #client(call: ActionCall): ObjectClient | undefined {
        return this.#clientOf(call.object, this.#host?.scope);
    }

    /** Find the client with a type: a scope's, else the page's space's, else another's. */
    #clientOf(type: ObjectType, scope: string | undefined): ObjectClient | undefined {
        const { clients = new Map<string, ObjectClient>(), scope: own } = this.#host ?? {};
        const ordered = [clients.get(scope ?? ""), clients.get(own ?? ""), ...clients.values()];

        return ordered.find((client) => client?.objects.some((each) => each.same(type)) === true);
    }

    /** Read the scope of the nearest object an action acts on at an element, absent for none. */
    #scopeAt({ call }: Action, element: Element | null): string | undefined {
        // take the first of several
        const target = call?.inputs.find(({ name }) => name === call.target);
        const shown = target?.shown(this.places(element));
        const several = schema.array(ObjectReference).safeParse(shown).data;

        return (several?.[0] ?? ObjectReference.safeParse(shown).data)?.scope;
    }

    /** Track the objects the person selects, refusing a second component tracking them. */
    select(read: () => readonly ObjectReference[]): () => void {
        // refuse two components tracking one selection
        if (this.#selection !== undefined) {
            throw new TypeError("the page already tracks a selection");
        }
        this.#selection = read;

        return () => {
            if (this.#selection === read) {
                this.#selection = undefined;
            }
        };
    }

    /** Read the objects the person selected in the page, none while no component tracks them. */
    selection(): ObjectReference[] {
        return [...(this.#selection?.() ?? [])];
    }

    /** Tell readers of the actions that the handlers changed. */
    #changed(): void {
        // write no signal on a server, which renders once
        if (!isServer) {
            this.#setRevision((count) => count + 1);
        }
    }

    /** Choose who handles an action: the innermost around the focus, else the page's latest. */
    #chosen(key: string, focus: Element | null): Handling | undefined {
        // follow the handlers as they come and go
        this.#revision();

        // take the enabled handlers, and those the focus is in the element of
        const enabled = (this.#handlers.get(key) ?? []).filter(
            ({ handler }) => handler.isEnabled?.(focus) ?? true,
        );
        const around = enabled.flatMap(({ handler, element }) =>
            element !== undefined && focus !== null && element.contains(focus)
                ? [{ handler, element }]
                : [],
        );

        // choose the innermost around the focus, else the latest handler for the whole page
        const innermost = around.find(({ element }) =>
            around.every((other) => other.element === element || other.element.contains(element)),
        );

        return innermost ?? enabled.findLast(({ element }) => element === undefined);
    }
}

/** Resolve the inputs of a page's own action from what is given and the places around. */
function resolveInputs(
    inputs: readonly ActionInput[],
    places: readonly Place[],
    given: ActionValues,
): ActionResolution {
    const values: Record<string, JsonValue> = {};
    for (const input of inputs) {
        const value = given[input.name] ?? input.shown(places);
        if (value !== undefined) {
            values[input.name] = value;
        }
    }

    return { values, initial: {}, asked: ActionInput.asked(inputs, values) };
}

/** Measure how deep an element lies in its document, below every element for the page itself. */
function depthOf(element: Element | undefined): number {
    // count the elements around it
    let depth = -1;
    for (let each: Element | null = element ?? null; each !== null; each = each.parentElement) {
        depth += 1;
    }

    return depth;
}

/** Report whether a key press activates the button or link it is on. */
function isPressing(event: KeyboardEvent): boolean {
    const isPlain = !event.ctrlKey && !event.metaKey && !event.altKey && !event.shiftKey;

    return (
        isPlain &&
        (event.key === "Enter" || event.key === " ") &&
        event.target instanceof Element &&
        event.target.matches(PRESSED)
    );
}

/** Report whether a target is a field, which handles keys such as undo itself. */
export function isTextField(target: EventTarget | null): boolean {
    return (
        target instanceof HTMLInputElement ||
        target instanceof HTMLTextAreaElement ||
        target instanceof HTMLSelectElement ||
        (target instanceof HTMLElement &&
            (target.isContentEditable || FIELD_ROLES.has(target.getAttribute("role") ?? "")))
    );
}

/** Give a variable's value to the actions of the running page while the component is mounted. */
export function provideVariable<Value extends JsonValue>(
    variable: PlaceVariable<Value>,
    value: Accessor<Value>,
): void {
    const place = () => new Place().with(variable, value());
    onCleanup(useContext(ActionRegistryContext).provide(place));
}

/** Give a variable's value to the actions in the element this is the `ref` of, while mounted. */
export function provide<Value extends JsonValue>(
    variable: PlaceVariable<Value>,
    value: Accessor<Value>,
): (element: Element) => void {
    const place = () => new Place().with(variable, value());

    return attached((actions, element) => actions.provide(place, element));
}

/** Read the actions the running page's components handle. */
export function useActions(): ActionRegistry {
    return useContext(ActionRegistryContext);
}

/** Handle an action of the running page while the calling component is mounted. */
export function handleAction<Inputs extends ActionInputs>(
    action: Action<Inputs>,
    handler: ActionHandler<Inputs>,
): void {
    // handle it until the component goes
    const stop = useContext(ActionRegistryContext).handle(action, handler);
    onCleanup(stop);
}

/** Handle an action in the element this is the `ref` of while it is mounted. */
export function handle<Inputs extends ActionInputs>(
    action: Action<Inputs>,
    handler: ActionHandler<Inputs>,
): (element: Element) => void {
    return attached((actions, element) => actions.handle(action, handler, element));
}

/** Attach something to the element a `ref` gets until its owner goes. */
function attached(
    attach: (actions: ActionRegistry, element: Element) => () => void,
): (element: Element) => void {
    // read the page's actions, and stop with the component that has the ref
    const actions = useContext(ActionRegistryContext);
    let stop: (() => void) | undefined;
    onCleanup(() => stop?.());

    return (element) => {
        stop?.();
        stop = attach(actions, element);
    };
}

/** Ask the person before actions run, through a part of the page while it is mounted. */
export function handleAsking(ask: ActionAsk): void {
    onCleanup(useContext(ActionRegistryContext).asks(ask));
}

/** Tell the person where actions fail, through a part of the page while it is mounted. */
export function handleFailing(fail: ActionFail): void {
    onCleanup(useContext(ActionRegistryContext).fails(fail));
}

/** Track the objects the person selects in the running page while the component is mounted. */
export function registerSelection(read: () => readonly ObjectReference[]): void {
    // track it until the component goes
    const stop = useContext(ActionRegistryContext).select(read);
    onCleanup(stop);
}
