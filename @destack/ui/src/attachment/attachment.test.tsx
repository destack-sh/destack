import { expect, test } from "@destack/test";
import { draw, markup } from "@destack/view/test";
import {
    Attachment,
    AttachmentAction,
    AttachmentContent,
    AttachmentDescription,
    AttachmentTitle,
    AttachmentTrigger,
} from "./index.ts";

/** Read the class of an attachment's element by its slot. */
function classOf(element: Element | undefined, slot: string): string | undefined {
    return element?.querySelector(`[data-slot=${slot}]`)?.className;
}

test("render an attachment with its state, a trigger that opens it and an action that keeps from submitting", () => {
    const container = draw(() => (
        <Attachment>
            <AttachmentTrigger aria-label="Open itinerary.pdf" />
            <AttachmentContent>
                <AttachmentTitle>itinerary.pdf</AttachmentTitle>
            </AttachmentContent>
            <AttachmentAction aria-label="Remove">x</AttachmentAction>
        </Attachment>
    ));
    expect(markup(container)).toBe(
        '<div data-slot="attachment" data-state="done" data-size="default" data-orientation="horizontal">' +
            '<button type="button" data-slot="attachment-trigger" aria-label="Open itinerary.pdf"></button>' +
            '<div data-slot="attachment-content"><span data-slot="attachment-title">itinerary.pdf</span></div>' +
            '<button data-slot="attachment-action" data-variant="ghost" data-size="icon-xs" type="button" aria-label="Remove">x</button></div>',
    );
});

test("mark an uploading attachment busy and fade its title, and color a failed one's description", () => {
    const container = draw(() => (
        <>
            <Attachment>
                <AttachmentTitle>done.pdf</AttachmentTitle>
                <AttachmentDescription>240 KB</AttachmentDescription>
            </Attachment>
            <Attachment state="uploading">
                <AttachmentTitle>tram.jpg</AttachmentTitle>
            </Attachment>
            <Attachment state="error">
                <AttachmentDescription>Too large</AttachmentDescription>
            </Attachment>
        </>
    ));
    const [done, uploading, failed] = [...container.querySelectorAll("[data-slot=attachment]")];
    expect([
        uploading?.getAttribute("aria-busy"),
        done?.getAttribute("aria-busy"),
        classOf(uploading, "attachment-title") === classOf(done, "attachment-title"),
        classOf(failed, "attachment-description") === classOf(done, "attachment-description"),
    ]).toEqual(["true", null, false, false]);
});
