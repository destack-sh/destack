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
