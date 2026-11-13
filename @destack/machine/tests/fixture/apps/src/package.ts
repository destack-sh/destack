import { t } from "@destack/locale";
import { definePackage } from "@destack/object";
import { file, shape } from "./canvas.ts";
import { channel, message, presence } from "./chat.ts";
import { page, workspace } from "./pages.ts";
import { comment, issue, team } from "./tracker.ts";

/** The apps the product benchmarks run: a tracker, pages, a canvas and a chat. */
export default definePackage({
    title: t`Apps`,
    description: t`List the apps of a space.`,
    icon: "squares-four",
    color: "blue",
    objects: [team, issue, comment, workspace, page, file, shape, channel, message, presence],
});
