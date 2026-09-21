/// Replace an activated video preview with its embedded player.
export function playVideo(event: MouseEvent) {
    // preserve ordinary navigation through modified clicks
    if (event.button !== 0 || event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) {
        return;
    }
    const preview =
        event.target instanceof Element
            ? event.target.closest<HTMLAnchorElement>("a[data-video-src]")
            : null;
    if (preview == null) {
        return;
    }

    // require the attributes emitted by the Markdown renderer
    const source = preview.dataset.videoSrc;
    const title = preview.dataset.videoTitle;
    if (source == undefined || title == undefined) {
        throw new Error("video preview is missing its source or title");
    }

    // create a player only after explicit activation
    event.preventDefault();
    const player = document.createElement("iframe");
    player.src = source;
    player.title = title;
    player.allow = "autoplay; encrypted-media; picture-in-picture; fullscreen";
    player.allowFullscreen = true;
    player.referrerPolicy = "strict-origin-when-cross-origin";
    preview.replaceWith(player);
    player.focus();
}
