import { t } from "@destack/locale";
import { defineExample } from "@destack/package/declare";
import { Textarea } from "./textarea.tsx";

/** A comment box that grows with what the person writes. */
export const textareaComment = defineExample({
    of: Textarea,
    name: "comment",
    description: t`A comment box that grows with what the person writes.`,
    properties: {
        name: "comment",
        placeholder: "Add a comment",
        "aria-label": "Comment",
    },
    render: (properties) => <Textarea {...properties} />,
});

/** The comment box unavailable. */
export const textareaCommentDisabled = defineExample({
    of: Textarea,
    name: "commentDisabled",
    description: t`The comment box unavailable.`,
    properties: {
        name: "comment",
        placeholder: "Add a comment",
        "aria-label": "Comment",
        disabled: true,
    },
    render: (properties) => <Textarea {...properties} />,
});

/** The comment box marked invalid. */
export const textareaCommentInvalid = defineExample({
    of: Textarea,
    name: "commentInvalid",
    description: t`The comment box marked invalid.`,
    properties: {
        name: "comment",
        placeholder: "Add a comment",
        "aria-label": "Comment",
        "aria-invalid": "true",
    },
    render: (properties) => <Textarea {...properties} />,
});
