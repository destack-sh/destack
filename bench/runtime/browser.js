"use strict";
var LEAF_DENY_LIST = new Set(["svg", "script", "style", "link", "meta"]);
var INTERACTIVE_TAGS = new Set([
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
var INTERACTIVE_ROLES = new Set([
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
var HIGHLIGHT_CONTAINER_ID = "bench-highlight-container";
/**
* Creates or returns the highlight container element placed at the top level of the document.
* This container holds highlight overlays and labels for highlighted elements.
*/
function getHighlightContainer() {
    var container = document.getElementById(HIGHLIGHT_CONTAINER_ID);
    if (!container) {
        container = document.createElement("div");
        container.id = HIGHLIGHT_CONTAINER_ID;
        container.style.position = "fixed";
        container.style.pointerEvents = "none";
        container.style.top = "0";
        container.style.left = "0";
        container.style.width = "100%";
        container.style.height = "100%";
        container.style.zIndex = "2147483647";
        document.documentElement.appendChild(container);
    }
    return container;
}
/**
* Generates highlight colors based on a given index.
* It returns a base color and a background color with slight transparency.
*/
function getHighlightColor(index) {
    var colors = [
        "#FF0000",
        "#00FF00",
        "#0000FF",
        "#FFA500",
        "#800080",
        "#008080",
        "#FF69B4",
        "#4B0082",
        "#FF4500",
        "#2E8B57",
        "#DC143C",
        "#4682B4",
    ];
    var base = colors[index % colors.length];
    return { base: base, background: base + "1A" };
}
/**
* Highlights the given element by drawing an overlay and a label at the element's position.
* It uses the passed highlight index to distinguish multiple highlighted elements.
*/
function highlightElement(element, index, iframe) {
    var container = getHighlightContainer();
    var _a = getHighlightColor(index), base = _a.base, background = _a.background;
    // position calculation
    var rect = element.getBoundingClientRect();
    var top = rect.top;
    var left = rect.left;
    if (iframe) {
        var iframeRect = iframe.getBoundingClientRect();
        top += iframeRect.top;
        left += iframeRect.left;
    }
    // overlay creation
    var overlay = document.createElement("div");
    overlay.style.position = "absolute";
    overlay.style.border = "2px solid " + base;
    overlay.style.backgroundColor = background;
    overlay.style.pointerEvents = "none";
    overlay.style.boxSizing = "border-box";
    overlay.style.top = top + "px";
    overlay.style.left = left + "px";
    overlay.style.width = rect.width + "px";
    overlay.style.height = rect.height + "px";
    container.appendChild(overlay);
    // label creation
    var label = document.createElement("div");
    label.className = "bench-highlight-label";
    label.style.position = "absolute";
    label.style.background = base;
    label.style.color = "white";
    label.style.padding = "1px 4px";
    label.style.borderRadius = "4px";
    label.style.fontSize = Math.min(12, Math.max(8, rect.height / 2)) + "px";
    label.dataset.index = index.toString();
    label.textContent = index.toString();
    var labelWidth = 20;
    var labelHeight = 16;
    var labelTop = top + 2;
    var labelLeft = left + rect.width - labelWidth - 2;
    // if element is too small, adjust label placement
    if (rect.width < labelWidth + 4 || rect.height < labelHeight + 4) {
        labelTop = top - labelHeight - 2;
        labelLeft = left + rect.width - labelWidth;
    }
    // ensure label stays within viewport
    if (labelTop < 0)
        labelTop = top + 2;
    if (labelLeft < 0)
        labelLeft = left + 2;
    if (labelLeft + labelWidth > window.innerWidth) {
        labelLeft = left + rect.width - labelWidth - 2;
    }
    label.style.top = labelTop + "px";
    label.style.left = labelLeft + "px";
    container.appendChild(label);
    element.setAttribute("bench-highlight-id", "bench-highlight-" + index);
}
/**
* Generates an XPath string for the given element by collecting
* its ancestors' tag names until the top of the document or a boundary is reached.
*/
function getXPath(element, stopAtBoundary) {
    if (stopAtBoundary === void 0) { stopAtBoundary = true; }
    // build array of segments from bottom to top
    var segments = [];
    var current = element;
    while (current && current.nodeType === Node.ELEMENT_NODE) {
        // stop if we hit a shadow root or an iframe boundary
        if (stopAtBoundary &&
            (current.parentNode instanceof ShadowRoot || current.parentNode instanceof HTMLIFrameElement)) {
            break;
        }
        // count how many siblings have the same tag name before this one
        var index = 0;
        var sibling = current.previousSibling;
        while (sibling) {
            if (sibling.nodeType === Node.ELEMENT_NODE &&
                sibling.nodeName === current.nodeName) {
                index++;
            }
            sibling = sibling.previousSibling;
        }
        // build xpath segment
        // (always include index, starting at [1] for the first occurrence)
        var tagName = current.nodeName.toLowerCase();
        segments.unshift(tagName + "[" + (index + 1) + "]");
        current = current.parentNode;
    }
    // join the segments to form the full xpath
    return segments.join("/");
}
/**
* Checks if the given element should be accepted into the DOM tree.
*/
function isElementIncluded(element) {
    return !LEAF_DENY_LIST.has(element.tagName.toLowerCase());
}
/**
* Checks if the given element is considered interactive.
*/
function isInteractiveElement(element) {
    var tagName = element.tagName.toLowerCase();
    var role = element.getAttribute("role");
    var ariaRole = element.getAttribute("aria-role");
    var tabIndex = element.getAttribute("tabindex");
    var hasInteractiveRole = INTERACTIVE_TAGS.has(tagName) ||
        (role && INTERACTIVE_ROLES.has(role)) ||
        (ariaRole && INTERACTIVE_ROLES.has(ariaRole)) ||
        (tabIndex !== null && tabIndex !== "-1") ||
        element.getAttribute("data-action") === "a-dropdown-select" ||
        element.getAttribute("data-action") === "a-dropdown-button";
    if (hasInteractiveRole)
        return true;
    // check simple event-related attributes
    if (element.onclick !== null || element.getAttribute("onclick") !== null)
        return true;
    if (element.hasAttribute("ng-click") || element.hasAttribute("@click") || element.hasAttribute("v-on:click"))
        return true;
    // check aria states
    if (element.hasAttribute("aria-expanded") ||
        element.hasAttribute("aria-pressed") ||
        element.hasAttribute("aria-selected") ||
        element.hasAttribute("aria-checked")) {
        return true;
    }
    return false;
}
/**
* Checks if the given element is visible.
*/
function isElementVisible(element) {
    var style = window.getComputedStyle(element);
    return (element.offsetWidth > 0 &&
        element.offsetHeight > 0 &&
        style.visibility !== "hidden" &&
        style.display !== "none");
}
/**
* Checks if the given element is at the top of the stacking order at its own center point.
*/
function isTopElement(element) {
    var doc = element.ownerDocument;
    if (doc !== window.document) {
        // inside iframe or different root, assume top there
        return true;
    }
    var shadowRoot = element.getRootNode();
    var rect = element.getBoundingClientRect();
    var x = rect.left + rect.width / 2;
    var y = rect.top + rect.height / 2;
    var context = shadowRoot instanceof ShadowRoot ? shadowRoot : document;
    var topEl = context.elementFromPoint(x, y);
    if (!topEl)
        return false;
    var current = topEl;
    while (current && current !== context.documentElement) {
        if (current === element)
            return true;
        current = current.parentElement;
    }
    return false;
}
/**
* Checks if the given text node is visible.
*/
function isTextNodeVisible(textNode) {
    var range = document.createRange();
    range.selectNodeContents(textNode);
    var rect = range.getBoundingClientRect();
    return rect.width !== 0 && rect.height !== 0;
}
/**
* Build a DOM node data tree from a given DOM node.
*/
function buildDomTree(node, highlight, iframe, highlightIndex) {
    var _a, _b, _c;
    var _d, _e;
    if (node.nodeType === Node.TEXT_NODE) {
        var textContent = (_d = node.textContent) === null || _d === void 0 ? void 0 : _d.trim();
        if (textContent && isTextNodeVisible(node)) {
            return [{
                    type: "TEXT_NODE",
                    text: textContent,
                    children: [],
                    attributes: {}
                }, highlightIndex];
        }
        return [null, highlightIndex];
    }
    if (node.nodeType === Node.ELEMENT_NODE) {
        var element_1 = node;
        if (!isElementIncluded(element_1))
            return [null, highlightIndex];
        var nodeData = {
            type: "ELEMENT_NODE",
            tagName: element_1.tagName.toLowerCase(),
            xpath: getXPath(element_1),
            children: []
        };
        // attributes
        var attributeNames = ((_e = element_1.getAttributeNames) === null || _e === void 0 ? void 0 : _e.call(element_1)) || [];
        for (var _i = 0, attributeNames_1 = attributeNames; _i < attributeNames_1.length; _i++) {
            var name_1 = attributeNames_1[_i];
            if (nodeData.attributes === undefined) {
                nodeData.attributes = {};
            }
            var attrVal = element_1.getAttribute(name_1);
            if (attrVal !== null) {
                nodeData.attributes[name_1] = attrVal;
            }
        }
        // flags
        var isInteractive = isInteractiveElement(element_1);
        var isVisible = isElementVisible(element_1);
        var isTop = isTopElement(element_1);
        nodeData.isInteractive = isInteractive;
        nodeData.isVisible = isVisible;
        nodeData.isTop = isTop;
        // highlight
        if (highlight && isInteractive && isVisible && isTop) {
            nodeData.index = highlightIndex;
            highlightElement(element_1, highlightIndex, iframe);
            highlightIndex++;
        }
        // handle shadow roots
        if (element_1.shadowRoot) {
            nodeData.isShadowRoot = true;
            var currentIndex_1 = highlightIndex;
            var shadowChildren_1 = [];
            Array.from(element_1.shadowRoot.childNodes).forEach(function (child) {
                var _a = buildDomTree(child, highlight, iframe, currentIndex_1), childNode = _a[0], newIndex = _a[1];
                if (childNode)
                    shadowChildren_1.push(childNode);
                currentIndex_1 = newIndex;
            });
            (_a = nodeData.children).push.apply(_a, shadowChildren_1);
            highlightIndex = currentIndex_1;
        }
        // iframes
        if (element_1.tagName === "IFRAME") {
            try {
                var iframeDoc = element_1.contentDocument;
                if (iframeDoc && iframeDoc.body) {
                    var currentIndex_2 = highlightIndex;
                    var iframeChildren_1 = [];
                    Array.from(iframeDoc.body.childNodes).forEach(function (child) {
                        var _a = buildDomTree(child, highlight, element_1, currentIndex_2), childNode = _a[0], newIndex = _a[1];
                        if (childNode)
                            iframeChildren_1.push(childNode);
                        currentIndex_2 = newIndex;
                    });
                    (_b = nodeData.children).push.apply(_b, iframeChildren_1);
                    highlightIndex = currentIndex_2;
                }
            }
            catch (_f) {
                // ignore iframe access errors
            }
        }
        else {
            var currentIndex_3 = highlightIndex;
            var children_1 = [];
            Array.from(node.childNodes).forEach(function (child) {
                var _a = buildDomTree(child, highlight, iframe, currentIndex_3), childNode = _a[0], newIndex = _a[1];
                if (childNode)
                    children_1.push(childNode);
                currentIndex_3 = newIndex;
            });
            (_c = nodeData.children).push.apply(_c, children_1);
            highlightIndex = currentIndex_3;
        }
        return [nodeData, highlightIndex];
    }
    return [null, highlightIndex];
}
/**
* Extract the DOM tree from the current document's body.
*/
function extractDocumentDomTree(highlight) {
    if (highlight === void 0) { highlight = true; }
    var tree = buildDomTree(document.body, highlight, null, 0)[0];
    return tree;
}