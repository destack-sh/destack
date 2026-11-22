import { defineExample } from "@destack/package/declare";
import { Input } from "./input.tsx";

/** A search box that names itself for assistive technology. */
export const inputSearch = defineExample({
    of: Input,
    name: "search",
    description: "a search box that names itself for assistive technology",
    properties: {
        type: "search",
        name: "query",
        placeholder: "Search notes",
        "aria-label": "Search notes",
    },
    render: (properties) => <Input {...properties} />,
});

/** The search box unavailable. */
export const inputSearchDisabled = defineExample({
    of: Input,
    name: "search-disabled",
    description: "the search box unavailable",
    properties: {
        type: "search",
        name: "query",
        placeholder: "Search notes",
        "aria-label": "Search notes",
        disabled: true,
    },
    render: (properties) => <Input {...properties} />,
});

/** The search box marked invalid. */
export const inputSearchInvalid = defineExample({
    of: Input,
    name: "search-invalid",
    description: "the search box marked invalid",
    properties: {
        type: "search",
        name: "query",
        placeholder: "Search notes",
        "aria-label": "Search notes",
        "aria-invalid": "true",
    },
    render: (properties) => <Input {...properties} />,
});
