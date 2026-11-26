import { t } from "@destack/locale";
import { defineExample } from "@destack/package/declare";
import { Input } from "./input.tsx";

/** A search box that names itself for assistive technology. */
export const inputDefault = defineExample({
    of: Input,
    name: "default",
    description: t`A search box that names itself for assistive technology.`,
    properties: {
        type: "search",
        name: "query",
        placeholder: "Search notes",
        "aria-label": "Search notes",
    },
    render: (properties) => <Input {...properties} />,
});

/** A title without a frame, as in a dialog it is the first line of. */
export const inputPlain = defineExample({
    of: Input,
    name: "plain",
    description: t`A title without a frame, as in a dialog it is the first line of.`,
    properties: {
        variant: "plain",
        placeholder: "Task title",
        "aria-label": "Title",
    },
    render: (properties) => <Input {...properties} />,
});

/** The search box marked invalid. */
export const inputInvalid = defineExample({
    of: Input,
    name: "invalid",
    description: t`The search box marked invalid.`,
    properties: {
        type: "search",
        name: "query",
        placeholder: "Search notes",
        "aria-label": "Search notes",
        "aria-invalid": "true",
    },
    render: (properties) => <Input {...properties} />,
});
