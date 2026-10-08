import { expect, onTestFinished, test } from "@destack/test";
import { schema } from "@destack/schema";
import { render } from "@solidjs/web";
import { flush } from "solid-js";
import { createRouter, defineRoutes, memoryHistory, useLocation, useNavigate } from "./index.ts";
import {
    createLoader,
    createSerializer,
    parseAsArrayOf,
    parseAsBoolean,
    parseAsFloat,
    parseAsHex,
    parseAsInteger,
    parseAsIsoDate,
    parseAsJson,
    parseAsNumberLiteral,
    parseAsString,
    parseAsStringEnum,
    parseAsStringLiteral,
    parseAsTimestamp,
    useQueryState,
    useQueryStates,
} from "./search.ts";

/** The URL state of a list of notes: a search text, a page, a sort and the tags it filters by. */
const notes = {
    query: parseAsString.withDefault(""),
    page: parseAsInteger.withDefault(1).withOptions({ history: "push" }),
    sort: parseAsStringLiteral(["title", "date"]).withDefault("title"),
    tags: parseAsArrayOf(parseAsString).withDefault([]),
};

test("read each value from its text, refusing text that names none", () => {
    expect([
        [parseAsInteger.parse("12"), parseAsInteger.parse("1.5"), parseAsInteger.parse("a")],
        [parseAsFloat.parse("1.5"), parseAsFloat.parse(""), parseAsFloat.parse("x")],
        [parseAsBoolean.parse("true"), parseAsBoolean.parse("false"), parseAsBoolean.parse("1")],
        [parseAsStringLiteral(["a", "b"]).parse("b"), parseAsStringLiteral(["a", "b"]).parse("c")],
        [parseAsArrayOf(parseAsInteger).parse("1,2"), parseAsArrayOf(parseAsInteger).parse("1,x")],
        [parseAsIsoDate.parse("2026-10-08")?.toISOString(), parseAsIsoDate.parse("2026-10")],
        [parseAsHex.parse("ff"), parseAsHex.serialize(10), parseAsHex.parse("zz")],
        [parseAsTimestamp.parse("0")?.toISOString(), parseAsTimestamp.parse("1e3")],
        [parseAsNumberLiteral([1, 2]).parse("2"), parseAsNumberLiteral([1, 2]).parse("3")],
        [
            parseAsStringEnum(["light", "dark"]).parse("dark"),
            parseAsStringEnum(["light"]).parse("x"),
        ],
        [
            parseAsJson(schema.object({ count: schema.number() })).parse('{"count":1}'),
            parseAsJson(schema.object({ count: schema.number() })).parse('{"count":"1"}'),
            parseAsJson(schema.object({ count: schema.number() })).parse("{"),
        ],
    ]).toEqual([
        [12, null, null],
        [1.5, null, null],
        [true, false, null],
        ["b", null],
        [[1, 2], null],
        ["2026-10-08T00:00:00.000Z", null],
        [255, "0a", null],
        ["1970-01-01T00:00:00.000Z", null],
        [2, null],
        ["dark", null],
        [{ count: 1 }, null, null],
    ]);
});

test("write links to values, leaving defaults out and keeping the base's other values and fragment", () => {
    const serialize = createSerializer(notes, { urlKeys: { query: "q" } });

    expect([
        serialize({ query: "milk", page: 2, sort: "title", tags: ["a", "b"] }),
        serialize("/notes?view=grid&page=3", { page: 1 }),
        serialize("/notes?q=old", { query: null }),
        serialize("/notes?q=old#top", { page: 4 }),
    ]).toEqual([
        "?q=milk&page=2&tags=a%2Cb",
        "/notes?view=grid",
        "/notes",
        "/notes?q=old&page=4#top",
    ]);

    // refuse to write a fraction as a whole number, a caller's mistake rounding would hide
    expect(() => serialize({ page: 1.5 })).toThrow(new RangeError("1.5 is no whole number"));
});

/** Render a router over a memory history at a path, exposing what a page reads and how it navigates. */
function renderPage<Exposed>(path: string, read: () => Exposed) {
    // route every path to a page exposing what it reads
    const exposed: { value?: Exposed; search?: () => string; back?: () => void } = {};
    const Page = () => {
        exposed.value = read();
        const location = useLocation();
        exposed.search = () => location.search;
        const navigate = useNavigate();
        exposed.back = () => navigate(-1);

        return <p>page</p>;
    };
    const Router = createRouter({
        routes: defineRoutes([{ path: "*path", component: Page }]),
        history: memoryHistory(path),
    });

    // mount and remove after the test
    const element = document.createElement("main");
    document.body.append(element);
    const dispose = render(() => <Router />, element);
    onTestFinished(() => {
        dispose();
        element.remove();
    });
    flush();

    return exposed;
}

test("follow values with their defaults, writing them back and replacing invalid or default text", () => {
    // open the list with an invalid page, a default sort and a search under its short key, which the page rewrites at once
    const page = renderPage("/notes?q=milk&page=x&sort=title", () =>
        useQueryStates(notes, { urlKeys: { query: "q" } }),
    );
    const [values, set] = page.value ?? [];
    const first = [values?.(), page.search?.()];

    // set a sort, replacing the entry, then a page, pushing one, and go back
    void set?.({ sort: "date" });
    flush();
    void set?.({ page: 2, tags: ["a"] });
    flush();
    const set2 = [values?.(), page.search?.()];
    page.back?.();
    flush();

    expect([first, set2, page.search?.()]).toEqual([
        [{ query: "milk", page: 1, sort: "title", tags: [] }, "?q=milk"],
        [{ query: "milk", page: 2, sort: "date", tags: ["a"] }, "?q=milk&sort=date&page=2&tags=a"],
        "?q=milk&sort=date",
    ]);
});

test("follow one value, null without a default, and remove it by setting null", () => {
    const page = renderPage("/notes?tag=work", () => useQueryState("tag", parseAsString));
    const [tag, setTag] = page.value ?? [];
    const before = tag?.();
    void setTag?.(null);
    flush();

    expect([before, tag?.(), page.search?.()]).toEqual(["work", null, ""]);
});

test("load values from an address or a query outside a component, at their defaults where missing or invalid", () => {
    const load = createLoader(notes, { urlKeys: { query: "q" } });

    expect([
        load("https://destack.sh/notes?q=milk&page=x&tags=a,b"),
        load(new URLSearchParams("sort=date")),
    ]).toEqual([
        { query: "milk", page: 1, sort: "title", tags: ["a", "b"] },
        { query: "", page: 1, sort: "date", tags: [] },
    ]);
});

test("set values from the current ones, resolve to the written query, and remove them all with null", async () => {
    // add a tag from the current tags, then remove every value
    const page = renderPage("/notes?tags=a", () => useQueryStates(notes));
    const [values, set] = page.value ?? [];
    const written = await set?.((old) => ({ tags: [...old.tags, "b"] }));
    flush();
    const added = [values?.().tags, written?.toString(), page.search?.()];
    await set?.(null);
    flush();

    expect([added, page.search?.()]).toEqual([[["a", "b"], "tags=a%2Cb", "?tags=a%2Cb"], ""]);
});

test("follow one value as text without a parser, and set it from the current one", () => {
    const page = renderPage("/notes?q=mi", () => useQueryState("q"));
    const [query, setQuery] = page.value ?? [];
    void setQuery?.((old) => `${old ?? ""}lk`);
    flush();

    expect([query?.(), page.search?.()]).toEqual(["milk", "?q=milk"]);
});
