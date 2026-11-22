import { expect, test } from "@destack/test";
import { classes, markup, render } from "@destack/view/test";
import { flush } from "@destack/view";
import {
    Avatar,
    AvatarFallback,
    AvatarGroup,
    AvatarGroupCount,
    AvatarImage,
    type AvatarImageStatus,
} from "./index.ts";

/** Render an avatar with a picture and initials, recording the picture's status. */
function drawAvatar(statuses: AvatarImageStatus[]): HTMLElement {
    return render(() => (
        <Avatar>
            <AvatarImage
                src="/people/ada.jpg"
                alt="Ada Lovelace"
                onStatusChange={(status) => statuses.push(status)}
            />
            <AvatarFallback>AL</AvatarFallback>
        </Avatar>
    )).container;
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

test("drop an image that fails to load and show the fallback", () => {
    const statuses: AvatarImageStatus[] = [];
    const container = drawAvatar(statuses);
    container.querySelector("img")?.dispatchEvent(new Event("error"));
    flush();
    expect([markup(container), statuses]).toEqual([
        '<span data-slot="avatar" data-size="default"><span data-slot="avatar-fallback">AL</span></span>',
        ["error"],
    ]);
});

test("fill a colored avatar's fallback with its swatch, and a plain one with the muted colors", () => {
    const { container } = render(() => (
        <>
            <Avatar color="teal">
                <AvatarFallback>F</AvatarFallback>
            </Avatar>
            <Avatar>
                <AvatarFallback>K</AvatarFallback>
            </Avatar>
        </>
    ));
    const fallbacks = [...container.querySelectorAll<HTMLElement>("[data-slot=avatar-fallback]")];

    // the colored fallback takes the swatch's solid and label, the plain one neither
    expect([
        container.querySelector("[data-slot=avatar]")?.getAttribute("data-color"),
        fallbacks.map((each) => each.getAttribute("style")),
    ]).toEqual([
        "teal",
        [
            "--x-backgroundColor: var(--destack-swatch-teal-solid); --x-color: var(--destack-swatch-teal-label);",
            null,
        ],
    ]);
});

test("overlap avatars inside a group and leave a lone avatar as it is", () => {
    const { container } = render(() => (
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

test("size a group's avatars and its count alike, an avatar's own size winning", () => {
    // a large group of one avatar at the group's size and one at its own small size
    const { container } = render(() => (
        <AvatarGroup size="lg">
            <Avatar>
                <AvatarFallback>AL</AvatarFallback>
            </Avatar>
            <Avatar size="sm">
                <AvatarFallback>GH</AvatarFallback>
            </Avatar>
            <AvatarGroupCount>+4</AvatarGroupCount>
        </AvatarGroup>
    ));

    expect(
        [...container.querySelectorAll("[data-size]")].map((element) => [
            element.getAttribute("data-slot"),
            element.getAttribute("data-size"),
        ]),
    ).toEqual([
        ["avatar", "lg"],
        ["avatar", "sm"],
        ["avatar-group-count", "lg"],
    ]);
});
