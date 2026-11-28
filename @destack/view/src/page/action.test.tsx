import { Localization, t } from "@destack/locale";
import { expect, expectTypeOf, onTestFinished, test, vi } from "@destack/test";
import { createMemo, createSignal, flush } from "solid-js";
import {
    type Action,
    ActionKeybinding,
    defineAction,
    defineVariable,
    ObjectError,
    Place,
} from "@destack/object";
import { PackageId } from "@destack/package";
import { DeclarationReference } from "@destack/package/declare";
import { schema } from "@destack/schema";
import { type ActionHandler, ActionRegistry } from "./action.ts";

/** Bold the selected text of an editor. */
const bold = defineAction({
    name: "bold",
    title: t`Bold`,
    category: t`Format`,
    keybinding: "mod+b",
});

test("run an action by the handler around the focus, else the page's, and only while enabled", async () => {
    // handle bold in two editors, the inner one only with a selection, and on the page
    const ran: string[] = [];
    const [hasSelection, setSelection] = createSignal(false);
    const outer = document.createElement("section");
    const inner = document.createElement("textarea");
    const other = document.createElement("textarea");
    outer.append(inner);
    document.body.append(outer, other);
    onTestFinished(() => {
        outer.remove();
        other.remove();
    });
    const actions = new ActionRegistry();
    actions.handle(bold, { run: () => void ran.push("page") });
    actions.handle(bold, { run: () => void ran.push("outer") }, outer);
    const stopInner = actions.handle(
        bold,
        {
            run: () => void ran.push("inner"),
            isEnabled: hasSelection,
        },
        inner,
    );

    // run with focus outside both, in the inner editor without and with a selection, then once it stops
    const press = async (focused: HTMLElement) => {
        await actions.run(bold, focused);
    };
    await press(other);
    await press(inner);
    setSelection(true);
    flush();
    await press(inner);
    stopInner();
    await press(inner);

    expect(ran).toEqual(["page", "outer", "inner", "outer"]);
});

test("list and offer only the actions that run now, refusing one none runs", async () => {
    // handle bold on the page while enabled
    const [isEnabled, setEnabled] = createSignal(true);
    const actions = new ActionRegistry();
    actions.handle(bold, { run: () => {}, isEnabled });

    // read it enabled, then disabled
    const listed = actions.actions(null).map((action) => action.name);
    setEnabled(false);
    flush();
    const refused = await actions.run(bold, null).catch((error: unknown) => String(error));

    expect({
        listed,
        has: actions.has(bold.reference, null),
        after: actions.actions(null),
        refused,
    }).toEqual({
        listed: ["bold"],
        has: false,
        after: [],
        refused: `TypeError: the page runs no action '${bold.package.id}/bold' now`,
    });
});

test("list the actions for a focus as handlers come and go, and hand the focus to the one that runs", async () => {
    // handle bold within an editor while it stays handled
    const ran: (Element | null)[] = [];
    const editor = document.createElement("textarea");
    document.body.append(editor);
    onTestFinished(() => editor.remove());
    const actions = new ActionRegistry();
    const listed = createMemo(() => actions.actions(editor).map((action) => action.name));
    const before = listed();
    const stop = actions.handle(
        bold,
        {
            run: ({ element }) => void ran.push(element),
        },
        editor,
    );
    flush();
    const during = listed();

    // run it in the editor, then stop handling it
    await actions.run(bold, editor);
    stop();
    flush();

    expect({
        before,
        during,
        after: listed(),
        ran,
        handled: actions.has(bold.reference, editor),
    }).toEqual({
        before: [],
        during: ["bold"],
        after: [],
        ran: [editor],
        handled: false,
    });
});

/** The week a calendar shows. */
const visibleWeek = defineVariable({ name: "visible-week", value: schema.number() });

/** Plan the week around, under a given title. */
const plan = defineAction({
    name: "plan",
    title: t`Plan this week`,
    keybinding: "p",
    input: { week: visibleWeek, title: schema.string().default("Plan") },
});

/** Read a place showing a week. */
function at(week: number): () => Place {
    return () => new Place().with(visibleWeek, week);
}

/** Press a key on an element, taken before when asked, reporting whether it is taken. */
function pressOn(target: HTMLElement, key: string, isTaken = false): boolean {
    const event = new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true });
    if (isTaken) {
        event.preventDefault();
    }
    target.dispatchEvent(event);

    return event.defaultPrevented;
}

/** Lay out a page with a calendar in it and a day in that, and a note beside the calendar. */
function layOut() {
    const calendar = document.createElement("section");
    const day = document.createElement("button");
    const note = document.createElement("button");
    calendar.append(day);
    document.body.append(calendar, note);
    onTestFinished(() => {
        calendar.remove();
        note.remove();
    });

    return { calendar, day, note };
}

test("give a handled action the nearest place's values, what is given over them, and refuse a missing one", async () => {
    // give the page's week, the calendar's and the day's
    const { calendar, day, note } = layOut();
    const actions = new ActionRegistry();
    const ran: unknown[] = [];
    actions.handle(plan, { run: ({ element, values }) => void ran.push([element, values]) });
    const stopPage = actions.provide(at(1));
    actions.provide(at(2), calendar);
    const stopDay = actions.provide(at(3), day);

    // run it in the day, beside the calendar, with a week of one's own, and without any
    await actions.run(plan, day);
    await actions.run(plan, note);
    stopDay();
    await actions.run(plan, day, { week: 9, title: "Own" });
    await actions.run(plan, day);
    stopPage();

    expect(ran).toEqual([
        [day, { week: 3 }],
        [note, { week: 1 }],
        [day, { week: 9, title: "Own" }],
        [day, { week: 2 }],
    ]);
    await expect(actions.run(plan, note)).rejects.toThrow(
        new TypeError(
            `input 'week' of action '${plan.package.id}/plan' is missing, and no part of the page asks for it`,
        ),
    );
});

test("run the innermost action a key binds, by the person's keys, leaving fields and taken presses alone", async () => {
    // handle two actions on the same key, one in the calendar, and bind a third to the person's
    const { calendar, day, note } = layOut();
    const field = document.createElement("input");
    calendar.append(field);
    const pick = defineAction({ name: "pick", title: t`Pick`, keybinding: "p" });
    const [keys, setKeys] = createSignal<Readonly<Record<string, string | null>>>({});
    const actions = new ActionRegistry({ keybindings: keys });
    const ran: string[] = [];
    actions.handle(pick, { run: () => void ran.push("page") });
    actions.handle(plan, { run: () => void ran.push("calendar") }, calendar);
    actions.provide(at(1));
    const closing = new AbortController();
    actions.listen(closing.signal);

    // press in the day, beside the calendar, in a field, and one something took
    const pressed = [
        pressOn(day, "p"),
        pressOn(note, "p"),
        pressOn(field, "p"),
        pressOn(day, "p", true),
    ];
    setKeys({ [DeclarationReference.key(plan.reference)]: "q" });
    flush();
    pressed.push(pressOn(day, "p"), pressOn(day, "q"));
    closing.abort();
    pressed.push(pressOn(day, "q"));
    await Promise.resolve();

    expect({
        pressed,
        ran,
        keys: [actions.keybindings(plan, false), actions.keybindings(pick, true)],
    }).toEqual({
        pressed: [true, true, false, true, true, true, false],
        ran: ["calendar", "page", "page", "calendar"],
        keys: [["q"], ["p"]],
    });
});

test("bind a view's key to an action over its own, and a person's own key over the view's", () => {
    // bind pick to v in the view, then to w for the person
    const pick = defineAction({ name: "pick", title: t`Pick`, keybinding: "p" });
    const other = defineAction({ name: "other", title: t`Other`, keybinding: "o" });
    const [keys, setKeys] = createSignal<Readonly<Record<string, string | null>>>({});
    const actions = new ActionRegistry({
        keybindings: keys,
        keys: { [DeclarationReference.key(pick.reference)]: ActionKeybinding.of("v") },
    });
    const viewed = [actions.keybindings(pick, false), actions.keybindings(other, false)];
    setKeys({ [DeclarationReference.key(pick.reference)]: "w" });
    flush();

    expect([...viewed, actions.keybindings(pick, false)]).toEqual([["v"], ["o"], ["w"]]);
});

test("run a key on what the pointer rests on where nothing has the focus, but no destructive one", async () => {
    // handle an action and a destructive one in the calendar
    const { calendar, day } = layOut();
    const remove = defineAction({
        name: "remove",
        title: t`Remove`,
        keybinding: "x",
        isDestructive: true,
    });
    const actions = new ActionRegistry();
    const ran: string[] = [];
    actions.handle(plan, { run: () => void ran.push("plan") }, calendar);
    actions.handle(remove, { run: () => void ran.push("remove") }, calendar);
    actions.provide(at(1));
    const closing = new AbortController();
    actions.listen(closing.signal);
    onTestFinished(() => closing.abort());

    // press on the page, then with the pointer on the day, then with the focus in the day
    const before = pressOn(document.body, "p");
    day.dispatchEvent(new PointerEvent("pointerover", { bubbles: true }));
    const pointed = [pressOn(document.body, "p"), pressOn(document.body, "x")];
    const focused = pressOn(day, "x");
    await Promise.resolve();

    expect({ before, pointed, focused, ran }).toEqual({
        before: false,
        pointed: [true, false],
        focused: true,
        ran: ["plan", "remove"],
    });
});

test("read the values an input has, and what the action runs on, from the innermost handler", async () => {
    // handle a move on the page without either, and in a task with both
    const move = defineAction({ name: "move", title: t`Move`, input: { to: schema.string() } });
    const task = document.createElement("article");
    document.body.append(task);
    onTestFinished(() => task.remove());
    const actions = new ActionRegistry();
    actions.handle(move, { run: () => undefined });
    actions.handle(
        move,
        {
            run: () => undefined,
            chosen: (input, element) => [input, element === task ? "task" : "page"],
            subject: () => "Pack",
        },
        task,
    );
    const [input] = move.inputs;
    const localization = Localization.of("en", []);
    const read = async (element: Element | null) =>
        input === undefined
            ? undefined
            : [
                  actions.chosen(move, input, element),
                  await actions.subjectTitle(move, element, localization),
              ];

    expect([await read(task), await read(null)]).toEqual([
        [["to", "task"], "Pack"],
        [undefined, undefined],
    ]);
});

test("run an action by strokes pressed after one another, which Escape or another key breaks off", async () => {
    // bind a sequence, and the first key of another by itself
    const ran: string[] = [];
    const go = defineAction({ name: "go", title: t`Go`, keybinding: "g m" });
    const mark = defineAction({ name: "mark", title: t`Mark`, keybinding: "m" });
    const closing = new AbortController();
    onTestFinished(() => closing.abort());
    const actions = new ActionRegistry();
    actions.handle(go, { run: () => void ran.push("go") });
    actions.handle(mark, { run: () => void ran.push("mark") });
    actions.listen(closing.signal);
    const taken = ["g", "m", "g", "Escape", "m", "g", "x", "g", "Shift", "m"].map((key) => {
        const event = new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true });
        document.body.dispatchEvent(event);

        return event.defaultPrevented;
    });
    await Promise.resolve();

    expect({ ran, taken }).toEqual({
        ran: ["go", "mark", "go"],
        taken: [true, true, true, true, true, true, false, true, false, true],
    });
});

test("list the actions on the nearest thing around an element, apart from the page's", async () => {
    // handle actions on tabs and on notes in the page, with a tab in a note
    const tabs = defineVariable({ name: "tabs", value: schema.array(schema.string()) });
    const notes = defineVariable({ name: "notes", value: schema.array(schema.string()) });
    const search = defineAction({ name: "search", title: t`Search` });
    const close = defineAction({ name: "close", title: t`Close`, input: { tabs } });
    const next = defineAction({
        name: "next",
        title: t`Next`,
        input: { tabs: tabs.exactOptional() },
    });
    const archive = defineAction({ name: "archive", title: t`Archive`, input: { notes } });
    const note = document.createElement("article");
    const tab = document.createElement("div");
    note.append(tab);
    document.body.append(note);
    onTestFinished(() => note.remove());
    const actions = new ActionRegistry();
    for (const each of [search, close, next, archive]) {
        actions.handle(each, { run: () => undefined });
    }
    actions.provide(() => new Place().with(notes, ["note"]), note);
    actions.provide((on) => (on === tab ? new Place().with(tabs, ["tab"]) : new Place()), tab);
    const on = async (element: Element | null) =>
        (await actions.on(element)).map(({ name }) => name);

    expect({
        tab: [await on(tab), actions.subjectAt(tab)?.part === tab],
        note: [await on(note), actions.subjectAt(note)?.part === note],
        page: [await on(null), actions.subjectAt(null)],
        running: [tab, note, null].map((each) => actions.actions(each).map(({ name }) => name)),
    }).toEqual({
        tab: [["close", "next"], true],
        note: [["archive"], true],
        page: [[], undefined],
        running: [
            ["search", "close", "next", "archive"],
            ["search", "next", "archive"],
            ["search", "next"],
        ],
    });
});

test("list the objects selected where a person is, the nearest of each type", () => {
    const note = {
        packageId: PackageId.parse("package-019f7480-0000-7000-8000-000000000001"),
        type: "note",
        scope: "space-019f7480-0000-7000-8000-000000000002",
        id: "note-019f7480-0000-7000-8000-000000000003",
    };
    const list = document.createElement("div");
    document.body.append(list);
    onTestFinished(() => list.remove());
    const actions = new ActionRegistry();
    actions.provide(() => Place.selected([note]), list);

    expect([actions.selection(list), actions.selection(null)]).toEqual([[note], []]);
});

test("type an action's values by its inputs, the optional ones left out as they may be", () => {
    type Values = Parameters<Parameters<typeof handlePlan>[0]["run"]>[0]["values"];

    expectTypeOf<Values>().toEqualTypeOf<{ readonly week: number } & { readonly title?: string }>();
    expectTypeOf<{ readonly week: string }>().not.toExtend<Values>();
});

/** Handle planning the week, typed by its inputs. */
function handlePlan(handler: ActionHandler<PlanInputs>): void {
    new ActionRegistry().handle(plan, handler);
}

/** The inputs of planning the week. */
type PlanInputs = typeof plan extends Action<infer Inputs> ? Inputs : never;

test("tell the person which action failed and why, and report what no declared error explains", async () => {
    // run an action whose error is declared, then one whose error is not
    const told: [string, string | undefined][] = [];
    const reported: unknown[] = [];
    const report = vi.spyOn(globalThis, "reportError").mockImplementation((error) => {
        reported.push(error);
    });
    onTestFinished(() => report.mockRestore());
    const actions = new ActionRegistry();
    const broken = new TypeError("broken");
    const refused = ObjectError.of("CALL_BLOCKED", "cannot bold: it is locked");
    const failures: Error[] = [refused, broken];
    actions.handle(bold, {
        run: () => {
            throw failures.shift() ?? broken;
        },
    });
    const stop = actions.fails((action, reason) => void told.push([action.name, reason]));
    actions.start(bold, null);
    actions.start(bold, null);
    await Promise.resolve();
    stop();

    // tell nobody once the part goes, reporting instead
    failures.push(refused);
    actions.start(bold, null);
    await Promise.resolve();

    expect({ told, reported }).toEqual({
        told: [
            ["bold", "cannot bold: it is locked"],
            ["bold", undefined],
        ],
        reported: [broken, refused],
    });
});
