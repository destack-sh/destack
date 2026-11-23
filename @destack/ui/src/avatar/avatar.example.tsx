import { defineExample } from "@destack/package/declare";
import { Avatar, AvatarFallback, AvatarGroup, AvatarGroupCount, AvatarImage } from "./avatar.tsx";
import { PICTURES } from "../example/picture.ts";

/** The people sharing a notebook. */
export const avatarNotebookPeople = defineExample({
    of: Avatar,
    name: "notebook-people",
    description:
        "the people sharing a notebook: two pictures, initials for one without, and the rest as a count",
    render: () => (
        <AvatarGroup aria-label="Shared with">
            <Avatar>
                <AvatarImage src={PICTURES.ada} alt="Ada Lovelace" />
                <AvatarFallback>AL</AvatarFallback>
            </Avatar>
            <Avatar>
                <AvatarImage src={PICTURES.grace} alt="Grace Hopper" />
                <AvatarFallback>GH</AvatarFallback>
            </Avatar>
            <Avatar>
                <AvatarFallback>KJ</AvatarFallback>
            </Avatar>
            <AvatarGroupCount>+4</AvatarGroupCount>
        </AvatarGroup>
    ),
});

/** The people sharing a notebook at the large size. */
export const avatarNotebookPeopleLarge = defineExample({
    of: AvatarGroup,
    name: "notebook-people-large",
    description: "the people sharing a notebook at the large size",
    properties: {
        size: "lg",
    },
    render: (properties) => (
        <AvatarGroup aria-label="Shared with" {...properties}>
            <Avatar>
                <AvatarImage src={PICTURES.ada} alt="Ada Lovelace" />
                <AvatarFallback>AL</AvatarFallback>
            </Avatar>
            <Avatar>
                <AvatarImage src={PICTURES.grace} alt="Grace Hopper" />
                <AvatarFallback>GH</AvatarFallback>
            </Avatar>
            <Avatar>
                <AvatarFallback>KJ</AvatarFallback>
            </Avatar>
            <AvatarGroupCount>+4</AvatarGroupCount>
        </AvatarGroup>
    ),
});

/** A team's people by their initials, each in a color of their choosing. */
export const avatarTeamColors = defineExample({
    of: Avatar,
    name: "team-colors",
    description: "a team's people by their initials, each in their own color",
    properties: {
        color: "cyan",
    },
    render: (properties) => (
        <AvatarGroup aria-label="Team">
            <Avatar {...properties}>
                <AvatarFallback>F</AvatarFallback>
            </Avatar>
            <Avatar color="pink">
                <AvatarFallback>I</AvatarFallback>
            </Avatar>
            <Avatar color="blue">
                <AvatarFallback>R</AvatarFallback>
            </Avatar>
            <Avatar>
                <AvatarFallback>KB</AvatarFallback>
            </Avatar>
        </AvatarGroup>
    ),
});
