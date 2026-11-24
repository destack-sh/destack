import type { JsonCondition } from "@destack/db";
import { Locale, Localization } from "@destack/locale";
import { present } from "@destack/schema";
import { expect, test } from "@destack/test";
import { createSignal, flush } from "@destack/view";
import { render, stubPopovers } from "@destack/view/test";
import {
    Filter,
    FilterControl,
    type FilterFields,
    type FilterSet,
    matchesFilter,
    FilterGroup,
    FilterMenu,
} from "./index.ts";

/** The states of a task. */
const STATES = [
    { value: "open", title: "Open" },
    { value: "done", title: "Done" },
];

/** The fields of a task. */
const FIELDS: FilterFields = {
    title: { label: "Title", type: { kind: "text" } },
    state: { label: "State", type: { kind: "choice" }, choices: STATES },
    estimate: { label: "Estimate", type: { kind: "number" } },
    due: { label: "Due", type: { kind: "time" } },
    isPinned: { label: "Pinned", type: { kind: "boolean" } },
};

/** List a set's rules: each field, its operator and its values, sets as lists. */
function rulesOf(set: FilterSet): unknown[] {
    return set.items.map((item) =>
        item.kind === "rule"
            ? [item.field, item.operator, ...item.values]
            : item.kind === "set"
              ? { [item.match]: rulesOf(item) }
              : item.where,
    );
}

/** English. */
const ENGLISH = Localization.of(Locale.parse("en"), []);

/** Read a condition's rules: each field, its operator and its values, sets as lists. */
function read(where: JsonCondition): unknown {
    const root = new FilterControl({ fields: FIELDS, value: where }).root();

    return root.match === "OR" ? { OR: rulesOf(root) } : rulesOf(root);
}

test("read the conditions people and programs write", () => {
    expect([
        read({}),
        read({ state: "open", title: { ilike: "%book%" } }),
        read({ AND: [{ estimate: { gte: 1, lte: 3 } }, { state: { in: ["open", "done"] } }] }),
        read({ due: { gte: 5 }, isPinned: true }),
        read({ estimate: { gt: 1, lt: 3 } }),
        read({ OR: [{ state: { ne: "done" } }, { title: { isNull: true } }] }),
        read({ title: { ilike: "a%b" }, author: { name: "Ada" } }),
        read({ title: { ilike: "%50\\%%" } }),
    ]).toEqual([
        [],
        [
            ["state", "is", "open"],
            ["title", "contains", "book"],
        ],
        [
            ["estimate", "between", 1, 3],
            ["state", "is", "open", "done"],
        ],
        [
            ["due", "since", 5],
            ["isPinned", "is", true],
        ],
        [
            ["estimate", "above", 1],
            ["estimate", "below", 3],
        ],
        {
            OR: [
                ["state", "is-not", "done"],
                ["title", "is-empty"],
            ],
        },
        [{ title: { ilike: "a%b" } }, { author: { name: "Ada" } }],
        [["title", "contains", "50%"]],
    ]);
});

test("write what a person adds, changes and removes, unfinished conditions left out", () => {
    const given: JsonCondition[] = [];
    const [where, setWhere] = createSignal<JsonCondition>({ author: { name: "Ada" } });
    const control = new FilterControl({
        fields: FIELDS,
        get value() {
            return where();
        },
        onValueChange: (next) => {
            given.push(next);
            setWhere(next);
        },
    });
    const rule = () => {
        flush();
        const last = present(control.root().items.at(-1), "the last condition");
        if (last.kind !== "rule") {
            throw new TypeError("the last condition is not of one field");
        }

        return last;
    };
    const added = control.add("state");
    const unfinished = [given.length, rule().values, rule().id === added];
    control.set(rule(), ["open"]);
    control.set(rule(), ["open", "done"]);
    const operators = control.operators("state");
    control.compare(rule(), present(operators[1], "is not"));
    control.compare(rule(), present(operators[2], "is empty"));
    control.remove(rule().id);
    flush();
    control.clear();

    expect([unfinished, given]).toEqual([
        [0, [], true],
        [
            { AND: [{ author: { name: "Ada" } }, { state: "open" }] },
            { AND: [{ author: { name: "Ada" } }, { state: { in: ["open", "done"] } }] },
            {
                AND: [
                    { author: { name: "Ada" } },
                    { state: { OR: [{ notIn: ["open", "done"] }, { isNull: true }] } },
                ],
            },
            { AND: [{ author: { name: "Ada" } }, { state: { isNull: true } }] },
            { author: { name: "Ada" } },
            {},
        ],
    ]);
});

test("read back every comparison it writes", () => {
    const values: Readonly<Record<string, readonly (string | number | boolean)[]>> = {
        title: ["50%_"],
        state: ["open", "done"],
        estimate: [1, 3],
        due: ["this-week"],
        isPinned: [true],
    };
    const lost = Object.keys(FIELDS).flatMap((field) => {
        const control = new FilterControl({ fields: FIELDS });

        return control.operators(field).flatMap((operator) => {
            const given = field === "due" && operator.arity === "one" ? [5] : values[field];
            const count = { none: 0, one: 1, span: 1, range: 2, many: 2 }[operator.arity];
            const wanted =
                field === "due" && operator.arity === "range"
                    ? [1, 2]
                    : (given ?? []).slice(0, count);
            const written = operator.write(wanted);
            const back = written === undefined ? undefined : operator.read(written);

            return JSON.stringify(back) === JSON.stringify(wanted)
                ? []
                : [`${field} ${operator.name}`];
        });
    });

    expect(lost).toEqual([]);
});

test("match rows as the engine does, spans of time from a moment", () => {
    const moment = { now: Date.UTC(2026, 10, 25, 12), timeZone: "UTC" };
    const control = new FilterControl({ fields: FIELDS });
    const within = present(control.operators("due")[0], "within");
    const due = (span: string): JsonCondition => ({ due: present(within.write([span]), span) });
    const row = { title: "Book", state: "open", due: Date.UTC(2026, 10, 26, 8) };

    expect([
        matchesFilter({ state: "open" }, row, moment),
        matchesFilter({ title: { ilike: "%OO%" } }, row, moment),
        matchesFilter({ OR: [{ state: "done" }, { title: "Pack" }] }, row, moment),
        matchesFilter({ state: { OR: [{ ne: "done" }, { isNull: true }] } }, {}, moment),
        matchesFilter(due("today"), row, moment),
        matchesFilter(due("tomorrow"), row, moment),
        matchesFilter(due("this-week"), row, moment),
    ]).toEqual([true, true, false, true, false, true, true]);
});

test("show conditions in words, and change them by their parts", () => {
    stubPopovers();
    const given: JsonCondition[] = [];
    const { container } = render(() => (
        <Filter
            fields={FIELDS}
            defaultValue={{ state: { in: ["open", "done"] }, estimate: { gte: 2 } }}
            onValueChange={(where) => void given.push(where)}
        />
    ));
    const shown = () =>
        [...container.querySelectorAll("[data-slot=filter-item]")].map((item) =>
            ["field", "operator", "value"].map(
                (part) => item.querySelector(`[data-slot=filter-${part}]`)?.textContent,
            ),
        );
    const before = shown();
    container.querySelector<HTMLElement>("[data-slot=filter-remove]")?.click();
    flush();
    const after = shown();
    container.querySelector<HTMLElement>("[data-slot=filter-clear]")?.click();
    flush();

    expect([before, after, shown(), given]).toEqual([
        [
            ["State", "is any of", "Open or Done"],
            ["Estimate", "is at least", "2"],
        ],
        [["Estimate", "is at least", "2"]],
        [],
        [{ estimate: { gte: 2 } }, {}],
    ]);
});

test("group conditions, and change how they combine", () => {
    stubPopovers();
    const given: JsonCondition[] = [];
    const { container } = render(() => (
        <Filter
            fields={FIELDS}
            defaultValue={{
                AND: [{ estimate: 2 }, { OR: [{ state: "open" }, { isPinned: true }] }],
            }}
            onValueChange={(where) => void given.push(where)}
        >
            <FilterGroup />
            <FilterMenu />
        </Filter>
    ));
    const [root] = container.querySelectorAll("[data-slot=filter-group]");
    const words = [...(root?.querySelectorAll("[data-slot=filter-match]") ?? [])].map(
        (each) => each.textContent,
    );
    const count = container.querySelector("[data-slot=filter-menu]")?.textContent;
    root?.querySelector<HTMLElement>("button[data-slot=filter-match]")?.click();
    flush();

    expect([words, count, given]).toEqual([
        ["Where", "and", "Where", "or"],
        "3 filters",
        [{ OR: [{ estimate: 2 }, { OR: [{ state: "open" }, { isPinned: true }] }] }],
    ]);
});

test("offer spans and choices as values to add at once", () => {
    const control = new FilterControl({ fields: FIELDS });
    const id = control.add("state", ["done"]);
    flush();
    const set = control.addSet();
    flush();
    control.add("estimate", [2], set);
    flush();

    expect([
        control.suggestions("state", ENGLISH).map(({ title }) => title),
        control.suggestions("due", ENGLISH).length,
        control.suggestions("title", ENGLISH),
        rulesOf(control.root()),
        control.root().items[0]?.id === id,
    ]).toEqual([
        ["Open", "Done"],
        10,
        [],
        [["state", "is", "done"], { OR: [["estimate", "is", 2]] }],
        true,
    ]);
});
