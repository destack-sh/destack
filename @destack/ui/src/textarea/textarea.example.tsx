import type { JSX } from "@solidjs/web";
import { Textarea } from "./textarea.tsx";

/** Show a comment box that grows with what the person writes. */
export function TextareaExample(): JSX.Element {
    return <Textarea name="comment" placeholder="Add a comment" aria-label="Comment" />;
}
