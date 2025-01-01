/**
 * Our in-Browser runtime.
 * Build with `tsc extension.ts --module none --removeComments`.
 */

enum DomNodeType {
    TEXT = 1,
    ELEMENT = 2,
}

/**
 * A DOM node. :DomNode
 */
type DomNode = {
    type: DomNodeType;
    tag?: string;
    id?: number;
    text?: string;
    attributes?: Record<string, string>;
    xpath?: string;
    isInteractive?: boolean;
    isVisible?: boolean;
    isTop?: boolean;
    isShadowRoot?: boolean;
    children: DomNode[];
};

const LABEL_WIDTH = 20;
const LABEL_HEIGHT = 16;
const LABEL_PADDING = 0;
const CELL_SIZE = 50;
const VIEWPORT_MARGIN = 0; // minimum pixels from viewport edge
const HIGHLIGHT_ID_KEY = "benchHighlightId"; // prefix for highlight dataset attribute

/** A cell in the ElementGrid. */
type ELementCell = {
    element: Element;
    rect: DOMRect;
};

function iou(rect1: DOMRect, rect2: DOMRect): number {
    const intersection = this.intersection(rect1, rect2);
    const union = this.union(rect1, rect2);
    return intersection / union;
}

function intersection(rect1: DOMRect, rect2: DOMRect) {
    const left = Math.max(rect1.left, rect2.left);
    const top = Math.max(rect1.top, rect2.top);
    const right = Math.min(rect1.right, rect2.right);
    const bottom = Math.min(rect1.bottom, rect2.bottom);
    const width = right - left;
    const height = bottom - top;
    return width * height;
}

function union(rect1: DOMRect, rect2: DOMRect) {
    const left = Math.min(rect1.left, rect2.left);
    const top = Math.min(rect1.top, rect2.top);
    const right = Math.max(rect1.right, rect2.right);
    const bottom = Math.max(rect1.bottom, rect2.bottom);
    const width = right - left;
    const height = bottom - top;
    return width * height;
}

/** A spatial index for Elements. */
class ElementGrid {
    // 2D map: grid[x][y] -> Array<GridCell>
    private grid: Map<number, Map<number, ELementCell[]>>;

    constructor() {
        this.grid = new Map();
    }

    private cellCoords(x: number, y: number): [number, number] {
        return [Math.floor(x / CELL_SIZE), Math.floor(y / CELL_SIZE)];
    }

    private insertCell(cx: number, cy: number, cellData: ELementCell) {
        if (!this.grid.has(cx)) {
            this.grid.set(cx, new Map());
        }
        const col = this.grid.get(cx)!;
        if (!col.has(cy)) {
            col.set(cy, []);
        }
        col.get(cy)!.push(cellData);
    }

    private removeCell(cx: number, cy: number, element: Element): void {
        const col = this.grid.get(cx);
        if (!col) return;
        const cell = col.get(cy);
        if (!cell) return;
        const index = cell.findIndex((c) => c.element === element);
        if (index !== -1) {
            cell.splice(index, 1);
        }
    }

    add(element: Element, rect: DOMRect): void {
        const [startCx, startCy] = this.cellCoords(rect.left, rect.top);
        const [endCx, endCy] = this.cellCoords(rect.right, rect.bottom);

        for (let cx = startCx; cx <= endCx; cx++) {
            for (let cy = startCy; cy <= endCy; cy++) {
                this.insertCell(cx, cy, { element, rect });
            }
        }
    }

    remove(element: Element, rect: DOMRect): void {
        const [startCx, startCy] = this.cellCoords(rect.left, rect.top);
        const [endCx, endCy] = this.cellCoords(rect.right, rect.bottom);

        for (let cx = startCx; cx <= endCx; cx++) {
            for (let cy = startCy; cy <= endCy; cy++) {
                this.removeCell(cx, cy, element);
            }
        }
    }

    hasIntersection(left: number, top: number, width: number, height: number): boolean {
        const [startCx, startCy] = this.cellCoords(left, top);
        const [endCx, endCy] = this.cellCoords(left + width, top + height);

        // search only the relevant cells
        for (let cx = startCx; cx <= endCx; cx++) {
            const col = this.grid.get(cx);
            if (!col) continue;
            for (let cy = startCy; cy <= endCy; cy++) {
                const cell = col.get(cy);
                if (!cell) continue;
                // check each label in these cells
                for (const { rect } of cell) {
                    // if the bounding boxes overlap...
                    if (left < rect.right && left + width > rect.left && top < rect.bottom && top + height > rect.top) {
                        return true;
                    }
                }
            }
        }
        return false;
    }

    /**
     * Gets the most overlapping element at the given position (if any).
     * Overlap is calculated as intersection over union (IoU).
     **/
    getOverlappingElement(query: DOMRect, minIoU: number): Element | null {
        const [startCx, startCy] = this.cellCoords(query.left, query.top);
        const [endCx, endCy] = this.cellCoords(query.right, query.bottom);
        let bestIoU = 0;
        let bestElement: Element | null = null;
        for (let cx = startCx; cx <= endCx; cx++) {
            for (let cy = startCy; cy <= endCy; cy++) {
                const cell = this.grid.get(cx)?.get(cy);
                if (!cell) continue;
                for (const { rect, element } of cell) {
                    const iouScore = iou(rect, query);
                    if (iouScore > bestIoU) {
                        bestIoU = iouScore;
                        bestElement = element;
                    }
                }
            }
        }
        if (bestIoU < minIoU) return null;
        return bestElement;
    }
}

class HighlightContext {
    root: Element;
    rootRect: DOMRect;
    highlightId: number;
    elementGrid: ElementGrid;
    rectByElement: Map<Element, DOMRect>;
    nodeByElement: Map<Element, DomNode>;
    labelGrid: ElementGrid;

    constructor(root: Element) {
        this.root = root;
        this.rootRect = root.getBoundingClientRect();
        this.highlightId = 0;
        this.labelGrid = new ElementGrid();
        this.elementGrid = new ElementGrid();
        this.rectByElement = new Map();
        this.nodeByElement = new Map();
    }

    /** Adds an element to the context (after positioning!). */
    addElement(element: Element, node: DomNode): void {
        const rect = element.getBoundingClientRect();
        this.elementGrid.add(element, rect);
        this.rectByElement.set(element, rect);
        this.nodeByElement.set(element, node);
    }

    /** Removes an element from the context. */
    removeElement(element: Element): void {
        const rect = this.rectByElement.get(element);
        if (!rect) return;
        this.elementGrid.remove(element, rect);
        this.rectByElement.delete(element);
        this.nodeByElement.delete(element);
    }

    /** Gets the overlapping element at the given position. */
    getNodeAt(rect: DOMRect, minIoU: number): DomNode | null {
        const element = this.elementGrid.getOverlappingElement(rect, minIoU);
        if (!element) return null;
        const node = this.nodeByElement.get(element);
        if (!node) throw new Error("no node found for element");
        return node;
    }

    /** Adds a label to the context (after positioning!). */
    addLabel(label: Element): void {
        const rect = label.getBoundingClientRect();
        this.labelGrid.add(label, rect);
    }

    /** Checks if a label is already at the given position. */
    isLabelAt(left: number, top: number): boolean {
        return this.labelGrid.hasIntersection(left, top, LABEL_WIDTH, LABEL_HEIGHT);
    }
}

function makeHighlightContext(element: Element): HighlightContext {
    return new HighlightContext(element);
}

const LEAF_DENY_LIST = new Set(["svg", "script", "style", "link", "meta"]);
const INTERACTIVE_TAGS = new Set([
    "a",
    "button",
    "details",
    "embed",
    "input",
    "label",
    "menu",
    "menuitem",
    "object",
    "select",
    "textarea",
    "summary",
]);
const INTERACTIVE_ROLES = new Set([
    "button",
    "menu",
    "menuitem",
    "link",
    "checkbox",
    "radio",
    "slider",
    "tab",
    "tabpanel",
    "textbox",
    "combobox",
    "grid",
    "listbox",
    "option",
    "progressbar",
    "scrollbar",
    "searchbox",
    "switch",
    "tree",
    "treeitem",
    "spinbutton",
    "tooltip",
    "a-button-inner",
    "a-dropdown-button",
    "click",
    "menuitemcheckbox",
    "menuitemradio",
    "a-button-text",
    "button-text",
    "button-icon",
    "button-icon-only",
    "button-text-icon-only",
    "dropdown",
    "combobox",
]);
const HIGHLIGHT_CONTAINER_ID = "bench-highlight-container";

/**
 * Creates or returns the highlight container element placed at the top level of the document.
 * This container holds highlight overlays and labels for highlighted elements.
 */
function getHighlightContainer(): HTMLDivElement {
    let container = document.getElementById(HIGHLIGHT_CONTAINER_ID) as HTMLDivElement | null;
    if (!container) {
        container = document.createElement("div");
        container.id = HIGHLIGHT_CONTAINER_ID;
        container.style.position = "fixed";
        container.style.pointerEvents = "none";
        container.style.top = "0";
        container.style.left = "0";
        container.style.width = "100%";
        container.style.height = "100%";
        container.style.zIndex = "2147483640";
        document.documentElement.appendChild(container);
    }
    return container;
}

/**
 * Generates highlight colors based on a given index.
 * It returns a base color and a background color with slight transparency.
 */
function getHighlightColor(index: number): string {
    const colors = [
        "#FF0000", // Pure red
        "#0066FF", // Brighter blue
        "#FF8000", // Vivid orange
        "#9900FF", // Bright purple
        "#00CCCC", // Bright teal
        "#FF1493", // Deep pink
        "#6600CC", // Bright indigo
        "#FF6600", // Bright orange-red
        "#00CC66", // Bright green
        "#FF0033", // Bright crimson
        "#0099FF", // Bright sky blue
    ];
    const base = colors[index % colors.length];
    return base;
}

/**
 * Highlights the given element by drawing an overlay and a label at the element's position.
 * It uses the passed highlight context to distinguish multiple highlighted elements.
 */
function highlightElement(
    element: Element,
    rect: DOMRect,
    context: HighlightContext,
    iframe: HTMLIFrameElement | null,
): void {
    const index = context.highlightId;
    const container = getHighlightContainer();
    const baseColor = getHighlightColor(index);

    // get position
    let top = rect.top;
    let left = rect.left;
    if (iframe) {
        const iframeRect = iframe.getBoundingClientRect();
        top += iframeRect.top;
        left += iframeRect.left;
    }

    // overlay
    const overlay = document.createElement("div");
    overlay.style.position = "absolute";
    const outlineWidth = Math.max(2, Math.min(5, 2 + Math.round(Math.sqrt(rect.width + rect.height) / 10)));
    overlay.style.outline = `${outlineWidth}px solid ${baseColor}CC`;
    overlay.style.outlineOffset = `${-outlineWidth / 2}px`;
    overlay.style.backgroundColor = `${baseColor}33`;
    overlay.style.pointerEvents = "none";
    overlay.style.boxSizing = "border-box";
    overlay.style.top = `${top}px`;
    overlay.style.left = `${left}px`;
    overlay.style.width = `${rect.width}px`;
    overlay.style.height = `${rect.height}px`;
    overlay.style.zIndex = "2147483641";

    // label
    const label = document.createElement("div");
    label.className = "bench-highlight-label";
    label.style.position = "absolute";
    label.style.background = baseColor;
    label.style.color = "white";
    label.style.padding = "1px 4px";
    label.style.borderRadius = "4px";
    label.style.fontWeight = "medium";
    label.style.fontSize = "14px";
    label.style.fontFamily = "monospace";
    label.dataset.index = index.toString();
    label.style.zIndex = "2147483642";
    label.textContent = index.toString();

    // determine if label should be placed outside based on container size
    const shouldPlaceOutside = rect.width < LABEL_WIDTH * 3 || rect.height < LABEL_HEIGHT * 2;

    // calculate viewport bounds with margin
    const bounds = {
        top: VIEWPORT_MARGIN,
        right: window.innerWidth - VIEWPORT_MARGIN,
        bottom: window.innerHeight - VIEWPORT_MARGIN,
        left: VIEWPORT_MARGIN,
    };

    // try positions in clockwise order
    let labelPosition;
    if (!shouldPlaceOutside) {
        // inside top-right
        const insideTopRight = {
            top: top,
            left: left + rect.width - LABEL_WIDTH - LABEL_PADDING,
        };
        if (
            insideTopRight.top >= bounds.top &&
            insideTopRight.left + LABEL_WIDTH <= bounds.right &&
            !context.isLabelAt(insideTopRight.left, insideTopRight.top)
        ) {
            labelPosition = insideTopRight;
        }
    }
    if (!shouldPlaceOutside && !labelPosition) {
        // inside bottom-left
        const insideBottomLeft = {
            top: top + rect.height - LABEL_HEIGHT - LABEL_PADDING,
            left: left + LABEL_PADDING,
        };
        if (
            insideBottomLeft.top >= bounds.top &&
            insideBottomLeft.left + LABEL_WIDTH <= bounds.right &&
            !context.isLabelAt(insideBottomLeft.left, insideBottomLeft.top)
        ) {
            labelPosition = insideBottomLeft;
        }
    }
    if (!labelPosition) {
        // outside top-right
        const outsideTopRight = {
            top: top - LABEL_HEIGHT - LABEL_PADDING,
            left: left + rect.width - LABEL_PADDING - LABEL_WIDTH,
        };
        if (
            outsideTopRight.top >= bounds.top &&
            outsideTopRight.left + LABEL_WIDTH <= bounds.right &&
            !context.isLabelAt(outsideTopRight.left, outsideTopRight.top)
        ) {
            labelPosition = outsideTopRight;
        }
    }
    if (!labelPosition) {
        // outside bottom-right
        const outsideBottomRight = {
            top: top + rect.height + LABEL_PADDING,
            left: left + rect.width - LABEL_PADDING - LABEL_WIDTH,
        };
        if (
            outsideBottomRight.top + LABEL_HEIGHT <= bounds.bottom &&
            outsideBottomRight.left + LABEL_WIDTH <= bounds.right &&
            !context.isLabelAt(outsideBottomRight.left, outsideBottomRight.top)
        ) {
            labelPosition = outsideBottomRight;
        }
    }
    if (!labelPosition) {
        // outside bottom-left
        const outsideBottomLeft = {
            top: top + rect.height + LABEL_PADDING,
            left: left - LABEL_WIDTH - LABEL_PADDING,
        };
        if (
            outsideBottomLeft.top + LABEL_HEIGHT <= bounds.bottom &&
            outsideBottomLeft.left >= bounds.left &&
            !context.isLabelAt(outsideBottomLeft.left, outsideBottomLeft.top)
        ) {
            labelPosition = outsideBottomLeft;
        }
    }
    if (!labelPosition) {
        // fallback: place where it fits best while respecting viewport bounds
        labelPosition = {
            top: Math.min(bounds.bottom - LABEL_HEIGHT, Math.max(bounds.top, top + LABEL_PADDING)),
            left: Math.min(bounds.right - LABEL_WIDTH, Math.max(bounds.left, left + LABEL_PADDING)),
        };
    }
    label.style.top = `${labelPosition.top}px`;
    label.style.left = `${labelPosition.left}px`;

    container.appendChild(overlay);
    container.appendChild(label);
    context.addLabel(label);
    (element as HTMLElement).dataset[HIGHLIGHT_ID_KEY] = index.toString();
}

/**
 * Generates an XPath string for the given element by collecting
 * its ancestors' tag names until the top of the document or a boundary is reached.
 */
function getXPath(element: Element, stopAtBoundary = true): string {
    // build array of segments from bottom to top
    const segments: string[] = [];
    let current: Node | null = element;

    while (current && current.nodeType === Node.ELEMENT_NODE) {
        // stop if we hit a shadow root or an iframe boundary
        if (
            stopAtBoundary &&
            (current.parentNode instanceof ShadowRoot || current.parentNode instanceof HTMLIFrameElement)
        ) {
            break;
        }

        // count how many siblings have the same tag name before this one
        let index = 0;
        let sibling = current.previousSibling;
        while (sibling) {
            if (sibling.nodeType === Node.ELEMENT_NODE && sibling.nodeName === current.nodeName) {
                index++;
            }
            sibling = sibling.previousSibling;
        }

        // build xpath segment
        // (always include index, starting at [1] for the first occurrence)
        const tagName = current.nodeName.toLowerCase();
        segments.unshift(`${tagName}[${index + 1}]`);

        current = current.parentNode;
    }

    // join the segments to form the full xpath
    return segments.join("/");
}

/**
 * Checks if the given element should be accepted into the DOM tree.
 */
function isElementIncluded(element: Element): boolean {
    return !LEAF_DENY_LIST.has(element.tagName.toLowerCase());
}

/**
 * Checks if the given element is considered interactive.
 */
function isInteractiveElement(element: Element): boolean {
    const tagName = element.tagName.toLowerCase();
    const role = element.getAttribute("role");
    const ariaRole = element.getAttribute("aria-role");
    const tabIndex = element.getAttribute("tabindex");

    const hasInteractiveRole =
        INTERACTIVE_TAGS.has(tagName) ||
        (role && INTERACTIVE_ROLES.has(role)) ||
        (ariaRole && INTERACTIVE_ROLES.has(ariaRole)) ||
        (tabIndex !== null && tabIndex !== "-1") ||
        element.getAttribute("data-action") === "a-dropdown-select" ||
        element.getAttribute("data-action") === "a-dropdown-button";

    if (hasInteractiveRole) return true;

    // check simple event-related attributes
    if ((element as HTMLElement).onclick !== null || element.getAttribute("onclick") !== null) {
        return true;
    } else if (element.hasAttribute("ng-click") || element.hasAttribute("@click") || element.hasAttribute("v-on:click")) {
        return true;
    }

    // check aria states
    if (
        element.hasAttribute("aria-expanded") ||
        element.hasAttribute("aria-pressed") ||
        element.hasAttribute("aria-selected") ||
        element.hasAttribute("aria-checked")
    ) {
        return true;
    }

    return false;
}

/**
 * Checks if the given element is visible.
 */
function isElementVisible(element: Element): boolean {
    const style = window.getComputedStyle(element);
    return (
        (element as HTMLElement).offsetWidth > 0 &&
        (element as HTMLElement).offsetHeight > 0 &&
        style.visibility !== "hidden" &&
        style.display !== "none"
    );
}

/**
 * Checks if the given element is at the top of the stacking order at its own center point.
 */
function isTopElement(element: Element): boolean {
    const doc = element.ownerDocument;
    if (doc !== window.document) {
        // inside iframe or different root, assume top there
        return true;
    }

    const shadowRoot = element.getRootNode();
    const rect = element.getBoundingClientRect();
    const x = rect.left + rect.width / 2;
    const y = rect.top + rect.height / 2;

    const context = shadowRoot instanceof ShadowRoot ? shadowRoot : document;
    const topEl = context.elementFromPoint(x, y);
    if (!topEl) return false;

    let current: Element | null = topEl as Element;
    while (current && current !== (context as Document).documentElement) {
        if (current === element) return true;
        current = current.parentElement;
    }
    return false;
}

/**
 * Checks if the given text node is visible.
 */
function isTextNodeVisible(textNode: Text): boolean {
    const range = document.createRange();
    range.selectNodeContents(textNode);
    const rect = range.getBoundingClientRect();
    return rect.width !== 0 && rect.height !== 0;
}

/**
 * Build a DOM node data tree from a given DOM node.
 */
function buildDomTree(
    node: Node,
    highlight: boolean,
    iframe: HTMLIFrameElement | null,
    context: HighlightContext,
): DomNode | null {
    if (node.nodeType === Node.TEXT_NODE) {
        const textContent = node.textContent?.trim();
        if (textContent && isTextNodeVisible(node as Text)) {
            return {
                type: DomNodeType.TEXT,
                text: textContent,
                children: [],
                attributes: {},
            };
        }
        return null;
    }

    if (node.nodeType === Node.ELEMENT_NODE) {
        const element = node as Element;
        if (!isElementIncluded(element)) return null;

        const nodeData: DomNode = {
            type: DomNodeType.ELEMENT,
            tag: element.tagName.toLowerCase(),
            xpath: getXPath(element),
            children: [],
        };

        // attributes
        const attributeNames = element.getAttributeNames?.() || [];
        for (const name of attributeNames) {
            if (nodeData.attributes === undefined) {
                nodeData.attributes = {};
            }
            const attrVal = element.getAttribute(name);
            if (attrVal !== null) {
                nodeData.attributes[name] = attrVal;
            }
        }

        // flags
        const isInteractive = isInteractiveElement(element);
        const isVisible = isElementVisible(element);
        const isTop = isTopElement(element);
        if (isInteractive) nodeData.isInteractive = true;
        if (isVisible) nodeData.isVisible = true;
        if (isTop) nodeData.isTop = true;

        // highlight
        if (highlight && isInteractive && isVisible && isTop) {
            const rect = element.getBoundingClientRect();
            const overlappingNode = context.getNodeAt(rect, 0.8);
            context.addElement(element, nodeData);
            if (overlappingNode == null && iou(rect, context.rootRect) < 0.4) {
                // highlight if not overlapping with anything or large part of screen
                context.highlightId++;
                nodeData.id = context.highlightId;
                highlightElement(element, rect, context, iframe);
            }
        }

        // handle shadow roots
        if (element.shadowRoot) {
            nodeData.isShadowRoot = true;
            const shadowChildren: DomNode[] = [];
            Array.from(element.shadowRoot.childNodes).forEach((child) => {
                const childNode = buildDomTree(child, highlight, iframe, context);
                if (childNode) shadowChildren.push(childNode);
            });
            nodeData.children.push(...shadowChildren);
        }

        // iframes
        if (element.tagName === "IFRAME") {
            try {
                const iframeDoc = (element as HTMLIFrameElement).contentDocument;
                if (iframeDoc && iframeDoc.body) {
                    const iframeChildren: DomNode[] = [];
                    Array.from(iframeDoc.body.childNodes).forEach((child) => {
                        const childNode = buildDomTree(child, highlight, element as HTMLIFrameElement, context);
                        if (childNode) iframeChildren.push(childNode);
                    });
                    nodeData.children.push(...iframeChildren);
                }
            } catch {
                // ignore iframe access errors
            }
        } else {
            const children: DomNode[] = [];
            Array.from(node.childNodes).forEach((child) => {
                const childNode = buildDomTree(child, highlight, iframe, context);
                if (childNode) children.push(childNode);
            });
            nodeData.children.push(...children);
        }

        return nodeData;
    }

    return null;
}

/**
 * Extract the DOM tree from the current document's body.
 */
function extractDocumentDomTree(highlight = true): DomNode | null {
    return buildDomTree(document.body, highlight, null, makeHighlightContext(document.body));
}

/**
 * Clean up any highlights from the DOM.
 */
function cleanupHighlights(scope: "container" | "attribute" | "all" = "all"): void {
    // remove the highlight container and all its contents
    if (scope === "container" || scope === "all") {
        const container = document.getElementById(HIGHLIGHT_CONTAINER_ID);
        if (container) {
            container.remove();
        }
    }
    if (scope === "attribute" || scope === "all") {
        // remove highlight attributes from elements
        const highlightedElements = document.querySelectorAll(`[data-${HIGHLIGHT_ID_KEY}]`);
        highlightedElements.forEach((el) => {
            delete (el as HTMLElement).dataset[HIGHLIGHT_ID_KEY];
        });
    }
}
