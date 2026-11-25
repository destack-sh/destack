import xIcon from "@destack/icon/phosphor/x";
import { defineExample } from "@destack/package/declare";
import { Button } from "../button/index.ts";
import { Icon } from "../icon/index.ts";
import {
    FloatingPanel,
    FloatingPanelBody,
    FloatingPanelClose,
    FloatingPanelContent,
    FloatingPanelHeader,
    FloatingPanelTitle,
    FloatingPanelTrigger,
} from "./index.ts";

/** A panel of notes beside the page. */
export const floatingPanelNotes = defineExample({
    of: FloatingPanel,
    name: "notes",
    description: "a panel of notes, which moves by its header and resizes by its sides",
    render: () => (
        <FloatingPanel>
            <FloatingPanelTrigger render={(part) => <Button {...part} variant="outline" />}>
                Notes
            </FloatingPanelTrigger>
            <FloatingPanelContent>
                <FloatingPanelHeader>
                    <FloatingPanelTitle>Notes</FloatingPanelTitle>
                    <FloatingPanelClose
                        aria-label="Close notes"
                        render={(part) => (
                            <Button {...part} variant="ghost" size="icon-sm">
                                <Icon icon={xIcon} />
                            </Button>
                        )}
                    />
                </FloatingPanelHeader>
                <FloatingPanelBody>Ask Grace about the launch date.</FloatingPanelBody>
            </FloatingPanelContent>
        </FloatingPanel>
    ),
});
