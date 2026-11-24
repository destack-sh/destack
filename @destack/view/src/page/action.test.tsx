import { t } from "@destack/locale";
import { expect, expectTypeOf, onTestFinished, test, vi } from "@destack/test";
import { createMemo, createSignal, flush } from "solid-js";
import { type Action, defineAction, defineVariable, ObjectError, Place } from "@destack/object";
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
    const actions = new ActionRegistry(keys);
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
