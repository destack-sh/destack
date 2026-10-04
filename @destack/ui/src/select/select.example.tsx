import type { JSX } from "@solidjs/web";
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

/** Show the sort order of a note list. */
export function SelectExample(): JSX.Element {
    return (
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
    );
}
