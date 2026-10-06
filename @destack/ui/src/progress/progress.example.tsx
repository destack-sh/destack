import { defineExample } from "@destack/package/declare";
import { Progress } from "./progress.tsx";

/** An upload two thirds of the way through. */
export const progressUpload = defineExample({
    of: Progress,
    name: "upload",
    description: "an upload two thirds of the way through",
    render: () => <Progress value={66} max={100} aria-label="Upload" />,
});
