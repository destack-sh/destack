import { defineExample } from "@destack/package/declare";
import { Button } from "../button/index.ts";
import { Spinner } from "./spinner.tsx";

/** A button that saves while it announces its progress. */
export const spinnerSavingButton = defineExample({
    of: Spinner,
    name: "saving-button",
    description: "a button that saves while it announces its progress",
    render: () => (
        <Button disabled>
            <Spinner aria-label="Saving" />
            Save
        </Button>
    ),
});
