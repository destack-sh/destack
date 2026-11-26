import { t } from "@destack/locale";
import { defineExample } from "@destack/package/declare";
import { expect, test } from "@destack/test";
import { createSignal } from "../solid/reactive.ts";
import { ViewDriver } from "./driver.ts";

/** A counter, a field and a scrolling list that record what a person does to them. */
function Panel() {
    const [count, setCount] = createSignal(0);
    const [events, setEvents] = createSignal<readonly string[]>([]);
    const note = (event: string) => setEvents((current) => [...current, event]);

    return (
        <div>
            <button
                type="button"
                onClick={() => setCount((current) => current + 1)}
                onContextMenu={() => note("menu")}
                onKeyDown={(event) => note(`key ${event.key}`)}
                onPointerDown={(event) => note(`down ${event.pointerType} ${String(event.button)}`)}
                onPointerUp={() => note("up")}
            >
                Count {count()}
            </button>
            <input
                aria-label="Name"
                onInput={(event) => note(`typed ${event.currentTarget.value}`)}
            />
            <ul
                aria-label="Lines"
                style={{ overflow: "auto", height: "40px" }}
                onScroll={(event) => note(`scrolled ${String(event.currentTarget.scrollTop)}`)}
                onWheel={(event) => note(`wheel ${String(event.deltaY)}`)}
            >
                <li>one</li>
                <li>two</li>
            </ul>
            <output>{events().join(", ")}</output>
        </div>
    );
}

/** The panel as an example a driver starts. */
const panel = defineExample({
    of: Panel,
    name: "panel",
    description: t`A counter, a field and a scrolling list.`,
    render: () => <Panel />,
});

test("play each step on a rendered example: click, press, fill, a held pointer and the wheel", async () => {
    // click the counter, press a key on it, fill the field, hold the secondary button and turn the wheel
    await using driver = ViewDriver.start({ examples: [panel] });
    const counter = { role: "button" };
    await driver.act({ action: "click", target: counter });
    await driver.act({ action: "press", key: "ArrowDown", target: counter });
    await driver.act({ action: "fill", target: { label: "Name" }, value: "Ada" });
    await driver.act({ action: "pointerMove", target: counter });
    await driver.act({ action: "pointerDown", button: "secondary" });
    await driver.act({ action: "pointerUp" });
    await driver.act({ action: "pointerMove", target: { text: "one" } });
    await driver.act({ action: "wheel", deltaY: 30 });

    expect({
        name: driver.observe({ kind: "name", target: counter }),
        value: driver.observe({ kind: "value", target: { label: "Name" } }),
        events: driver.observe({ kind: "text", target: { css: "output" } }),
    }).toEqual({
        name: "Count 1",
        value: "Ada",
        events: "down mouse 0, up, key ArrowDown, typed Ada, down mouse 2, up, menu, wheel 30, scrolled 30",
    });
});

test("press as a pen or a finger where the scenario's globals say so", async () => {
    // press the counter with each kind of pointer
    const events: unknown[] = [];
    for (const pointer of ["pen", "touch"] as const) {
        await using driver = ViewDriver.start({ examples: [panel], globals: { pointer } });
        await driver.act({ action: "click", target: { role: "button" } });
        events.push(driver.observe({ kind: "text", target: { css: "output" } }));
    }

    expect(events).toEqual(["down pen 0, up", "down touch 0, up"]);
});
