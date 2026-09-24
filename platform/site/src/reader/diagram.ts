/** Render Mermaid fences and follow the reader theme. */
export function renderDiagrams(article: HTMLElement): () => void {
    // retain source for theme changes and failed renders
    const diagrams = Array.from(
        article.querySelectorAll<HTMLElement>("[data-mermaid]"),
        (element) => ({
            element,
            source: element.textContent!,
        }),
    );
    if (diagrams.length === 0) {
        return () => {};
    }

    // serialize updates and discard results from an unmounted reader
    const preference = window.matchMedia("(prefers-color-scheme: dark)");
    let isDisposed = false;
    let pending = Promise.resolve();
    const update = () => {
        pending = pending
            .then(async () => {
                // load mermaid after the fonts, and stop once the reader is gone
                const { default: mermaid } = await import("mermaid");
                await document.fonts.ready;
                if (isDisposed) {
                    return;
                }

                // use strict SVG labels and the site's reading font
                const theme = document.documentElement.dataset.theme;
                const isDark = theme === "dark" || (theme === undefined && preference.matches);
                const frame = getComputedStyle(diagrams[0].element.parentElement!);
                const body = getComputedStyle(diagrams[0].element);
                const background = themeColor(frame.backgroundColor);
                const ink = themeColor(frame.color);
                const line = themeColor(frame.borderTopColor);
                mermaid.initialize({
                    startOnLoad: false,
                    securityLevel: "strict",
                    suppressErrorRendering: true,
                    theme: "base",
                    look: "classic",
                    layout: "dagre",
                    htmlLabels: false,
                    themeCSS: `.edgeLabel text { paint-order: stroke; stroke: ${background}; stroke-width: 6px; stroke-linejoin: round; }`,
                    themeVariables: {
                        darkMode: isDark,
                        fontFamily: body.fontFamily,
                        fontSize: body.fontSize,
                        primaryColor: background,
                        primaryTextColor: ink,
                        primaryBorderColor: ink,
                        secondaryColor: background,
                        secondaryTextColor: ink,
                        secondaryBorderColor: ink,
                        tertiaryColor: background,
                        tertiaryTextColor: ink,
                        tertiaryBorderColor: line,
                        lineColor: ink,
                        textColor: ink,
                        edgeLabelBackground: background,
                    },
                    flowchart: {
                        htmlLabels: false,
                        curve: "linear",
                        nodeSpacing: 32,
                        rankSpacing: 32,
                        padding: 8,
                        diagramPadding: 4,
                    },
                });

                // preserve each source if Mermaid rejects its diagram
                for (const { element, source } of diagrams) {
                    if (isDisposed) {
                        return;
                    }
                    try {
                        const identifier = `diagram-${crypto.randomUUID()}`;
                        const { svg } = await mermaid.render(identifier, source);
                        if (isDisposed) {
                            return;
                        }
                        element.innerHTML = svg;
                        const drawing = element.querySelector("svg");
                        if (drawing) {
                            drawing.setAttribute("xmlns", "http://www.w3.org/2000/svg");
                            drawing.style.width = `${drawing.viewBox.baseVal.width}px`;
                            drawing.style.maxWidth = "none";
                            drawing.setAttribute("height", String(drawing.viewBox.baseVal.height));
                        }
                        if (drawing && !drawing.hasAttribute("aria-labelledby")) {
                            drawing.setAttribute(
                                "aria-label",
                                element.closest("figure")?.querySelector("figcaption")
                                    ?.textContent ??
                                    element.getAttribute("aria-label") ??
                                    "Diagram",
                            );
                        }
                    } catch (error) {
                        if (!isDisposed) {
                            showError(element, source, error);
                        }
                    }
                }
            })
            .catch((error) => {
                if (isDisposed) {
                    return;
                }
                for (const { element, source } of diagrams) {
                    showError(element, source, error);
                }
            });
    };

    // redraw when the explicit theme or system preference changes
    const observer = new MutationObserver(update);
    observer.observe(document.documentElement, {
        attributes: true,
        attributeFilter: ["data-theme"],
    });
    preference.addEventListener("change", update);
    update();

    return () => {
        isDisposed = true;
        observer.disconnect();
        preference.removeEventListener("change", update);
    };
}

/** Convert computed CSS RGB colors to Mermaid's hexadecimal theme format. */
function themeColor(value: string): string {
    const match = /^rgb\((\d+), (\d+), (\d+)\)$/.exec(value);
    if (!match) {
        throw new Error(`Unsupported diagram theme color: ${value}`);
    }

    return `#${match
        .slice(1)
        .map((component) => Number(component).toString(16).padStart(2, "0"))
        .join("")}`;
}

/** Display the rendering failure alongside the original source. */
function showError(element: HTMLElement, source: string, error: unknown) {
    // log the failure and build the alert and the source listing
    console.error("Could not render diagram", error);
    const message = document.createElement("p");
    message.setAttribute("role", "alert");
    message.textContent = `Could not render diagram: ${error instanceof Error ? error.message : String(error)}`;
    const listing = document.createElement("pre");
    listing.tabIndex = 0;
    listing.textContent = source;

    // show the alert and the source in place of the diagram
    element.replaceChildren(message, listing);
}
