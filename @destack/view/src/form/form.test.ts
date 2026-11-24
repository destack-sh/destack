import { Locale, Localization } from "@destack/locale";
import { ObjectError } from "@destack/object";
import { schema } from "@destack/schema";
import { SimulatedTelemetry } from "@destack/environment/simulation";
import { PackageId } from "@destack/package";
import { TelemetryScope } from "@destack/telemetry";
import { expect, test } from "@destack/test";
import { createSignal, flush } from "../solid/reactive.ts";
import { StoredItem } from "../primitives/storage.ts";
import { testWorld } from "../test/browser.ts";
import { wait } from "../test/index.ts";
import { FormControl, type FormOptions } from "./form.ts";

/** The first strong isolate MessageFormat 2 places around a placeholder's value. */
const OPEN = String.fromCodePoint(0x2068);

/** The pop directional isolate closing a placeholder's value. */
const CLOSE = String.fromCodePoint(0x2069);

/** The package keeping drafts. */
const NOTES = {
    id: PackageId.parse("package-01996ab0-0000-7000-8000-000000000001"),
    name: "@example/notes",
    version: "2026.9.0",
};

/** The language problems are written in. */
const ENGLISH = Localization.of(Locale.parse("en"), []);

/** A shopping list. */
const Shopping = schema.object({
    title: schema.string().min(1).max(20),
    count: schema.int().min(1),
    items: schema.array(schema.object({ name: schema.string().min(1) })),
});

/** A shopping list's values. */
type Shopping = schema.Infer<typeof Shopping>;

/** Edit a shopping list, settling each submission as the test says. */
function open(options: Partial<FormOptions<Shopping>> = {}, stored?: StoredItem) {
    const [start, setStart] = createSignal<Shopping>({
        title: "Groceries",
        count: 2,
        items: [{ name: "Milk" }, { name: "Bread" }],
    });
    const submitted: [Shopping, Partial<Shopping>][] = [];
    const answers: PromiseWithResolvers<void>[] = [];
    const form = new FormControl<Shopping>(
        {
            values: start,
            schema: Shopping,
            submit: (values, edits) => {
                submitted.push([values, edits]);
                const answer = Promise.withResolvers<void>();
                answers.push(answer);

                return answer.promise;
            },
            ...options,
        },
        ENGLISH,
        stored,
    );
    const change = (values: Partial<Shopping>) => {
        setStart((current) => ({ ...current, ...values }));
        flush();
    };
    const settle = async (run: (answer: PromiseWithResolvers<void>) => void) => {
        const answer = answers.at(-1);
        if (answer !== undefined) {
            run(answer);
        }
        await wait(0);
        flush();
    };

    return { form, submitted, change, settle, title: form.field("title") };
}

test("keep edits until submitted, then submit them and follow the save", async () => {
    const { form, title, submitted, change, settle } = open();
    title.set("Market");
    flush();
    const kept = [submitted.length, form.isEdited(), title.isEdited(), title.value()];
    form.submit();
    flush();
    const pending = form.status();
    change({ title: "Market" });
    await settle((answer) => answer.resolve());

    expect([kept, submitted, pending, form.status(), form.isEdited(), title.value()]).toEqual([
        [0, true, true, "Market"],
        [
            [
                { title: "Market", count: 2, items: [{ name: "Milk" }, { name: "Bread" }] },
                { title: "Market" },
            ],
        ],
        "pending",
        "saved",
        false,
        "Market",
    ]);
});

test("keep what is edited while a submission is saved", async () => {
    const { form, title, settle } = open();
    title.set("Market");
    form.submit();
    title.set("Market on Friday");
    form.field("count").set(3);
    await settle((answer) => answer.resolve());

    expect([form.isEdited(), title.value(), form.field("count").value()]).toEqual([
        true,
        "Market on Friday",
        3,
    ]);
});

test("show why a value is refused once a person settled on it, then as it changes", () => {
    const { form, title } = open();
    const problems: (string | undefined)[] = [];
    const read = () => {
        flush();
        problems.push(title.problem());
    };
    title.set("");
    read();
    title.settle();
    read();
    title.set("Market");
    read();
    title.set("a market with a very long name");
    read();
    form.reset();
    read();

    expect(problems).toEqual([
        undefined,
        "Enter a value",
        undefined,
        `Enter at most ${OPEN}20${CLOSE} characters`,
        undefined,
    ]);
});

test("submit each change, or each value a person settles on", () => {
    const changing = open({ mode: "change" });
    changing.title.set("A");
    changing.title.set("AB");
    const settling = open({ mode: "settle" });
    settling.title.set("A");
    settling.title.set("AB");
    settling.title.settle();
    settling.form.field("count").settle();
    const edits = (each: typeof changing) => each.submitted.map(([, edit]) => edit);

    expect([edits(changing), edits(settling)]).toEqual([
        [{ title: "A" }, { title: "AB" }],
        [{ title: "AB" }, { title: "AB" }],
    ]);
});

test("show every refusal from the first submit on, by its pointer", () => {
    const { form, submitted } = open();
    form.field("title").set("");
    form.field("items").item(1).set({ name: "" });
    const isSent = form.submit();
    flush();

    expect([isSent, submitted, [...form.problems()], form.field("items").problem()]).toEqual([
        false,
        [],
        [
            ["/title", "Enter a value"],
            ["/items/1/name", "Enter a value"],
        ],
        "Enter a value",
    ]);
});

test("tell why a submission failed, at the values it refused", async () => {
    const { form, title, settle } = open();
    title.set("Market");
    form.submit();
    const refusal = ObjectError.of("INPUT_INVALID", "input 'title' is invalid: it must be free", {
        data: { issues: [{ message: "taken", path: ["title"] }] },
    });
    await settle((answer) => answer.reject(refusal));
    const failed = [form.status(), form.error() === refusal, title.problem(), form.isEdited()];
    title.set("Fair");
    flush();
    const edited = title.problem();
    form.refuse({ title: "That title is taken." });
    flush();

    expect([failed, edited, title.problem()]).toEqual([
        ["failed", true, "Enter a valid value", true],
        undefined,
        "That title is taken.",
    ]);
});

test("show a value someone else set, to keep the person's or take theirs", async () => {
    const { form, title, change, settle } = open();
    const read = () => [title.value(), title.conflict()];
    change({ title: "Fair" });
    const unedited = read();
    title.set("Market");
    change({ title: "Bazaar" });
    const conflicted = read();
    title.keep();
    flush();
    const kept = read();
    change({ title: "Stall" });
    title.take();
    flush();
    const taken = [...read(), form.isEdited()];
    title.set("Market");
    form.submit();
    change({ title: "Market" });
    title.set("Market hall");
    flush();
    const own = read();
    await settle((answer) => answer.resolve());

    expect({ unedited, conflicted, kept, taken, own }).toEqual({
        unedited: ["Fair", undefined],
        conflicted: ["Market", { value: "Bazaar" }],
        kept: ["Market", undefined],
        taken: ["Stall", undefined, false],
        own: ["Market hall", undefined],
    });
});

test("edit the items of a list, and values inside them", () => {
    const { form } = open();
    const items = form.field("items");
    items.insert({ name: "Eggs" });
    items.move(2, 0);
    items.remove(1);
    items.item(1).set({ name: "Rye" });
    flush();

    expect([items.value(), items.item(0).field("name").value()]).toEqual([
        [{ name: "Eggs" }, { name: "Rye" }],
        "Eggs",
    ]);
});

test("submit a cleared optional value as absent, and forget an edit that comes back", () => {
    const submitted: unknown[] = [];
    const form = new FormControl<{ readonly title: string; readonly count?: number | undefined }>(
        {
            values: () => ({ title: "Milk", count: 2 }),
            schema: schema.object({
                title: schema.string(),
                count: schema.int().exactOptional(),
            }),
            submit: (values) => {
                submitted.push(values);

                return undefined;
            },
        },
        ENGLISH,
    );
    form.field("count").set(undefined);
    form.submit();
    form.field("title").set("Bread");
    form.field("title").set("Milk");
    flush();

    expect([submitted, form.status(), form.isEdited()]).toEqual([
        [{ title: "Milk" }],
        "saved",
        false,
    ]);
});

test("refuse a union's value by the issues of its closest member", () => {
    const form = new FormControl<{ readonly shape: unknown }>(
        {
            values: () => ({ shape: undefined }),
            schema: schema.object({
                shape: schema.union([
                    schema.object({ kind: schema.literal("circle"), radius: schema.number() }),
                    schema.object({ kind: schema.literal("square"), side: schema.number() }),
                ]),
            }),
            submit: () => undefined,
        },
        ENGLISH,
    );
    form.field("shape").set({ kind: "square", side: "two" });
    form.submit();
    flush();

    expect([...form.problems()]).toEqual([["/shape/side", "Enter a number"]]);
});

test("keep edits past the page until they are saved, leaving those of another shape", async () => {
    const { tab } = testWorld();
    const telemetry = TelemetryScope.of(NOTES, new SimulatedTelemetry(tab));
    const stored = new StoredItem("draft", tab, telemetry);
    const draft = {
        name: "shopping",
        schema: schema.object({ title: schema.string().exactOptional() }),
    };
    const first = open({ draft }, stored);
    first.title.set("Market");
    const kept = stored.read();
    const second = open({ draft }, stored);
    second.form.restore();
    flush();
    const restored = [second.title.value(), second.form.isEdited()];
    second.form.submit();
    await second.settle((answer) => answer.resolve());
    const saved = stored.read();
    stored.write('{"edits":{"title":4},"base":{}}');
    const third = open({ draft }, stored);
    third.form.restore();

    expect([kept, restored, saved, third.title.value()]).toEqual([
        '{"edits":{"title":"Market"},"base":{"title":"Groceries"}}',
        ["Market", true],
        "",
        "Groceries",
    ]);
});

test("say what a schema says, else what is wrong in general", () => {
    const form = new FormControl<{ readonly title: string; readonly count: number }>(
        {
            values: () => ({ title: "", count: 0 }),
            schema: schema.object({
                title: schema.string().min(1, "Give the trip a title"),
                count: schema.int().min(1),
            }),
            submit: () => undefined,
        },
        ENGLISH,
    );
    form.submit();
    flush();

    expect([...form.problems().values()]).toEqual([
        "Give the trip a title",
        `Enter ${OPEN}1${CLOSE} or more`,
    ]);
});

test("leave what a person submitted before out of conflicts", () => {
    const { form, title, change } = open({ mode: "change" });
    title.set("A");
    title.set("AB");
    change({ title: "A" });
    title.set("ABC");
    flush();

    expect([title.value(), title.conflict(), form.status()]).toEqual(["ABC", undefined, "pending"]);
});

test("show what changed under a kept draft", () => {
    const { tab } = testWorld();
    const telemetry = TelemetryScope.of(NOTES, new SimulatedTelemetry(tab));
    const stored = new StoredItem("draft", tab, telemetry);
    const draft = {
        name: "shopping",
        schema: schema.object({ title: schema.string().exactOptional() }),
    };
    open({ draft }, stored).title.set("Market");
    const { form, title, change } = open({ draft }, stored);
    change({ title: "Fair" });
    form.restore();
    flush();

    expect([title.value(), title.conflict()]).toEqual(["Market", { value: "Fair" }]);
});

test("fail a submission that throws", () => {
    const failure = new Error("offline");
    const { form, title } = open({
        submit: () => {
            throw failure;
        },
    });
    title.set("Market");
    form.submit();
    flush();

    expect([form.status(), form.error() === failure, title.value()]).toEqual([
        "failed",
        true,
        "Market",
    ]);
});
