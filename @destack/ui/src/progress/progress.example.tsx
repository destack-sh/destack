import { t } from "@destack/locale";
import { defineExample } from "@destack/package/declare";
import {
    Progress,
    ProgressIndicator,
    ProgressLabel,
    ProgressTrack,
    ProgressValue,
} from "./progress.tsx";

/** An upload two thirds of the way through. */
export const progressUpload = defineExample({
    of: Progress,
    name: "upload",
    description: t`An upload two thirds of the way through.`,
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
    description: t`A sync of unknown length, its indicator sweeping across the bar.`,
    properties: {
        "aria-label": "Sync",
    },
    render: (properties) => (
        <Progress {...properties}>
            <ProgressTrack>
                <ProgressIndicator />
            </ProgressTrack>
        </Progress>
    ),
});

/** An upload that finished. */
export const progressUploadComplete = defineExample({
    of: Progress,
    name: "uploadComplete",
    description: t`An upload that finished.`,
    properties: {
        value: 100,
        max: 100,
        "aria-label": "Upload",
    },
    render: (properties) => <Progress {...properties} />,
});

/** An import counted in rows, with its label and its value above the bar. */
export const progressImport = defineExample({
    of: ProgressValue,
    name: "import",
    description: t`An import counted in rows, with its label and its value above the bar.`,
    layout: "padded",
    render: () => (
        <Progress value={1840} max={5200} format={{}}>
            <ProgressLabel>Importing contacts</ProgressLabel>
            <ProgressValue>{(written) => `${written ?? ""} of 5,200`}</ProgressValue>
            <ProgressTrack />
        </Progress>
    ),
});
