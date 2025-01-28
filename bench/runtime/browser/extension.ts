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
    type?: DomNodeType;
    id?: string;
    tag?: string;
    text?: string;
    attributes?: Record<string, string>;
    xpath?: string;
    isInteractive?: boolean;
    isVisible?: boolean;
    isTop?: boolean;
    isShadowRoot?: boolean;
    isHighlighted?: boolean;
    children?: DomNode[];
    isFocused?: boolean;
};

const LABEL_WIDTH = 20;
const LABEL_HEIGHT = 16;
const LABEL_PADDING = 0;
const CELL_SIZE = 50;
const VIEWPORT_MARGIN = 0; // minimum pixels from viewport edge
const HIGHLIGHT_ID_KEY = "benchHighlightId"; // prefix for highlight dataset attribute
const FOCUS_HIGHLIGHT_CLASS = "bench-focus-highlight";
const REGULAR_HIGHLIGHT_CLASS = "bench-regular-highlight";
const LABEL_CLASS = "bench-highlight-label";

/** A cell in the ElementGrid. */
type ELementCell = {
    element: Element;
    rect: DOMRect;
};

function iou(rect1: DOMRect, rect2: DOMRect): number {
    const intersectionVal = intersection(rect1, rect2);
    const unionVal = union(rect1, rect2);
    if (unionVal === 0) return 0;
    return intersectionVal / unionVal;
}

function intersection(rect1: DOMRect, rect2: DOMRect) {
    const left = Math.max(rect1.left, rect2.left);
    const top = Math.max(rect1.top, rect2.top);
    const right = Math.min(rect1.right, rect2.right);
    const bottom = Math.min(rect1.bottom, rect2.bottom);
    const width = Math.max(0, right - left);
    const height = Math.max(0, bottom - top);
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
    focusedElement: Element | null;

    constructor(root: Element, focusedElement: Element | null) {
        this.root = root;
        this.rootRect = root.getBoundingClientRect();
        this.highlightId = 0;
        this.labelGrid = new ElementGrid();
        this.elementGrid = new ElementGrid();
        this.rectByElement = new Map();
        this.nodeByElement = new Map();
        this.focusedElement = focusedElement;
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

function makeHighlightContext(element: Element, focusedElement: Element | null): HighlightContext {
    return new HighlightContext(element, focusedElement);
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
const ATTRIBUTE_WHITELIST = new Set(["href", "src", "alt", "title", "placeholder", "value", "aria-label"]);
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
    isFocused: boolean
): void {
    const index = context.highlightId;
    const container = getHighlightContainer();
    const baseColor = isFocused ? "#FFFF00CC" : getHighlightColor(index);

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
    const outlineWidth = Math.max(2, Math.min(4, 2 + Math.round(Math.sqrt(rect.width + rect.height) / 10)));
    if (isFocused) {
        overlay.classList.add(FOCUS_HIGHLIGHT_CLASS);
        overlay.style.outline = `${outlineWidth + 1}px dashed #FFFF00CC`;
        overlay.style.backgroundColor = `#FFFF0055`;
    } else {
        overlay.classList.add(REGULAR_HIGHLIGHT_CLASS);
        overlay.style.outline = `${outlineWidth}px solid ${baseColor}CC`;
        overlay.style.backgroundColor = `${baseColor}33`;
    }
    overlay.style.outlineOffset = `${-outlineWidth / 2}px`;
    overlay.style.pointerEvents = "none";
    overlay.style.boxSizing = "border-box";
    overlay.style.top = `${top}px`;
    overlay.style.left = `${left}px`;
    overlay.style.width = `${rect.width}px`;
    overlay.style.height = `${rect.height}px`;
    overlay.style.zIndex = "2147483641";

    // label
    const label = document.createElement("div");
    label.className = LABEL_CLASS;
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
function isElementInteractive(element: Element): boolean {
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
        style.display !== "none" &&
        !(element.hasAttribute("hidden") || element.hasAttribute("aria-hidden"))
    );
}

/**
 * Checks if the given element is at the top of the stacking order at its own center point.
 */
function isElementTop(element: Element): boolean {
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

const MAX_TEXT_LENGTH = 300;

/** Trims the text to a maximum length. */
function trimText(text: string | null | undefined): string | undefined {
    if (!text) return undefined;
    text = text.trim();
    if (text.length === 0) return undefined;
    if (text.length <= MAX_TEXT_LENGTH) return text;
    const half = Math.floor(MAX_TEXT_LENGTH / 2);
    return text.slice(0, half) + "..." + text.slice(-half);
}

/**
 * Build a DOM node data tree from a given DOM node.
 */
function extract(
    node: Node,
    highlight: boolean,
    iframe: HTMLIFrameElement | null,
    context: HighlightContext,
): DomNode | null {
    if (node.nodeType === Node.TEXT_NODE) {
        const textContent = trimText(node.textContent);
        if (textContent && isTextNodeVisible(node as Text)) {
            return { type: DomNodeType.TEXT, text: textContent };
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
            text: trimText(element.textContent),
        };

        // attributes
        const attributeNames = element.getAttributeNames?.() || [];
        for (const name of attributeNames) {
            if (nodeData.attributes === undefined) {
                nodeData.attributes = {};
            }
            const attrVal = element.getAttribute(name);
            if (attrVal !== null && ATTRIBUTE_WHITELIST.has(name)) {
                nodeData.attributes[name] = attrVal;
            }
        }

        // flags
        const isInteractive = isElementInteractive(element);
        const isVisible = isElementVisible(element);
        const isTop = isElementTop(element);
        const isFocusedElement = element === context.focusedElement;
        if (isInteractive) nodeData.isInteractive = true;
        if (isVisible) nodeData.isVisible = true;
        if (isTop) nodeData.isTop = true;
        if (isFocusedElement) nodeData.isFocused = true;

        // highlight
        if (highlight && isInteractive && isVisible && isTop) {
            const rect = element.getBoundingClientRect();
            const overlappingNode = context.getNodeAt(rect, 0.8);
            context.addElement(element, nodeData);
            if (overlappingNode == null && iou(rect, context.rootRect) < 0.4) {
                // highlight if not overlapping with anything or large part of screen
                context.highlightId++;
                nodeData.id = context.highlightId.toString();
                highlightElement(element, rect, context, iframe, isFocusedElement);
                nodeData.isHighlighted = true;
            }
        }

        // handle shadow roots
        if (element.shadowRoot) {
            nodeData.isShadowRoot = true;
            const shadowChildren: DomNode[] = [];
            Array.from(element.shadowRoot.childNodes).forEach((child) => {
                const childNode = extract(child, highlight, iframe, context);
                if (childNode) shadowChildren.push(childNode);
            });
            if (nodeData.children === undefined) {
                nodeData.children = [];
            }
            nodeData.children.push(...shadowChildren);
        }

        // iframes
        if (element.tagName === "IFRAME") {
            try {
                const iframeDoc = (element as HTMLIFrameElement).contentDocument;
                if (iframeDoc && iframeDoc.body) {
                    const iframeChildren: DomNode[] = [];
                    Array.from(iframeDoc.body.childNodes).forEach((child) => {
                        const childNode = extract(child, highlight, element as HTMLIFrameElement, context);
                        if (childNode) iframeChildren.push(childNode);
                    });
                    if (nodeData.children === undefined) {
                        nodeData.children = [];
                    }
                    nodeData.children.push(...iframeChildren);
                }
            } catch {
                // ignore iframe access errors
            }
        } else {
            const children: DomNode[] = [];
            Array.from(node.childNodes).forEach((child) => {
                const childNode = extract(child, highlight, iframe, context);
                if (childNode) children.push(childNode);
            });
            if (nodeData.children === undefined) {
                nodeData.children = [];
            }
            nodeData.children.push(...children);
        }

        return nodeData;
    }

    return null;
}

/**
 * Adds highlights and extract the DOM tree from the current document's body.
 */
function highlight(): DomNode[] {
    const focusedElement = document.activeElement as Element | null;
    const root = extract(document.body, true, null, makeHighlightContext(document.body, focusedElement));
    // collect all interactive nodes into list
    const interactiveNodes: DomNode[] = [];
    function walk(node: DomNode): void {
        if (node.isHighlighted) {
            const miniNode: DomNode = { id: node.id, tag: node.tag, isFocused: node.isFocused };
            if (node.text) {
                miniNode.text = node.text;
            }
            if (node.attributes && Object.keys(node.attributes).length > 0) {
                miniNode.attributes = node.attributes;
            }
            interactiveNodes.push(miniNode);
        }
        if (node.children) {
            for (const child of node.children) {
                walk(child);
            }
        }
    }
    if (root) {
        walk(root);
    }
    return interactiveNodes;
}

/**
 * Clean up any highlights from the DOM.
 */
function cleanup(scope: "container" | "attribute" | "all" = "all"): void {
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
        // remove highlight classes (regular and focus)
        const highlightedOverlays = document.querySelectorAll(`.${REGULAR_HIGHLIGHT_CLASS}, .${FOCUS_HIGHLIGHT_CLASS}`);
        highlightedOverlays.forEach(el => {
            el.remove();
        });
        const highlightLabels = document.querySelectorAll(`.${LABEL_CLASS}`);
        highlightLabels.forEach(el => {
            el.remove();
        });
    }
}