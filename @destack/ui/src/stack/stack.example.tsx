import { defineExample } from "@destack/package/declare";
import { Button } from "../button/index.ts";
import { Stack } from "./stack.tsx";

/** A note's title, body and action stacked with one space between each. */
export const stackNote = defineExample({
    of: Stack,
    name: "note",
    description: "a note's title, body and action stacked with one space between each",
    render: () => (
        <Stack space="3" render={(attributes) => <article {...attributes} />}>
            <h2>Launch plan</h2>
            <p>Ship to the waitlist on Thursday, then open signups on Friday.</p>
            <Button variant="outline">Open note</Button>
        </Stack>
    ),
});
