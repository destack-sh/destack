import { defineExample } from "@destack/package/declare";
import { InputGroup, InputGroupAddon, InputGroupInput } from "../input-group/index.ts";
import { Stack } from "../stack/index.ts";
import { CopyButton } from "./copy-button.tsx";

/** The link to a note. */
const LINK = "https://notes.example/trip-to-lisbon";

/** A note's link with the button that copies it, alone and in its field. */
export const copyButtonLink = defineExample({
    of: CopyButton,
    name: "link",
    description: "a note's link with the button that copies it, alone and in its field",
    layout: "padded",
    render: () => (
        <Stack space="4">
            <InputGroup>
                <InputGroupInput readonly aria-label="Link" value={LINK} />
                <InputGroupAddon align="inline-end">
                    <CopyButton value={LINK} label="Copy the link" size="icon-sm" />
                </InputGroupAddon>
            </InputGroup>
            <div>
                <CopyButton value={LINK} variant="outline">
                    Copy link
                </CopyButton>
            </div>
        </Stack>
    ),
});
