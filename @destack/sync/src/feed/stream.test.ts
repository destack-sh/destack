import { afterEach, beforeEach, expect, test } from "@destack/test";
import { vi } from "vitest";
import { TABLE } from "@destack/db";
import type { Page, RowChange } from "../query/page.ts";
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

    // buffer the pages within the interval
    const merged = settled(published.next());
    source.push(page(2, [change("update", "a", "Renamed")]));
    source.push(page(3, [change("insert", "b")]));
    source.push(page(4, [change("delete", "b")]));
    source.push(page(5, [change("insert", "c")]));
    await vi.advanceTimersByTimeAsync(EVERY - 1);
    expect(merged.value).toBeUndefined();

    // publish each row's net change at the interval's end
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

test("publish the buffered page once the pages end", async () => {
    const source = pages();
    const published = coalesce(source.pages, EVERY, [note]);

    // publish the first page and buffer the second within the interval
    source.push(page(1, [change("insert", "a")]));
    await published.next();
    source.push(page(2, [change("update", "a", "Renamed")]));

    // publish the buffered page at the end
    source.end();
    expect(await published.next()).toEqual({
        done: false,
        value: page(2, [change("update", "a", "Renamed")]),
    });
    expect(await published.next()).toEqual({ done: true, value: undefined });
});

/** Build a page completing at a sequence. */
function page(sequence: number, changes: RowChange[]): Page {
    return { reset: false, complete: true, changes, position: { epoch: "e", sequence } };
}

/** Build one change of a note. */
function change(operation: RowChange["operation"], id: string, title = "Open"): RowChange {
    return {
        table: note[TABLE].sqlName,
        operation,
        row: operation === "delete" ? { id } : { id, title },
    };
}

/** Follow a promise's value after it settles. */
function settled<Value>(promise: Promise<Value>): { value: Value | undefined } {
    const holder: { value: Value | undefined } = { value: undefined };
    void promise.then((value) => (holder.value = value));

    return holder;
}

/** Start a source of pushed pages. */
function pages(): {
    readonly pages: AsyncGenerator<Page>;
    push(page: Page): void;
    end(): void;
} {
    // queue pushed pages until taken
    const queued: (Page | undefined)[] = [];
    let arrival = Promise.withResolvers<void>();
    const generate = async function* (): AsyncGenerator<Page> {
        for (;;) {
            // wait for the next page or the end
            while (queued.length === 0) {
                await arrival.promise;
                arrival = Promise.withResolvers<void>();
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
        push(pushed) {
            queued.push(pushed);
            arrival.resolve();
        },
        end() {
            queued.push(undefined);
            arrival.resolve();
        },
    };
}
