import { expect, onTestFinished, test } from "@destack/test";
import { BASELINE, installBaseline } from "./baseline.ts";

/** Run the same calls through the platform's built-ins and through the baseline's, side by side. */
function compare(run: () => unknown): { native: unknown; installed: unknown } {
    // record the platform's answers, then take its built-ins away
    const native = run();
    const saved = BASELINE.map(({ owner, name }) => Object.getOwnPropertyDescriptor(owner, name));
    for (const { owner, name } of BASELINE) {
        Reflect.deleteProperty(owner, name);
    }
    onTestFinished(() => {
        BASELINE.forEach(({ owner, name }, index) => {
            const descriptor = saved[index];
            if (descriptor !== undefined) {
                Object.defineProperty(owner, name, descriptor);
            }
        });
    });

    // install the baseline in their place and answer again
    installBaseline();

    return { native, installed: run() };
}

test("encode and decode bytes as the platform does, in hex and both base64 alphabets", () => {
    const bytes = Uint8Array.from([0, 1, 127, 128, 250, 251, 252, 253, 254, 255]);
    const { native, installed } = compare(() => [
        bytes.toHex(),
        bytes.toBase64(),
        bytes.toBase64({ alphabet: "base64url" }),
        bytes.toBase64({ alphabet: "base64url", omitPadding: true }),
        Array.from(Uint8Array.fromHex("00017f80fafbfcfdfeff")),
        Array.from(Uint8Array.fromHex("ABCDEF")),
        Array.from(Uint8Array.fromBase64("AAF/gPr7/P3+/w==")),
        Array.from(Uint8Array.fromBase64("AAF_gPr7_P3-_w", { alphabet: "base64url" })),
        Array.from(Uint8Array.fromBase64(" AAF/ gPr7\n/P3+/w ")),
    ]);

    // the baseline's answers match the platform's on every call
    expect(installed).toEqual(native);
});

test("refuse malformed hex and base64 with a syntax error, as the platform does", () => {
    const { native, installed } = compare(() =>
        ["abc", "zz", "A*==", "AAF_gP"].map((value, index) => {
            try {
                return index < 2 ? Uint8Array.fromHex(value) : Uint8Array.fromBase64(value);
            } catch (error) {
                return error instanceof Error ? error.name : String(error);
            }
        }),
    );

    // both refuse every input with a SyntaxError
    expect([native, installed]).toEqual([
        ["SyntaxError", "SyntaxError", "SyntaxError", "SyntaxError"],
        ["SyntaxError", "SyntaxError", "SyntaxError", "SyntaxError"],
    ]);
});

test("escape strings for regular expressions and parse URLs as the platform does", () => {
    const { native, installed } = compare(() => [
        RegExp.escape("a.b*c(d)[e]{f}|g/h\\i^$+?"),
        RegExp.escape("1st, -x=y <a> #&!%:;@~'`\""),
        RegExp.escape("tab\tline\nend"),
        URL.parse("https://destack.sh/docs/")?.href,
        URL.parse("/blog/", "https://destack.sh")?.href,
        URL.parse("not a url"),
    ]);

    // the baseline's answers match the platform's on every call
    expect(installed).toEqual(native);
});

test("combine and compare sets as the platform does, keeping its order", () => {
    const left = new Set([1, 2, 3, 4]);
    const right = new Set([3, 4, 5]);
    const { native, installed } = compare(() => [
        [...left.union(right)],
        [...left.intersection(right)],
        [...left.difference(right)],
        [...left.symmetricDifference(right)],
        [left.isSubsetOf(right), new Set([3]).isSubsetOf(right)],
        [left.isSupersetOf(new Set([1, 2])), left.isSupersetOf(right)],
        [left.isDisjointFrom(new Set([9])), left.isDisjointFrom(right)],
        [...left.union(new Map([[7, "seven"]]))],
    ]);

    // the baseline's answers match the platform's on every call, maps as set-likes included
    expect(installed).toEqual(native);
});

test("walk the smaller side and refuse invalid set-likes, as the platform does", () => {
    const larger = new Set([5, 4, 3, 2, 1]);
    const smaller = new Set([1, 3, 5]);
    const refused = (value: unknown) => {
        try {
            const method: unknown = Reflect.get(Set.prototype, "union");
            const union: unknown =
                typeof method === "function" ? Reflect.apply(method, larger, [value]) : undefined;

            return union instanceof Set ? [...union] : union;
        } catch (error) {
            return error instanceof Error ? error.name : String(error);
        }
    };
    const { native, installed } = compare(() =>
        [
            [...larger.intersection(smaller)],
            [...smaller.intersection(larger)],
            [...larger.difference(smaller)],
            [...larger.symmetricDifference(new Set([1, 9]))],
            [larger.isDisjointFrom(new Set([9, 8])), smaller.isSubsetOf(larger)],
            [null, { size: Number.NaN, has: () => true, keys: () => [].values() }],
            [
                { size: -1, has: () => true, keys: () => [].values() },
                { size: 1, keys: () => [].values() },
            ],
        ].map((cases, index) => (index < 5 ? cases : cases.map(refused))),
    );

    // the order follows the side each member walks, and both refuse the same set-likes with the same errors
    expect(installed).toEqual(native);
});
