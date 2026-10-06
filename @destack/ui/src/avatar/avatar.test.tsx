import { expect, test } from "@destack/test";
import { classes, draw, markup } from "@destack/view/test";
import { flush } from "@destack/view";
import {
    Avatar,
    AvatarFallback,
    AvatarGroup,
    AvatarImage,
    type AvatarImageStatus,
} from "./index.ts";

/** Render an avatar with a picture and initials, recording the picture's status. */
function drawAvatar(statuses: AvatarImageStatus[]): HTMLElement {
    return draw(() => (
        <Avatar>
            <AvatarImage
                src="/people/ada.jpg"
                alt="Ada Lovelace"
                onStatusChange={(status) => statuses.push(status)}
            />
            <AvatarFallback>AL</AvatarFallback>
        </Avatar>
    ));
}

test("show the fallback beside the hidden image until the image loads, then the image alone", () => {
    const statuses: AvatarImageStatus[] = [];
    const container = drawAvatar(statuses);
    const before = markup(container);
    container.querySelector("img")?.dispatchEvent(new Event("load"));
    flush();
    expect([before, markup(container), statuses]).toEqual([
        '<span data-slot="avatar" data-size="default"><img data-slot="avatar-image" src="/people/ada.jpg" alt="Ada Lovelace"><span data-slot="avatar-fallback">AL</span></span>',
        '<span data-slot="avatar" data-size="default"><img data-slot="avatar-image" src="/people/ada.jpg" alt="Ada Lovelace"></span>',
        ["loaded"],
    ]);
});

test("drop an image that fails to load and keep the fallback", () => {
    const statuses: AvatarImageStatus[] = [];
    const container = drawAvatar(statuses);
    container.querySelector("img")?.dispatchEvent(new Event("error"));
    flush();
    expect([markup(container), statuses]).toEqual([
        '<span data-slot="avatar" data-size="default"><span data-slot="avatar-fallback">AL</span></span>',
        ["error"],
    ]);
});

test("overlap avatars inside a group and leave a lone avatar as it is", () => {
    const container = draw(() => (
        <>
            <Avatar>
                <AvatarFallback>AL</AvatarFallback>
            </Avatar>
            <AvatarGroup>
                <Avatar>
                    <AvatarFallback>GH</AvatarFallback>
                </Avatar>
            </AvatarGroup>
        </>
    ));
    const grouped = container.querySelector("[data-slot=avatar-group] [data-slot=avatar]");
    expect(grouped?.className).not.toBe(classes(container)[0]);
});
