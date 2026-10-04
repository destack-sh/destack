import type { JSX } from "@solidjs/web";
import { Input } from "./input.tsx";

/** Show a search box that names itself for assistive technology. */
export function InputExample(): JSX.Element {
    return (
        <Input type="search" name="query" placeholder="Search notes" aria-label="Search notes" />
    );
}
