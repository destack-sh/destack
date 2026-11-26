import { t } from "@destack/locale";
import { defineExample, type PropertiesOf } from "@destack/package/declare";
import type { JSX } from "@destack/view";
import {
    Avatar,
    AvatarBadge,
    AvatarFallback,
    AvatarGroup,
    AvatarGroupCount,
    AvatarImage,
} from "./avatar.tsx";
import { PICTURES } from "../example/picture.ts";

/** A person's picture. */
export const avatarDefault = defineExample({
    of: Avatar,
    name: "default",
    description: t`Ada's picture, her initials standing in while it loads.`,
    render: renderAda,
});

/** A person without a picture, by their initials. */
export const avatarInitials = defineExample({
    of: Avatar,
    name: "initials",
    description: t`A person without a picture, by their initials.`,
    render: (properties) => (
        <Avatar {...properties}>
            <AvatarFallback>KJ</AvatarFallback>
        </Avatar>
    ),
});

/** A person's initials with a mark on their corner. */
export const avatarBadge = defineExample({
    of: Avatar,
    name: "badge",
    description: t`Ada's initials with a mark on their corner, as for someone online.`,
    render: (properties) => (
        <Avatar {...properties}>
            <AvatarFallback>AL</AvatarFallback>
            <AvatarBadge />
        </Avatar>
    ),
});

/** The people sharing a notebook. */
export const avatarGroupDefault = defineExample({
    of: AvatarGroup,
    name: "default",
    description: t`The people sharing a notebook: two pictures, initials for one without, and the rest as a count.`,
    render: renderSharing,
});

/** Render Ada's picture with her initials. */
function renderAda(properties: PropertiesOf<typeof Avatar>): JSX.Element {
    return (
        <Avatar {...properties}>
            <AvatarImage src={PICTURES.ada} alt="Ada Lovelace" />
            <AvatarFallback>AL</AvatarFallback>
        </Avatar>
    );
}

/** Render the people sharing a notebook. */
function renderSharing(properties: PropertiesOf<typeof AvatarGroup>): JSX.Element {
    return (
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
    );
}
