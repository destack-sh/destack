import { expect, test } from "@destack/test";
import { draw, markup } from "@destack/view/test";
import { Message, MessageContent, MessageFooter } from "./index.ts";

test("render a message on its side, aligning its content and footer to the reader's own side", () => {
    const container = draw(() => (
        <>
            <Message>
                <MessageContent>Hi</MessageContent>
                <MessageFooter>Sent</MessageFooter>
            </Message>
            <Message align="end">
                <MessageContent>Hi</MessageContent>
                <MessageFooter>Sent</MessageFooter>
            </Message>
        </>
    ));
    const [theirs, mine] = [...container.querySelectorAll("[data-slot=message]")];
    expect([
        markup(container).startsWith('<div data-slot="message" data-align="start">'),
        mine?.getAttribute("data-align"),
        theirs?.querySelector("[data-slot=message-content]")?.className ===
            mine?.querySelector("[data-slot=message-content]")?.className,
        theirs?.querySelector("[data-slot=message-footer]")?.className ===
            mine?.querySelector("[data-slot=message-footer]")?.className,
    ]).toEqual([true, "end", false, false]);
});
