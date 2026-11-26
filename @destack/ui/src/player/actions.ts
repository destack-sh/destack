import { t } from "@destack/locale";
import { defineAction } from "@destack/object";
import { schema } from "@destack/schema";

/** Play a film or a sound, or pause it. */
export const togglePlay = defineAction({
    name: "player.togglePlay",
    title: t.context("action")`Play`,
    category: t.context("title")`Player`,
    keybinding: ["space", "k"],
});

/** Go back ten seconds. */
export const skipBack = defineAction({
    name: "player.skipBack",
    title: t`Back 10 seconds`,
    category: t.context("title")`Player`,
    keybinding: ["j", "left"],
});

/** Go forward ten seconds. */
export const skipForward = defineAction({
    name: "player.skipForward",
    title: t`Forward 10 seconds`,
    category: t.context("title")`Player`,
    keybinding: ["l", "right"],
});

/** Go to a time in seconds. */
export const seekTo = defineAction({
    name: "player.seekTo",
    title: t`Go to time`,
    category: t.context("title")`Player`,
    input: { time: schema.number().nonnegative() },
});

/** Mute the sound, or turn it on again. */
export const toggleMute = defineAction({
    name: "player.toggleMute",
    title: t.context("action")`Mute`,
    category: t.context("title")`Player`,
    keybinding: "m",
});

/** Turn the sound up. */
export const volumeUp = defineAction({
    name: "player.volumeUp",
    title: t.context("action")`Volume up`,
    category: t.context("title")`Player`,
    keybinding: "up",
});

/** Turn the sound down. */
export const volumeDown = defineAction({
    name: "player.volumeDown",
    title: t.context("action")`Volume down`,
    category: t.context("title")`Player`,
    keybinding: "down",
});

/** Show captions, or hide them. */
export const toggleCaptions = defineAction({
    name: "player.toggleCaptions",
    title: t.context("action")`Captions`,
    category: t.context("title")`Player`,
    keybinding: "c",
});

/** Show a film over the whole screen, or back in its place. */
export const toggleFullscreen = defineAction({
    name: "player.toggleFullscreen",
    title: t.context("action")`Full screen`,
    category: t.context("title")`Player`,
    keybinding: "f",
});

/** Show a film in a window over others, or back in its place. */
export const togglePictureInPicture = defineAction({
    name: "player.togglePictureInPicture",
    title: t`Picture in picture`,
    category: t.context("title")`Player`,
    keybinding: "p",
});

/** Play what comes before. */
export const previousTrack = defineAction({
    name: "player.previous",
    title: t.context("action")`Previous`,
    category: t.context("title")`Player`,
    keybinding: "shift+p",
});

/** Play what comes after. */
export const nextTrack = defineAction({
    name: "player.next",
    title: t.context("action")`Next`,
    category: t.context("title")`Player`,
    keybinding: "shift+n",
});
