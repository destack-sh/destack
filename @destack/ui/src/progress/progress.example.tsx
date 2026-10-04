import type { JSX } from "@solidjs/web";
import { Progress } from "./progress.tsx";

/** Show an upload two thirds of the way through. */
export function ProgressExample(): JSX.Element {
    return <Progress value={66} max={100} aria-label="Upload" />;
}
