import { defineExample } from "@destack/package/declare";
import { Progress, ProgressIndicator } from "./progress.tsx";

/** An upload two thirds of the way through. */
export const progressUpload = defineExample({
    of: Progress,
    name: "upload",
    description: "an upload two thirds of the way through",
    properties: {
        value: 66,
        max: 100,
        "aria-label": "Upload",
    },
    render: (properties) => <Progress {...properties} />,
});

/** A sync of unknown length, its indicator sweeping across the bar. */
export const progressSync = defineExample({
    of: Progress,
    name: "sync",
    description: "a sync of unknown length, its indicator sweeping across the bar",
    properties: {
        "aria-label": "Sync",
    },
    render: (properties) => (
        <Progress {...properties}>
            <ProgressIndicator />
        </Progress>
    ),
});

/** An upload that finished. */
export const progressUploadComplete = defineExample({
    of: Progress,
    name: "upload-complete",
    description: "an upload that finished",
    properties: {
        value: 100,
        max: 100,
        "aria-label": "Upload",
    },
    render: (properties) => <Progress {...properties} />,
});
