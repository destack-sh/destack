import { defineExample } from "@destack/package/declare";
import { DataList, DataListItem, DataListLabel, DataListValue } from "./data-list.tsx";

/** An error's details beside their labels, to read out to support. */
export const dataListErrorDetails = defineExample({
    of: DataList,
    name: "error-details",
    description: "an error's details beside their labels, to read out to support",
    properties: {
        orientation: "horizontal",
    },
    render: (properties) => (
        <DataList {...properties}>
            <DataListItem>
                <DataListLabel>Code</DataListLabel>
                <DataListValue>500</DataListValue>
            </DataListItem>
            <DataListItem>
                <DataListLabel>Request</DataListLabel>
                <DataListValue>req_7f3a9c2e41b8</DataListValue>
            </DataListItem>
            <DataListItem>
                <DataListLabel>Time</DataListLabel>
                <DataListValue>23 Nov 2026, 14:02 UTC</DataListValue>
            </DataListItem>
        </DataList>
    ),
});

/** A note's properties, each label above its value. */
export const dataListNoteProperties = defineExample({
    of: DataList,
    name: "note-properties",
    description: "a note's properties, each label above its value",
    properties: {
        orientation: "vertical",
    },
    render: (properties) => (
        <DataList {...properties}>
            <DataListItem>
                <DataListLabel>Notebook</DataListLabel>
                <DataListValue>Launch</DataListValue>
            </DataListItem>
            <DataListItem>
                <DataListLabel>Edited</DataListLabel>
                <DataListValue>Yesterday by Ada</DataListValue>
            </DataListItem>
        </DataList>
    ),
});
