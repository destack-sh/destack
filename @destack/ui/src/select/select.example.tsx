import { defineExample } from "@destack/package/declare";
import {
    Select,
    SelectContent,
    SelectGroup,
    SelectItem,
    SelectLabel,
    SelectSeparator,
    SelectTrigger,
    SelectValue,
} from "./select.tsx";

/** The sort order of a note list. */
export const selectSortOrder = defineExample({
    of: Select,
    name: "sort-order",
    description: "the sort order of a note list",
    render: () => (
        <Select name="sort" aria-label="Sort by">
            <SelectTrigger>
                <SelectValue />
            </SelectTrigger>
            <SelectContent>
                <SelectGroup>
                    <SelectLabel>Date</SelectLabel>
                    <SelectItem value="updated">Last edited</SelectItem>
                    <SelectItem value="created">Created</SelectItem>
                </SelectGroup>
                <SelectSeparator />
                <SelectItem value="title">Title</SelectItem>
            </SelectContent>
        </Select>
    ),
});

/** The sort order of a note list unavailable. */
export const selectSortOrderDisabled = defineExample({
    of: Select,
    name: "sort-order-disabled",
    description: "the sort order of a note list unavailable",
    render: () => (
        <Select name="sort" aria-label="Sort by" disabled>
            <SelectTrigger>
                <SelectValue />
            </SelectTrigger>
            <SelectContent>
                <SelectGroup>
                    <SelectLabel>Date</SelectLabel>
                    <SelectItem value="updated">Last edited</SelectItem>
                    <SelectItem value="created">Created</SelectItem>
                </SelectGroup>
                <SelectSeparator />
                <SelectItem value="title">Title</SelectItem>
            </SelectContent>
        </Select>
    ),
});

/** The sort order of a note list before one is chosen. */
export const selectSortOrderPlaceholder = defineExample({
    of: Select,
    name: "sort-order-placeholder",
    description: "the sort order of a note list before one is chosen",
    render: () => (
        <Select name="sort" aria-label="Sort by" placeholder="Sort by…">
            <SelectTrigger>
                <SelectValue />
            </SelectTrigger>
            <SelectContent>
                <SelectItem value="updated">Last edited</SelectItem>
                <SelectItem value="title">Title</SelectItem>
            </SelectContent>
        </Select>
    ),
});

/** The tags of a note, several chosen at once. */
export const selectNoteTags = defineExample({
    of: Select,
    name: "note-tags",
    description: "the tags of a note, several chosen at once",
    render: () => (
        <Select name="tags" aria-label="Tags" multiple defaultValue={["travel", "family"]}>
            <SelectItem value="travel">Travel</SelectItem>
            <SelectItem value="food">Food</SelectItem>
            <SelectItem value="family">Family</SelectItem>
        </Select>
    ),
});
