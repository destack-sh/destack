import { defineExample } from "@destack/package/declare";
import { Textarea } from "./textarea.tsx";

/** A comment box that grows with what the person writes. */
export const textareaComment = defineExample({
    of: Textarea,
    name: "comment",
    description: "a comment box that grows with what the person writes",
    render: () => <Textarea name="comment" placeholder="Add a comment" aria-label="Comment" />,
});

/** The comment box unavailable. */
export const textareaCommentDisabled = defineExample({
    of: Textarea,
    name: "comment-disabled",
    description: "the comment box unavailable",
    render: () => (
        <Textarea name="comment" placeholder="Add a comment" aria-label="Comment" disabled />
    ),
});

/** The comment box marked invalid. */
export const textareaCommentInvalid = defineExample({
    of: Textarea,
    name: "comment-invalid",
    description: "the comment box marked invalid",
    render: () => (
        <Textarea
            name="comment"
            placeholder="Add a comment"
            aria-label="Comment"
            aria-invalid="true"
        />
    ),
});
