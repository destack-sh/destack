import { afterEach, beforeEach, expect, test } from "@destack/test";
import { vi } from "vitest";
import { TABLE } from "@destack/db";
import type { QueryPage, RowChange } from "../query/page.ts";
import { note } from "../test/fixture.ts";
import { coalesce } from "./stream.ts";

/** The interval between published pages, in milliseconds. */
const EVERY = 1000;

beforeEach(() => {
    vi.useFakeTimers();
});

afterEach(() => {
    vi.useRealTimers();
});

test("publish a page at once after a quiet interval, and merge the pages within it at its end", async () => {
    const source = pages();
    const published = coalesce(source.pages, EVERY, [note]);

    // publish the first page at once
    source.push(page(1, [change("update", "a")]));
    expect(await published.next()).toEqual({
        done: false,
        value: page(1, [change("update", "a")]),
    });

    // hold the pages within the interval: rename one note, add and remove another, and add a third
    const merged = settled(published.next());
    source.push(page(2, [change("update", "a", "Renamed")]));
    source.push(page(3, [change("insert", "b")]));
    source.push(page(4, [change("delete", "b")]));
    source.push(page(5, [change("insert", "c")]));
    await vi.advanceTimersByTimeAsync(EVERY - 1);
    expect(merged.value).toBeUndefined();

    // publish each row's net change at the last page's position once the interval ends
    await vi.advanceTimersByTimeAsync(1);
    expect(merged.value).toEqual({
        done: false,
        value: page(5, [change("update", "a", "Renamed"), change("insert", "c")]),
    });

    // publish at once again after a quiet interval
    const quiet = published.next();
    await vi.advanceTimersByTimeAsync(EVERY);
    source.push(page(6, [change("delete", "a")]));
    expect(await quiet).toEqual({ done: false, value: page(6, [change("delete", "a")]) });
});

test("publish the held page once the pages end", async () => {
    const source = pages();
    const published = coalesce(source.pages, EVERY, [note]);

    // publish the first page, then hold the second within the interval
    source.push(page(1, [change("insert", "a")]));
    await published.next();
    source.push(page(2, [change("update", "a", "Renamed")]));

    // publish the held page without waiting for the interval, then end
    source.end();
    expect(await published.next()).toEqual({
        done: false,
        value: page(2, [change("update", "a", "Renamed")]),
    });
    expect(await published.next()).toEqual({ done: true, value: undefined });
});

/** Build a page completing at a sequence. */
function page(sequence: number, changes: RowChange[]): QueryPage {
    return { reset: false, complete: true, changes, position: { epoch: "e", sequence } };
}

/** Build one change of a note by identifier, with its title after an insertion or update. */
function change(operation: RowChange["operation"], id: string, title = "Open"): RowChange {
    return {
        table: note[TABLE].sqlName,
        operation,
        row: operation === "delete" ? { id } : { id, title },
    };
}

/** Follow a promise's value once it settles, absent before. */
function settled<Value>(promise: Promise<Value>): { value: Value | undefined } {
    const holder: { value: Value | undefined } = { value: undefined };
    void promise.then((value) => (holder.value = value));

    return holder;
}

/** Start a source of pages a test pushes one by one and ends. */
function pages(): {
    readonly pages: AsyncGenerator<QueryPage>;
    push(page: QueryPage): void;
    end(): void;
} {
    // queue pushed pages until the generator takes them, waking it as they arrive
    const queued: (QueryPage | undefined)[] = [];
    let wake = () => {};
    const generate = async function* (): AsyncGenerator<QueryPage> {
        for (;;) {
            // wait for the next page or the end
            if (queued.length === 0) {
                await new Promise<void>((resolve) => (wake = resolve));
            }
            const next = queued.shift();
            if (next === undefined) {
                return;
            }
            yield next;
        }
    };

    return {
        pages: generate(),
        push(page) {
            queued.push(page);
            wake();
        },
        end() {
            queued.push(undefined);
            wake();
        },
    };
}
