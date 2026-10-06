import { expect, test } from "@destack/test";
import { flush } from "@destack/view";
import {
    Select,
    SelectContent,
    SelectGroup,
    SelectItem,
    SelectLabel,
    SelectSeparator,
    SelectTrigger,
    SelectValue,
} from "./index.ts";
import { draw, markup } from "@destack/view/test";

test("render a native select with a button showing the chosen option, grouped options and a separator", () => {
    const container = draw(() => (
        <Select name="sort" aria-label="Sort by">
            <SelectTrigger>
                <SelectValue />
            </SelectTrigger>
            <SelectContent>
                <SelectGroup>
                    <SelectLabel>Date</SelectLabel>
                    <SelectItem value="created">Created</SelectItem>
                    <SelectItem value="updated">Updated</SelectItem>
                </SelectGroup>
                <SelectSeparator />
                <SelectItem value="title">Title</SelectItem>
            </SelectContent>
        </Select>
    ));
    expect(markup(container)).toBe(
        '<select data-slot="select" name="sort" aria-label="Sort by">' +
            '<button data-slot="select-trigger"><selectedcontent data-slot="select-value"></selectedcontent></button>' +
            '<optgroup data-slot="select-group"><legend data-slot="select-label">Date</legend>' +
            '<option data-slot="select-item" value="created">Created</option>' +
            '<option data-slot="select-item" value="updated">Updated</option></optgroup>' +
            '<hr data-slot="select-separator">' +
            '<option data-slot="select-item" value="title">Title</option></select>',
    );
    expect(container.querySelector("select")?.value).toBe("created");
});

/** List the values of a container's chosen options. */
function chosen(container: Element): string[] {
    const select = container.querySelector("select");

    return [...(select?.options ?? [])]
        .filter((option) => option.selected)
        .map((option) => option.value);
}

/** Choose options of a container's select as the person does. */
function pick(container: Element, values: readonly string[]): void {
    const select = container.querySelector("select");
    for (const option of select?.options ?? []) {
        option.selected = values.includes(option.value);
    }
    select?.dispatchEvent(new Event("change", { bubbles: true }));
    flush();
}

test("show a placeholder until an option is chosen, then report the choice", () => {
    const changes: (string | undefined)[] = [];
    const container = draw(() => (
        <Select
            aria-label="Sort by"
            placeholder="Sort by…"
            onValueChange={(value) => changes.push(value)}
        >
            <SelectItem value="created">Created</SelectItem>
            <SelectItem value="title">Title</SelectItem>
        </Select>
    ));
    flush();
    const before = chosen(container);
    pick(container, ["title"]);
    expect([before, chosen(container), changes]).toEqual([[""], ["title"], ["title"]]);
});

test("choose several options of a multiple select as a list box", () => {
    const changes: (readonly string[])[] = [];
    const container = draw(() => (
        <Select
            aria-label="Tags"
            multiple
            defaultValue={["travel"]}
            onValueChange={(value) => changes.push(value)}
        >
            <SelectItem value="travel">Travel</SelectItem>
            <SelectItem value="food">Food</SelectItem>
            <SelectItem value="family">Family</SelectItem>
        </Select>
    ));
    flush();
    const before = chosen(container);
    pick(container, ["travel", "family"]);
    expect([before, changes, container.querySelector("select")?.multiple]).toEqual([
        ["travel"],
        [["travel", "family"]],
        true,
    ]);
});

test("hold a controlled select at its owner's value", () => {
    const container = draw(() => (
        <Select aria-label="Sort by" value="created">
            <SelectItem value="created">Created</SelectItem>
            <SelectItem value="title">Title</SelectItem>
        </Select>
    ));
    flush();
    pick(container, ["title"]);
    expect(chosen(container)).toEqual(["created"]);
});
