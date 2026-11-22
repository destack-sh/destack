import { defineExample } from "@destack/package/declare";
import { Button } from "../button/index.ts";
import { Stack } from "../stack/index.ts";
import { Spinner } from "./spinner.tsx";

/** A button that saves while it announces its progress. */
export const spinnerSavingButton = defineExample({
    of: Spinner,
    name: "saving-button",
    description: "a button that saves while it announces its progress",
    properties: {
        "aria-label": "Saving",
    },
    render: (properties) => (
        <Button disabled>
            <Spinner {...properties} />
            Save
        </Button>
    ),
});

/** A page that loads, with the spinner over a line saying what it waits for. */
export const spinnerLoadingPage = defineExample({
    of: Spinner,
    name: "loading-page",
    description: "a page that loads, with the spinner over a line saying what it waits for",
    properties: {
        "aria-label": "Loading notes",
    },
    render: (properties) => (
        <Stack space="2">
            <Spinner {...properties} />
            <p>Loading your notes</p>
        </Stack>
    ),
});
