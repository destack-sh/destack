import {
    Action,
    type ActionInput,
    type MethodKind,
    type ObjectType,
    type ObjectView,
} from "@destack/object";
import { type KeyPress, Shortcut } from "@destack/locale";
import type { JsonObject, JsonValue } from "@destack/schema";
import type { Frame } from "./frame.ts";
import { type Page, type PageKey, PageState } from "./page.ts";

/** The margin the pager's own lines start at, beside the gutter's words. */
const MARGIN = "  ";

/** A key's name rather than a character, such as ArrowUp or Backspace. */
const NAMED_KEY = /^[A-Z][a-zA-Z]+$/u;

/** The most actions the palette lists at once. */
const PALETTE_ROWS = 8;

/** The word a person types to confirm an action that removes its targets for good. */
const CONFIRMATION = "yes";

/** The pager's own keys in browsing, by key: moves on the page and the modes they open. */
const BROWSE_KEYS = new Map<
    string,
    PageKey | "quit" | "palette" | "transitions" | "edit" | "view" | "filter"
>([
    ["ArrowUp", "up"],
    ["k", "up"],
    ["ArrowDown", "down"],
    ["j", "down"],
    ["ArrowLeft", "left"],
    ["h", "left"],
    ["ArrowRight", "right"],
    ["l", "right"],
    ["Enter", "enter"],
    [" ", "select"],
    ["q", "quit"],
    ["Escape", "quit"],
    [":", "palette"],
    ["s", "transitions"],
    ["e", "edit"],
    ["v", "view"],
    ["/", "filter"],
]);

/** What the pager does with a key: browse, filter, pick an action, or fill its input. */
export type PagerMode =
    | { readonly kind: "browse" }
    | { readonly kind: "filter"; readonly text: string }
    | {
          readonly kind: "palette";
          readonly text: string;
          readonly cursor: number;
          readonly actions: readonly Action[];
          readonly targets: readonly JsonObject[];
      }
    | {
          readonly kind: "input";
          readonly action: Action;
          readonly targets: readonly JsonObject[];
          readonly questions: readonly Question[];
          readonly values: JsonObject;
          readonly text: string;
      };

/** A question the pager asks before running an action: one of its arguments, or a confirmation. */
type Question = ActionInput | "confirmation";

/** An answer read by its question: its value, none for a skipped argument, or invalid. */
type Typed = { readonly value: JsonValue | undefined } | "invalid";

/** What a key asks of the loop driving the pager. */
export type PagerStep =
    | { readonly kind: "draw" }
    | { readonly kind: "quit" }
    | { readonly kind: "view"; readonly view: ObjectView }
    | {
          readonly kind: "run";
          readonly action: Action;
          readonly targets: readonly JsonObject[];
          readonly input: JsonObject;
      };

/** The pager after a key, and what the key asks of the loop. */
export interface PagerAnswer {
    /** The pager after the key. */
    readonly pager: Pager;
    /** What the key asks of the loop. */
    readonly step: PagerStep;
}

/** The method kind each narrowing key lists, the palette listing every kind. */
const NARROWED: Readonly<Record<"transitions" | "edit", MethodKind>> = {
    transitions: "transition",
    edit: "update",
};

/** A page under a person's keys: where they stand, what they type, and the actions they run. */
export class Pager {
    /** The type whose objects the page lists. */
    readonly type: ObjectType;
    /** The actions the type's packages declare, beside those its methods derive. */
    readonly declared: readonly Action[];
    /** Where the person stands on the page. */
    readonly state: PageState;
    /** What the next key does. */
    readonly mode: PagerMode;
    /** The text the rows' titles are filtered by, empty for every row. */
    readonly filter: string;
    /** The line telling what the last action did, empty for none. */
    readonly status: string;

    /** Create a pager of a type and its actions, a state, a mode, a filter and a status. */
    constructor(
        paged: Pick<Pager, "type" | "declared">,
        state: PageState,
        mode: PagerMode,
        filter: string,
        status: string,
    ) {
        // set the type and its actions, the state, the mode, the filter and the status
        this.type = paged.type;
        this.declared = paged.declared;
        this.state = state;
        this.mode = mode;
        this.filter = filter;
        this.status = status;
    }

    /** Start paging a type's objects on the first line, browsing every row. */
    static start(type: ObjectType, declared: readonly Action[]): Pager {
        return new Pager({ type, declared }, PageState.start(), { kind: "browse" }, "", "");
    }

    /** Report what an action did, browsing again. */
    told(status: string): Pager {
        return new Pager(this, this.state, { kind: "browse" }, this.filter, status);
    }

    /** Answer a key on a page of a type's objects, among its views and its declared actions. */
    press(
        event: KeyPress,
        page: Page,
        view: ObjectView,
        frame: Frame,
        height: number,
    ): PagerAnswer {
        // type into a filter, a palette or an input, else browse
        const moment = { now: frame.now, timeZone: frame.terminal.timeZone };
        if (this.mode.kind === "filter") {
            return this.#filtering(event);
        } else if (this.mode.kind === "palette") {
            return this.#picking(event, this.mode, frame);
        } else if (this.mode.kind === "input") {
            return this.#filling(event, this.mode);
        }

        // run an action whose key combination the key presses, on the targets
        const targets = this.#targets(page);
        const { type } = this;
        const actions = type.actions(targets, this.declared, moment);
        const bound = actions.find(
            (action) =>
                action.keybinding !== undefined &&
                Shortcut.matches(action.keybinding, event, false),
        );
        if (bound !== undefined) {
            return this.#chosen(bound, targets);
        }

        // quit on ctrl-c, then answer the pager's own keys
        const intent = event.ctrlKey
            ? event.key === "c"
                ? "quit"
                : undefined
            : BROWSE_KEYS.get(event.key);
        if (intent === undefined) {
            return { pager: this, step: { kind: "draw" } };
        } else if (intent === "quit") {
            return { pager: this, step: { kind: "quit" } };
        }
        // open the palette on the actions for the targets, narrowed to transitions or edits
        else if (intent === "palette" || intent === "transitions" || intent === "edit") {
            const listed =
                intent === "palette"
                    ? actions
                    : actions.filter(
                          (action) =>
                              action.call !== undefined &&
                              type.method(action.call.method).kind === NARROWED[intent],
                      );
            const [single, ...others] = listed;

            // run the one edit at once, else list the actions
            return intent === "edit" && single !== undefined && others.length === 0
                ? this.#chosen(single, targets)
                : this.#with({ kind: "palette", text: "", cursor: 0, actions: listed, targets });
        }
        // switch to the next view the type offers
        else if (intent === "view") {
            const { views } = type;
            const index = views.findIndex((each) => each.name === view.name);
            const next = views[(index + 1) % views.length] ?? view;

            return {
                pager: new Pager(this, PageState.start(), this.mode, this.filter, ""),
                step: { kind: "view", view: next },
            };
        } else if (intent === "filter") {
            return this.#with({ kind: "filter", text: this.filter });
        }

        // move on the page
        const state = page.press(this.state, intent, height);

        return {
            pager: new Pager(this, state, this.mode, this.filter, ""),
            step: { kind: "draw" },
        };
    }

    /** Draw the page under the pager, with the filter, the palette or the input below it. */
    draw(page: Page, frame: Frame, height: number): string {
        // draw the page in the room the mode's lines leave
        const { terminal } = frame;
        const mode = this.#lines(frame);
        const drawn = page.paged(this.state, height - mode.length, frame, this.type);
        const status = this.status === "" ? [] : [`${MARGIN}${terminal.style("dim", this.status)}`];

        return [drawn, ...status, ...mode].join("\n");
    }

    /** Type a filter's text, applying it on enter and dropping it on escape. */
    #filtering(event: KeyPress): PagerAnswer {
        // apply or drop the filter, else edit its text
        const text = this.mode.kind === "filter" ? this.mode.text : "";
        if (event.key === "Enter") {
            return {
                pager: new Pager(this, PageState.start(), { kind: "browse" }, text, ""),
                step: { kind: "draw" },
            };
        } else if (event.key === "Escape") {
            return {
                pager: new Pager(this, this.state, { kind: "browse" }, "", ""),
                step: { kind: "draw" },
            };
        }

        return this.#with({ kind: "filter", text: edited(text, event) });
    }

    /** Pick an action from the palette by typing its title and moving, choosing it on enter. */
    #picking(
        event: KeyPress,
        mode: Extract<PagerMode, { readonly kind: "palette" }>,
        frame: Frame,
    ): PagerAnswer {
        // close on escape, choose on enter, move with the arrows, else type
        const matching = mode.actions.filter((action) => matches(action, mode.text, frame));
        if (event.key === "Escape") {
            return this.#with({ kind: "browse" });
        } else if (event.key === "Enter") {
            const chosen = matching[mode.cursor];

            return chosen === undefined
                ? this.#with({ kind: "browse" })
                : this.#chosen(chosen, mode.targets);
        } else if (event.key === "ArrowUp" || event.key === "ArrowDown") {
            const step = event.key === "ArrowUp" ? -1 : 1;
            const cursor = Math.max(0, Math.min(matching.length - 1, mode.cursor + step));

            return this.#with({ ...mode, cursor });
        }

        return this.#with({ ...mode, text: edited(mode.text, event), cursor: 0 });
    }

    /** Fill an action's input one field at a time, running it once every field is given. */
    #filling(event: KeyPress, mode: Extract<PagerMode, { readonly kind: "input" }>): PagerAnswer {
        // cancel on escape, else type until enter
        if (event.key === "Escape") {
            return this.#with({ kind: "browse" });
        } else if (event.key !== "Enter") {
            return this.#with({ ...mode, text: edited(mode.text, event) });
        }

        // read the answer by its question, skipping an optional argument left empty
        const [question, ...rest] = mode.questions;
        const typed = question === undefined ? { value: undefined } : answerOf(question, mode.text);
        if (typed === "invalid") {
            const asked = question === "confirmation" ? CONFIRMATION : (question?.name ?? "");

            return {
                pager: new Pager(this, this.state, mode, this.filter, `${asked} is invalid`),
                step: { kind: "draw" },
            };
        }
        const values =
            question === undefined || question === "confirmation" || typed.value === undefined
                ? mode.values
                : { ...mode.values, [question.name]: typed.value };

        // run once every field is given
        return rest.length === 0
            ? {
                  pager: this.#browsing(),
                  step: {
                      kind: "run",
                      action: mode.action,
                      targets: mode.targets,
                      input: values,
                  },
              }
            : this.#with({ ...mode, questions: rest, values, text: "" });
    }

    /** Take a chosen action: run it, or ask its input and a confirmation first. */
    #chosen(chosen: Action, targets: readonly JsonObject[]): PagerAnswer {
        // ask the call's arguments, and a confirmation when it removes for good
        const questions: Question[] = [
            ...(chosen.call
                ?.resolve({ places: [] }, {})
                .asked.filter((input) => input.name !== chosen.call?.target) ?? []),
            ...(chosen.isDestructive ? ["confirmation" as const] : []),
        ];

        return questions.length === 0
            ? {
                  pager: this.#browsing(),
                  step: { kind: "run", action: chosen, targets, input: {} },
              }
            : this.#with({
                  kind: "input",
                  action: chosen,
                  targets,
                  questions,
                  values: {},
                  text: "",
              });
    }

    /** List the rows the next action acts on: the selected ones, else the one under the cursor. */
    #targets(page: Page): JsonObject[] {
        // take the selected rows, else the cursor's, else the first line's
        const rows = page.lines.flatMap((line) =>
            line.row === undefined ? [] : [{ key: line.key, row: line.row }],
        );
        const selected = rows.filter((each) => this.state.selected.has(each.key));
        const cursor = rows.find((each) => each.key === (this.state.cursor ?? rows[0]?.key));

        return selected.length > 0
            ? selected.map((each) => each.row)
            : cursor === undefined
              ? []
              : [cursor.row];
    }

    /** Draw the mode's lines: the filter, the palette's actions, or the field asked for. */
    #lines(frame: Frame): string[] {
        // draw a filter's text, a palette's matches with the cursor's inverted, or the next field
        const { terminal } = frame;
        const mode = this.mode;
        if (mode.kind === "filter") {
            return [`${MARGIN}/ ${mode.text}`];
        } else if (mode.kind === "palette") {
            const matching = mode.actions
                .filter((action) => matches(action, mode.text, frame))
                .slice(0, PALETTE_ROWS);
            const listed = matching.map((action, index) => {
                const title = `${MARGIN}  ${terminal.localization.render(action.title)}`;

                return index === mode.cursor ? terminal.style("inverse", title) : title;
            });

            return [
                `${MARGIN}: ${mode.text}`,
                ...(listed.length === 0
                    ? [`${MARGIN}  ${terminal.style("dim", "no actions")}`]
                    : listed),
            ];
        } else if (mode.kind === "input") {
            const [question] = mode.questions;
            const title = terminal.localization.render(mode.action.title);
            const asked =
                question === undefined
                    ? ""
                    : question === "confirmation"
                      ? `type ${CONFIRMATION} to ${title.toLowerCase()} for good`
                      : question.value.kind === "choice"
                        ? `${question.name} (${question.value.choices.map((each) => each.value).join("/")})`
                        : question.value.kind === "boolean"
                          ? `${question.name} (yes/no)`
                          : question.name;

            return [
                `${MARGIN}${terminal.style("bold", title)} ${terminal.style("dim", asked)} ${mode.text}`,
            ];
        }

        return [];
    }

    /** Switch to a mode where the cursor stands. */
    #with(mode: PagerMode): PagerAnswer {
        return {
            pager: new Pager(this, this.state, mode, this.filter, ""),
            step: { kind: "draw" },
        };
    }

    /** Browse again where the cursor stands. */
    #browsing(): Pager {
        return new Pager(this, this.state, { kind: "browse" }, this.filter, "");
    }
}

/** Report whether an action's title as a person reads it contains typed text, ignoring case. */
function matches(action: Action, text: string, frame: Frame): boolean {
    return frame.terminal.localization
        .render(action.title)
        .toLowerCase()
        .includes(text.toLowerCase());
}

/** Edit typed text by a key: a character appended, the last one taken back by backspace. */
function edited(text: string, event: KeyPress): string {
    // take back the last character, append a typed one, and leave the text for any other key
    if (event.key === "Backspace") {
        return text.slice(0, -1);
    } else if (!NAMED_KEY.test(event.key) && !event.ctrlKey) {
        return `${text}${event.key}`;
    }

    return text;
}

/** Read an answer by its question: a value, none for a skipped argument, or invalid. */
function answerOf(question: Question, text: string): Typed {
    // confirm by the word, and skip an optional argument left empty
    if (question === "confirmation") {
        return text === CONFIRMATION ? { value: undefined } : "invalid";
    } else if (text === "") {
        return question.presence === "required" ? "invalid" : { value: undefined };
    }

    // read a number or a time, a boolean, a choice or text by the input's kind
    const number = Number(text);
    if (question.value.kind === "number" || question.value.kind === "time") {
        return Number.isFinite(number) ? { value: number } : "invalid";
    } else if (question.value.kind === "boolean") {
        return text === "yes" ? { value: true } : text === "no" ? { value: false } : "invalid";
    } else if (question.value.kind === "choice") {
        return question.value.choices.some((each) => each.value === text)
            ? { value: text }
            : "invalid";
    }

    return { value: text };
}
