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
        container.style.zIndex = "2147483640";
        document.documentElement.appendChild(container);
    }
    return container;
}
function getHighlightColor(index) {
    var colors = [
        "#FF0000",
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
function highlightElement(element, index, iframe) {
    var container = getHighlightContainer();
    var _a = getHighlightColor(index), base = _a.base, background = _a.background;
    var rect = element.getBoundingClientRect();
    var top = rect.top;
    var left = rect.left;
    if (iframe) {
        var iframeRect = iframe.getBoundingClientRect();
        top += iframeRect.top;
        left += iframeRect.left;
    }
    var overlay = document.createElement("div");
    overlay.style.position = "absolute";
    overlay.style.outline = "2px solid " + base;
    overlay.style.backgroundColor = background;
    overlay.style.pointerEvents = "none";
    overlay.style.boxSizing = "border-box";
    overlay.style.top = top + "px";
    overlay.style.left = left + "px";
    overlay.style.width = rect.width + "px";
    overlay.style.height = rect.height + "px";
    overlay.style.zIndex = "2147483641";
    container.appendChild(overlay);
    var label = document.createElement("div");
    label.className = "bench-highlight-label";
    label.style.position = "absolute";
    label.style.background = base;
    label.style.color = "white";
    label.style.padding = "1px 4px";
    label.style.borderRadius = "4px";
    label.style.fontWeight = "medium";
    label.style.fontSize = "14px";
    label.style.fontFamily = "monospace";
    label.dataset.index = index.toString();
    label.style.zIndex = "2147483642";
    label.textContent = index.toString();
    var labelWidth = 20;
    var labelHeight = 16;
    var labelPadding = 0;
    var viewportMargin = 4;
    var shouldPlaceOutside = rect.width < labelWidth * 3 ||
        rect.height < labelHeight * 2;
    var bounds = {
        top: viewportMargin,
        right: window.innerWidth - viewportMargin,
        bottom: window.innerHeight - viewportMargin,
        left: viewportMargin
    };
    var labelPosition;
    if (!shouldPlaceOutside) {
        var insideTopRight = {
            top: top + labelPadding,
            left: left + rect.width - labelWidth - labelPadding
        };
        if (insideTopRight.top >= bounds.top &&
            insideTopRight.left + labelWidth <= bounds.right) {
            labelPosition = insideTopRight;
        }
    }
    if (!labelPosition) {
        var outsideTopRight = {
            top: top - labelHeight - labelPadding,
            left: left + rect.width - labelPadding - labelWidth
        };
        if (outsideTopRight.top >= bounds.top &&
            outsideTopRight.left + labelWidth <= bounds.right) {
            labelPosition = outsideTopRight;
        }
    }
    if (!labelPosition) {
        var outsideBottomRight = {
            top: top + rect.height + labelPadding,
            left: left + rect.width - labelPadding - labelWidth
        };
        if (outsideBottomRight.top + labelHeight <= bounds.bottom &&
            outsideBottomRight.left + labelWidth <= bounds.right) {
            labelPosition = outsideBottomRight;
        }
    }
    if (!labelPosition) {
        labelPosition = {
            top: Math.min(bounds.bottom - labelHeight, Math.max(bounds.top, top + labelPadding)),
            left: Math.min(bounds.right - labelWidth, Math.max(bounds.left, left + labelPadding))
        };
    }
    label.style.top = labelPosition.top + "px";
    label.style.left = labelPosition.left + "px";
    container.appendChild(label);
    element.setAttribute("bench-highlight-id", "bench-highlight-" + index);
}
function getXPath(element, stopAtBoundary) {
    if (stopAtBoundary === void 0) { stopAtBoundary = true; }
    var segments = [];
    var current = element;
    while (current && current.nodeType === Node.ELEMENT_NODE) {
        if (stopAtBoundary &&
            (current.parentNode instanceof ShadowRoot || current.parentNode instanceof HTMLIFrameElement)) {
            break;
        }
        var index = 0;
        var sibling = current.previousSibling;
        while (sibling) {
            if (sibling.nodeType === Node.ELEMENT_NODE &&
                sibling.nodeName === current.nodeName) {
                index++;
            }
            sibling = sibling.previousSibling;
        }
        var tagName = current.nodeName.toLowerCase();
        segments.unshift(tagName + "[" + (index + 1) + "]");
        current = current.parentNode;
    }
    return segments.join("/");
}
function isElementIncluded(element) {
    return !LEAF_DENY_LIST.has(element.tagName.toLowerCase());
}
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
    if (element.onclick !== null || element.getAttribute("onclick") !== null) {
        return true;
    }
    else if (element.hasAttribute("ng-click") || element.hasAttribute("@click") || element.hasAttribute("v-on:click")) {
        return true;
    }
    if (element.hasAttribute("aria-expanded") ||
        element.hasAttribute("aria-pressed") ||
        element.hasAttribute("aria-selected") ||
        element.hasAttribute("aria-checked")) {
        return true;
    }
    return false;
}
function isElementVisible(element) {
    var style = window.getComputedStyle(element);
    return (element.offsetWidth > 0 &&
        element.offsetHeight > 0 &&
        style.visibility !== "hidden" &&
        style.display !== "none");
}
function isTopElement(element) {
    var doc = element.ownerDocument;
    if (doc !== window.document) {
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
function isTextNodeVisible(textNode) {
    var range = document.createRange();
    range.selectNodeContents(textNode);
    var rect = range.getBoundingClientRect();
    return rect.width !== 0 && rect.height !== 0;
}
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
        var isInteractive = isInteractiveElement(element_1);
        var isVisible = isElementVisible(element_1);
        var isTop = isTopElement(element_1);
        nodeData.isInteractive = isInteractive;
        nodeData.isVisible = isVisible;
        nodeData.isTop = isTop;
        if (highlight && isInteractive && isVisible && isTop) {
            nodeData.index = highlightIndex;
            highlightElement(element_1, highlightIndex, iframe);
            highlightIndex++;
        }
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
function extractDocumentDomTree(highlight) {
    if (highlight === void 0) { highlight = true; }
    var tree = buildDomTree(document.body, highlight, null, 0)[0];
    return tree;
}
function cleanupHighlights() {
    var container = document.getElementById(HIGHLIGHT_CONTAINER_ID);
    if (container) {
        container.remove();
    }
    var highlightedElements = document.querySelectorAll('[bench-highlight-id^="bench-highlight-"]');
    highlightedElements.forEach(function (el) {
        el.removeAttribute('bench-highlight-id');
    });
}
