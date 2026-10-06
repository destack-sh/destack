import { defineExample } from "@destack/package/declare";
import { Icon } from "@destack/icon";
import { Marker, MarkerContent, MarkerIcon } from "./marker.tsx";

/** The day a conversation continues on and a person joining it. */
export const markerConversationDay = defineExample({
    of: Marker,
    name: "conversation-day",
    description: "the day a conversation continues on and a person joining it",
    render: () => (
        <>
            <Marker variant="separator">
                <MarkerContent>Yesterday</MarkerContent>
            </Marker>
            <Marker>
                <MarkerIcon>
                    <Icon name="plus" />
                </MarkerIcon>
                <MarkerContent>Grace Hopper joined the notebook</MarkerContent>
            </Marker>
        </>
    ),
});
