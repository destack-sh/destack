import { defineExample } from "@destack/package/declare";
import { Input } from "./input.tsx";

/** A search box that names itself for assistive technology. */
export const inputSearch = defineExample({
    of: Input,
    name: "search",
    description: "a search box that names itself for assistive technology",
    render: () => (
        <Input type="search" name="query" placeholder="Search notes" aria-label="Search notes" />
    ),
});

/** The search box unavailable. */
export const inputSearchDisabled = defineExample({
    of: Input,
    name: "search-disabled",
    description: "the search box unavailable",
    render: () => (
        <Input
            type="search"
            name="query"
            placeholder="Search notes"
            aria-label="Search notes"
            disabled
        />
    ),
});

/** The search box marked invalid. */
export const inputSearchInvalid = defineExample({
    of: Input,
    name: "search-invalid",
    description: "the search box marked invalid",
    render: () => (
        <Input
            type="search"
            name="query"
            placeholder="Search notes"
            aria-label="Search notes"
            aria-invalid="true"
        />
    ),
});
