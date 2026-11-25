import fileTextIcon from "@destack/icon/phosphor/file-text";
import magnifyingGlassIcon from "@destack/icon/phosphor/magnifying-glass";
import { defineExample } from "@destack/package/declare";
import { Icon } from "../icon/index.ts";
import { Spinner } from "../spinner/index.ts";
import { Stack } from "../stack/index.ts";
import { Marker, MarkerContent, MarkerIcon } from "./marker.tsx";

/** What an assistant did for an answer, and does now. */
export const markerSteps = defineExample({
    of: Marker,
    name: "steps",
    description: "what an assistant did for an answer, the last step under way",
    layout: "padded",
    render: () => (
        <Stack space="3">
            <Marker>
                <MarkerIcon>
                    <Icon icon={magnifyingGlassIcon} />
                </MarkerIcon>
                <MarkerContent>Searched the notes of March</MarkerContent>
            </Marker>
            <Marker render={(attributes) => <a {...attributes} href="#budget" />}>
                <MarkerIcon>
                    <Icon icon={fileTextIcon} />
                </MarkerIcon>
                <MarkerContent>Read Budget for the spring trip</MarkerContent>
            </Marker>
            <Marker role="status">
                <MarkerIcon>
                    <Spinner />
                </MarkerIcon>
                <MarkerContent busy>Comparing the quotes</MarkerContent>
            </Marker>
        </Stack>
    ),
});

/** A day, someone joining, and the end of a section. */
export const markerSeparators = defineExample({
    of: Marker,
    name: "separators",
    description: "a day and someone joining between two lines, and a row above a line",
    layout: "padded",
    render: () => (
        <Stack space="4">
            <Marker variant="separator">
                <MarkerContent>Today</MarkerContent>
            </Marker>
            <Marker variant="separator">
                <MarkerContent>Grace joined the conversation</MarkerContent>
            </Marker>
            <Marker variant="border">
                <MarkerContent>Ada renamed the channel to launch</MarkerContent>
            </Marker>
        </Stack>
    ),
});
