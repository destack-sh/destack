import type { JSX } from "@solidjs/web";
import { Avatar, AvatarFallback, AvatarGroup, AvatarGroupCount, AvatarImage } from "./avatar.tsx";

/** Show the people sharing a notebook: two pictures, initials for one without, and the rest as a count. */
export function AvatarExample(): JSX.Element {
    return (
        <AvatarGroup aria-label="Shared with">
            <Avatar>
                <AvatarImage src="/people/ada.jpg" alt="Ada Lovelace" />
                <AvatarFallback>AL</AvatarFallback>
            </Avatar>
            <Avatar>
                <AvatarImage src="/people/grace.jpg" alt="Grace Hopper" />
                <AvatarFallback>GH</AvatarFallback>
            </Avatar>
            <Avatar>
                <AvatarFallback>KJ</AvatarFallback>
            </Avatar>
            <AvatarGroupCount>+4</AvatarGroupCount>
        </AvatarGroup>
    );
}
