import { Icon } from "@destack/icon";
import type { JSX } from "@solidjs/web";
import { Marker, MarkerContent, MarkerIcon } from "./marker.tsx";

/** Show the day a conversation continues on and a person joining it. */
export function MarkerExample(): JSX.Element {
    return (
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
    );
}
