import type { JSX } from "@solidjs/web";
import { Button } from "../button/index.ts";
import { Spinner } from "./spinner.tsx";

/** Show a button that saves while it announces its progress. */
export function SpinnerExample(): JSX.Element {
    return (
        <Button disabled>
            <Spinner aria-label="Saving" />
            Save
        </Button>
    );
}
