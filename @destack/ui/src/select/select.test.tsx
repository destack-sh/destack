import { expect, test } from "@destack/test";
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
