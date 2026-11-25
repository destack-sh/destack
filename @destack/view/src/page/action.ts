import { isServer } from "@solidjs/web";
import { DeclarationReference } from "@destack/package/declare";
import { type Accessor, createSignal, onCleanup, type Setter, useContext } from "solid-js";
import { ObjectReference } from "@destack/sync";
import { type Localization, Shortcut, t } from "@destack/locale";
import {
    type Action,
    type ActionCall,
    ActionInput,
    ActionKeybinding,
    type ActionInputs,
    type ActionResolution,
    type ActionValues,
    Choice,
    defineAction,
    type ObjectType,
    Place,
    selected,
    type ValuesOf,
} from "@destack/object";
import type { ObjectNode } from "@destack/object/client";
import { FamilyError } from "@destack/error";
import { JsonValue, mapValues, schema } from "@destack/schema";
import { ActionRegistryContext } from "./context.ts";
import { isModifier, type ShortcutBinding, ShortcutSequence } from "./sequence.ts";

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
    /** Read the values an input has where the person is, which its list marks. */
    readonly chosen?: (input: string, element: Element | null) => readonly JsonValue[];
    /** Title what it runs on where the person is, such as a tab, absent for the page's own. */
    readonly subject?: (element: Element | null) => string | undefined;
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
    /** Read the place where a person is in the element. */
    readonly place: (element: Element | null) => Place;
    /** The element, whose actions find the place. */
    readonly element: Element | undefined;
}

/** What actions act on at a place of a page, such as a list's rows. */
export interface ActionSubject {
    /** The part that gives it, absent for the page itself. */
    readonly part: Element | undefined;
    /** What it gives the actions on it. */
    readonly places: readonly Place[];
}

/** What a page calls methods with: its clients, and the space it is in. */
export interface ActionHost {
    /** The clients of the scopes the page opens, by scope. */
    readonly clients: ReadonlyMap<string, ObjectNode>;
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
    /** The strokes a person types of shortcuts pressed after one another. */
    readonly sequence = new ShortcutSequence<DeclarationReference>();
    /** Count the changes to the handlers, which readers of the actions follow. */
    readonly #revision: Accessor<number>;
    /** Replace the count of changes. */
    readonly #setRevision: Setter<number>;
    /** The clients of the scopes the page opens, which searches follow. */
    readonly #clients: Accessor<readonly ObjectClient[]>;
    /** Replace the clients. */
    readonly #setClients: Setter<readonly ObjectClient[]>;

    /** Create a page's registry without handlers, its keys the person's, else the declared ones. */
    constructor(keybindings: Accessor<Readonly<Record<string, Shortcut | null>>> = () => ({})) {
        // follow the changes to the handlers
        this.#keybindings = keybindings;
        const [revision, setRevision] = createSignal(0, { ownedWrite: true });
        this.#revision = revision;
        this.#setRevision = setRevision;

        // follow the clients apart from the handlers, which change far more
        const [clients, setClients] = createSignal<readonly ObjectClient[]>([], {
            ownedWrite: true,
        });
        this.#clients = clients;
        this.#setClients = setClients;

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
        const places = this.places(focus);
        const handled = [...this.#actions].flatMap(([key, action]) => {
            const chosen = this.#chosen(key, focus);

            return chosen === undefined || !isGiven(action, places)
                ? []
                : [{ action, depth: depthOf(chosen.element) }];
        });
        const ordered = handled.toSorted((one, other) => other.depth - one.depth);

        // add the calls of methods on objects around, and those on the page's space
        const offered = [...this.#offered.values()].filter(
            (action) =>
                action.call !== undefined &&
                this.#client(action.call) !== undefined &&
                isGiven(action, places),
        );

        return [...ordered.map(({ action }) => action), ...offered];
    }

    /** List every action at an element, each titled method of the objects around too. */
    async at(element: Element | null): Promise<Action[]> {
        return this.#listed(this.actions(element), this.places(element));
    }

    /** List the actions on what is nearest around an element, before its rows are read. */
    actionsOn(element: Element | null): Action[] {
        const places = this.subjectAt(element)?.places ?? [];

        return this.actions(element).filter(({ subject }) => subject?.shown(places) !== undefined);
    }

    /** List the actions on what is nearest around an element, as its rows allow. */
    async on(element: Element | null): Promise<Action[]> {
        return this.#listed(this.actionsOn(element), this.subjectAt(element)?.places ?? []);
    }

    /** Find what actions act on nearest around an element, undefined for nothing. */
    subjectAt(element: Element | null): ActionSubject | undefined {
        // look from the innermost part around, the page last
        const running = this.actions(element);
        const provisions = this.#around(element);
        const parts = new Set(provisions.map(({ element: part }) => part));

        return [...parts]
            .map((part) => ({
                part,
                places: provisions
                    .filter((each) => each.element === part)
                    .map(({ place }) => place(element)),
            }))
            .find(
                ({ places }) =>
                    Place.selection(places).length > 0 ||
                    running.some(({ subject }) => subject?.shown(places) !== undefined),
            );
    }

    /** List running actions as the rows of some places allow, and those rows' methods. */
    async #listed(running: readonly Action[], places: readonly Place[]): Promise<Action[]> {
        // read the rows of the nearest objects of each type the page opens
        const types = new Set(
            [...(this.#host?.clients.values() ?? [])].flatMap(({ objects }) => objects),
        );
        const lists: Action[] = [];
        const listed = new Set<ObjectType>();
        for (const type of types) {
            const references = selected(type).read(places);
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

        // put the rest of the running actions first
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
        const live = client.search("", { types: [type], where, limit: CHOICES }).subscribe();
        const hits = await live.read();
        await live.close();

        return hits.map((hit) => client.choice(hit, localization));
    }

    /** Find the space an input's objects are in: the one its action is on, else the page's. */
    scopeOf(action: Action, { value }: ActionInput, element: Element | null): string | undefined {
        return value.kind === "object"
            ? this.#clientOf(value.type, this.#scopeAt(action, element))?.scope
            : undefined;
    }

    /** List the values an input has as the action's handler tells, absent where it tells none. */
    chosen(
        action: Action,
        input: ActionInput,
        element: Element | null,
    ): readonly JsonValue[] | undefined {
        const key = DeclarationReference.key(action.reference);

        return this.#chosen(key, element)?.handler.chosen?.(input.name, element);
    }

    /** Follow the rows an action runs on, telling the values of an input all of them have. */
    follow(
        action: Action,
        input: ActionInput,
        element: Element | null,
        tell: (values: readonly JsonValue[]) => void,
    ): () => void {
        // read the targets of a call
        const { call } = action;
        const given = this.#resolve(action, element, {}).values[call?.target ?? ""];
        const targets = schema
            .array(ObjectReference)
            .safeParse(Array.isArray(given) ? given : [given]).data;
        const closing = new AbortController();
        if (call === undefined || targets === undefined) {
            return () => closing.abort();
        }

        // follow those of each space, telling once every space's are read
        const spaces = Map.groupBy(targets, ({ scope }) => scope);
        const read = new Map<string, readonly (JsonValue | undefined)[]>();
        const watch = async (scope: string, references: readonly ObjectReference[]) => {
            // follow the rows until the caller stops
            const client = this.#host?.clients.get(scope);
            const where = { id: { in: references.map(({ id }) => id) } };
            const live = client?.queryType(call.object).findMany({ where }).subscribe();
            if (live === undefined) {
                return;
            }
            try {
                for await (const rows of live.watch(closing.signal)) {
                    const values = rows.map((row) =>
                        call.chosen(input, scope, JsonValue.object(row)),
                    );
                    read.set(scope, values);
                    const all = [...read.values()].flat();
                    const keys = new Set(all.map((value) => JSON.stringify(value)));
                    const [first] = all;
                    if (read.size === spaces.size) {
                        tell(keys.size === 1 && first !== undefined ? [first] : []);
                    }
                }
            } finally {
                await live.close();
            }
        };
        for (const [scope, references] of spaces) {
            void watch(scope, references).catch(reportError);
        }

        return () => closing.abort();
    }

    /** Title what an action runs on at an element, undefined where nothing says so. */
    async subjectTitle(
        action: Action,
        element: Element | null,
        localization: Localization,
    ): Promise<string | undefined> {
        // take the handler's, for an action of the page's own
        const key = DeclarationReference.key(action.reference);
        const handled = this.#chosen(key, element)?.handler.subject?.(element);
        const target = action.subject;
        if (handled !== undefined || target?.value.kind !== "object") {
            return handled;
        }

        // title a lone object, and count several
        const { type } = target.value;
        const shown = target.shown(this.places(element));
        const several = schema.array(ObjectReference).safeParse(shown).data;
        const lone = ObjectReference.safeParse(shown).data ?? several?.[0];
        if (several !== undefined && several.length !== 1) {
            const count = localization.number(several.length);

            return `${count} ${localization.render(type.declared.title, { count: several.length })}`;
        }
        const client = lone === undefined ? undefined : this.#host?.clients.get(lone.scope);
        const row = lone === undefined ? undefined : await client?.find(type, lone);

        return row === undefined || client === undefined
            ? undefined
            : type.appearance(row, client.context().moment, localization).title;
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
        if (!isServer) {
            this.#setClients([...host.clients.values()]);
        }
    }

    /** List the clients of the scopes the page opens. */
    clients(): readonly ObjectClient[] {
        return this.#clients();
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
    provide(place: (element: Element | null) => Place, element?: Element): () => void {
        // add it, until it stops
        const provision: Provision = { place, element };
        this.#provisions = [...this.#provisions, provision];
        this.#changed();

        return () => {
            this.#provisions = this.#provisions.filter((each) => each !== provision);
            this.#changed();
        };
    }

    /** List the places around an element, the nearest first, the page's own last. */
    places(element: Element | null): Place[] {
        return this.#around(element).map(({ place }) => place(element));
    }

    /** List the provisions around an element, the innermost first, the page's own last. */
    #around(element: Element | null): Provision[] {
        // follow the places as they come and go
        this.#revision();

        // take the page's, and those of the parts around the element
        const around = this.#provisions.filter(
            ({ element: of }) => of === undefined || (element !== null && of.contains(element)),
        );

        // put a later one of a part before an earlier
        return around
            .toReversed()
            .toSorted((one, other) => depthOf(other.element) - depthOf(one.element));
    }

    /** List the key combinations that run an action: the person's, else the platform's declared. */
    keybindings(action: Action, isCommand: boolean): readonly Shortcut[] {
        const chosen = this.#keybindings()[DeclarationReference.key(action.reference)];

        return ActionKeybinding.on(action.keybinding, isCommand, chosen);
    }

    /** Run the innermost action a key press binds, of a part where one asks. */
    press(event: KeyboardEvent, isCommand: boolean, part?: Element): boolean {
        // bind the actions here, within the part where one asks, leaving a field's keys to it
        const focus = event.target instanceof Element ? event.target : null;
        const isField = isTextField(event.target);
        const bindings = this.actions(focus).flatMap((action) => {
            // take one handled in the part, where its focus may be
            const key = DeclarationReference.key(action.reference);
            const chosen = this.#chosen(key, focus)?.element;
            const isOwn = part === undefined || (chosen !== undefined && part.contains(chosen));
            const isBound = isOwn && (action.focus === "any" || !isField);

            return !isBound
                ? []
                : this.keybindings(action, isCommand).map((each) => ({
                      key: each,
                      target: action.reference,
                  }));
        });

        return this.type(event, bindings, { isCommand, part }, (reference) => {
            const action = this.find(reference);
            if (action !== undefined) {
                this.start(action, focus);
            }
        });
    }

    /** Take a press for the shortcuts of some bindings, running what it completes. */
    type(
        event: KeyboardEvent,
        bindings: readonly ShortcutBinding<DeclarationReference>[],
        { isCommand, part }: { readonly isCommand: boolean; readonly part?: Element | undefined },
        run: (reference: DeclarationReference) => void,
    ): boolean {
        // leave what something took, and a button's own
        if (event.defaultPrevented || isPressing(event)) {
            return false;
        }

        // forget the strokes another key breaks off, which Escape does alone
        const isTyping = this.sequence.isTyping();
        const pressed = this.sequence.press(event, bindings, isCommand);
        const isBroken = isTyping && !pressed.isTaken && part === undefined && !isModifier(event);
        if (isBroken) {
            this.sequence.cancel();
        }

        // read any other key that breaks them off anew
        const isEscaped = isBroken && event.key === "Escape";
        const { target, isTaken } =
            isBroken && !isEscaped ? this.sequence.press(event, bindings, isCommand) : pressed;
        if (!isTaken && !isEscaped) {
            return false;
        }

        // take the press, and run what it completes
        event.preventDefault();
        if (target !== undefined) {
            run(target);
        }

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
        client: ObjectNode,
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
        client: ObjectNode,
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
    #client(call: ActionCall): ObjectNode | undefined {
        return this.#clientOf(call.object, this.#host?.scope);
    }

    /** Find the client with a type: a scope's, else the page's space's, else another's. */
    #clientOf(type: ObjectType, scope: string | undefined): ObjectNode | undefined {
        const { clients = new Map<string, ObjectNode>(), scope: own } = this.#host ?? {};
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

    /** List the objects selected at an element, the nearest of each type. */
    selection(element: Element | null): ObjectReference[] {
        return Place.selection(this.places(element));
    }

    /** Tell readers of the actions that the handlers or places changed. */
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

/** Report whether places give what an action acts on, as they do for one on nothing. */
function isGiven({ subject }: Action, places: readonly Place[]): boolean {
    return (
        subject === undefined ||
        subject.presence !== "required" ||
        subject.shown(places) !== undefined
    );
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

/** Give a place to the actions in the element this is the `ref` of, by where a person is in it. */
export function provide(place: (element: Element | null) => Place): (element: Element) => void {
    return attached((actions, element) => actions.provide(place, element));
}

/** Give a place to the actions of the running page while the calling component is mounted. */
export function provideOnPage(place: () => Place): void {
    onCleanup(useContext(ActionRegistryContext).provide(place));
}

/** Read the actions the running page's components handle. */
export function useActions(): ActionRegistry {
    return useContext(ActionRegistryContext);
}

/** Handle an action of the running page while the calling component is mounted. */
export function handleOnPage<Inputs extends ActionInputs>(
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
