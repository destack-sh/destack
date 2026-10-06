import { defineExample } from "@destack/package/declare";
import { Avatar, AvatarFallback, AvatarGroup, AvatarGroupCount, AvatarImage } from "./avatar.tsx";

/** The people sharing a notebook: two pictures, initials for one without, and the rest as a count. */
export const avatarNotebookPeople = defineExample({
    of: Avatar,
    name: "notebook-people",
    description:
        "the people sharing a notebook: two pictures, initials for one without, and the rest as a count",
    render: () => (
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
    ),
});
