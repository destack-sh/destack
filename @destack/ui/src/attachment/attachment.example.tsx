import { Icon } from "@destack/icon";
import type { JSX } from "@solidjs/web";
import {
    Attachment,
    AttachmentAction,
    AttachmentActions,
    AttachmentContent,
    AttachmentDescription,
    AttachmentGroup,
    AttachmentMedia,
    AttachmentTitle,
    AttachmentTrigger,
} from "./attachment.tsx";

/** Show a note's attachments: a ready itinerary that opens, and a photo still uploading. */
export function AttachmentExample(): JSX.Element {
    return (
        <AttachmentGroup aria-label="Attachments">
            <Attachment>
                <AttachmentTrigger aria-label="Open itinerary.pdf" />
                <AttachmentMedia>
                    <Icon name="file-pdf" />
                </AttachmentMedia>
                <AttachmentContent>
                    <AttachmentTitle>itinerary.pdf</AttachmentTitle>
                    <AttachmentDescription>PDF · 240 KB</AttachmentDescription>
                </AttachmentContent>
                <AttachmentActions>
                    <AttachmentAction aria-label="Remove itinerary.pdf">
                        <Icon name="x" />
                    </AttachmentAction>
                </AttachmentActions>
            </Attachment>
            <Attachment state="uploading">
                <AttachmentMedia variant="image">
                    <img src="/photos/tram.jpg" alt="" />
                </AttachmentMedia>
                <AttachmentContent>
                    <AttachmentTitle>tram.jpg</AttachmentTitle>
                    <AttachmentDescription>Uploading</AttachmentDescription>
                </AttachmentContent>
            </Attachment>
        </AttachmentGroup>
    );
}
