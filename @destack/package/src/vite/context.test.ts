import { BrowserContext, CONTEXT_SYMBOL } from "@destack/environment/browser";
import { expect, onTestFinished, test } from "@destack/test";
import { frameModule } from "./context.ts";

/** A module whose async functions await, reject, iterate and yield while reading their context. */
const MODULE = `
export async function trace(realm, key) {
    const seen = [realm.active().getValue(key)];
    await tick();
    seen.push(realm.active().getValue(key));
    try {
        await Promise.reject(new Error("refused"));
    } catch {
        seen.push(realm.active().getValue(key));
    }
    for await (const value of pages()) {
        seen.push(realm.active().getValue(key) + ":" + value);
    }
    seen.push(realm.active().getValue(key));
    {
        await using held = { [Symbol.asyncDispose]: tick };
        seen.push(realm.active().getValue(key));
    }
    seen.push(realm.active().getValue(key));
    pages: for await (const value of pages()) {
        if (value === "first") {
            continue pages;
        }
        seen.push(realm.active().getValue(key) + ":" + value);
    }
    return seen;
}

export async function hold(realm, key) {
    await using held = { [Symbol.asyncDispose]: tick };
    await tick();
    return realm.active().getValue(key);
}

async function* pages() {
    yield "first";
    await tick();
    yield "second";
}

const tick = async () => new Promise((resolve) => setTimeout(resolve, 0));
`;

test("frame each async function, suspending and resuming around its awaits, loops and yields", () => {
    // compile the module's async functions into frames
    const framed = frameModule(
        `async function read(source) { try { return await source.read(); } catch { return undefined; } }\nconst wait = async () => ({ at: await clock.now() });\nfunction sync() { return 1; }\n`,
        "read.js",
    );

    // expect the prelude, a frame per async function, and the plain function untouched
    expect(framed?.toString()).toEqual(
        [
            `const __destackContext = { enter: () => globalThis[globalThis.Symbol.for("destack.context")].enter() };`,
            `async function read(source) { const __destackFrame = __destackContext.enter(); try { try { return __destackFrame.resume(await __destackFrame.suspend(source.read())); } catch { __destackFrame.resume(); return undefined; } } finally { __destackFrame.exit(); } }`,
            `const wait = async () => { const __destackFrame = __destackContext.enter(); try { return ( ({ at: __destackFrame.resume(await __destackFrame.suspend(clock.now())) })); } finally { __destackFrame.exit(); } };`,
            `function sync() { return 1; }`,
            ``,
        ].join("\n"),
    );
});

test("leave a module without async functions unchanged", () => {
    expect(
        frameModule("export const one = 1;\nexport function two() { return 2; }\n", "plain.js"),
    ).toBe(undefined);
});

test("keep each concurrent call's context across awaits, rejections, loops, yields and disposals", async () => {
    // install the realm's context, then load the module compiled into frames
    const realm = BrowserContext.install();
    onTestFinished(() => {
        Reflect.deleteProperty(globalThis, CONTEXT_SYMBOL);
    });
    const framed = frameModule(MODULE, "trace.js");
    const loaded: unknown = await import(
        `data:text/javascript,${encodeURIComponent(framed?.toString() ?? "")}`
    );
    const module = { trace: exportOf(loaded, "trace"), hold: exportOf(loaded, "hold") };

    // run two calls at once in their own contexts, reading the caller's context after each starts
    const key = Symbol("call");
    const root = realm.active();
    const [first, firstAfter] = realm.with(root.setValue(key, "a"), () => [
        module.trace(realm, key),
        realm.active().getValue(key),
    ]);
    const [second, secondAfter] = realm.with(root.setValue(key, "b"), () => [
        module.trace(realm, key),
        realm.active().getValue(key),
    ]);
    const held = realm.with(root.setValue(key, "c"), () => module.hold(realm, key));

    // expect each call to see its own context throughout, the callers theirs, and the root once both end
    expect({
        first: await first,
        second: await second,
        held: await held,
        callers: [firstAfter, secondAfter],
        idle: realm.active().getValue(key),
    }).toEqual({
        first: ["a", "a", "a", "a:first", "a:second", "a", "a", "a", "a:second"],
        second: ["b", "b", "b", "b:first", "b:second", "b", "b", "b", "b:second"],
        held: "c",
        callers: ["a", "b"],
        idle: undefined,
    });
});

/** Read a function the compiled module exports, calling it with a realm and a key. */
function exportOf(
    loaded: unknown,
    name: string,
): (realm: BrowserContext, key: symbol) => Promise<unknown> {
    // take the module's exported function, refusing a module without it
    const exported: unknown =
        typeof loaded === "object" && loaded !== null ? Reflect.get(loaded, name) : undefined;
    if (typeof exported !== "function") {
        throw new TypeError(`the compiled module exports no function '${name}'`);
    }

    return async (realm, key) => {
        const value: unknown = await Reflect.apply(exported, undefined, [realm, key]);

        return value;
    };
}
