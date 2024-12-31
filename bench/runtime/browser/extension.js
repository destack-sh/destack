var DomNodeType;
(function (DomNodeType) {
    DomNodeType[DomNodeType["TEXT"] = 1] = "TEXT";
    DomNodeType[DomNodeType["ELEMENT"] = 2] = "ELEMENT";
})(DomNodeType || (DomNodeType = {}));
var LABEL_WIDTH = 20;
var LABEL_HEIGHT = 16;
var LABEL_PADDING = 0;
var VIEWPORT_MARGIN = 4;
var HIGHLIGHT_ID_KEY = "benchHighlightId";
var HighlightContext = (function () {
    function HighlightContext(root) {
        this.root = root;
        this.highlightId = 0;
        this.labels = [];
        this.boundingRectByElement = new Map();
    }
    HighlightContext.prototype.addLabel = function (label) {
        this.labels.push(label);
        this.boundingRectByElement.set(label, label.getBoundingClientRect());
    };
    HighlightContext.prototype.isLabelAt = function (left, top) {
        var _this = this;
        return this.labels.some(function (label) {
            var rect = _this.boundingRectByElement.get(label);
            return ((left >= rect.left && left <= rect.right &&
                top >= rect.top && top <= rect.bottom) ||
                (left + LABEL_WIDTH >= rect.left && left + LABEL_WIDTH <= rect.right &&
                    top >= rect.top && top <= rect.bottom) ||
                (left >= rect.left && left <= rect.right &&
                    top + LABEL_HEIGHT >= rect.top && top + LABEL_HEIGHT <= rect.bottom) ||
                (left + LABEL_WIDTH >= rect.left && left + LABEL_WIDTH <= rect.right &&
                    top + LABEL_HEIGHT >= rect.top && top + LABEL_HEIGHT <= rect.bottom));
        });
    };
    return HighlightContext;
}());
function makeHighlightContext(element) {
    return new HighlightContext(element);
}
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
        "#0066FF",
        "#FF8000",
        "#9900FF",
        "#00CCCC",
        "#FF1493",
        "#6600CC",
        "#FF6600",
        "#00CC66",
        "#FF0033",
        "#0099FF",
    ];
    var base = colors[index % colors.length];
    return { base: base, background: base + "1A" };
}
function highlightElement(element, context, iframe) {
    var index = context.highlightId;
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
    var outlineWidth = Math.max(2, Math.min(5, 2 + Math.round(Math.sqrt(rect.width + rect.height) / 10)));
    overlay.style.outline = outlineWidth + "px solid " + base;
    overlay.style.backgroundColor = background;
    overlay.style.pointerEvents = "none";
    overlay.style.boxSizing = "border-box";
    overlay.style.top = top + "px";
    overlay.style.left = left + "px";
    overlay.style.width = rect.width + "px";
    overlay.style.height = rect.height + "px";
    overlay.style.zIndex = "2147483641";
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
    var shouldPlaceOutside = rect.width < LABEL_WIDTH * 3 ||
        rect.height < LABEL_HEIGHT * 2;
    var bounds = {
        top: VIEWPORT_MARGIN,
        right: window.innerWidth - VIEWPORT_MARGIN,
        bottom: window.innerHeight - VIEWPORT_MARGIN,
        left: VIEWPORT_MARGIN
    };
    var labelPosition;
    if (!shouldPlaceOutside) {
        var insideTopRight = {
            top: top - LABEL_PADDING - LABEL_HEIGHT,
            left: left + rect.width - LABEL_WIDTH - LABEL_PADDING
        };
        if (insideTopRight.top >= bounds.top &&
            insideTopRight.left + LABEL_WIDTH <= bounds.right && !context.isLabelAt(insideTopRight.left, insideTopRight.top)) {
            labelPosition = insideTopRight;
        }
    }
    if (!shouldPlaceOutside && !labelPosition) {
        var insideBottomLeft = {
            top: top + rect.height - LABEL_HEIGHT - LABEL_PADDING,
            left: left - LABEL_WIDTH - LABEL_PADDING
        };
        if (insideBottomLeft.top >= bounds.top &&
            insideBottomLeft.left + LABEL_WIDTH <= bounds.right && !context.isLabelAt(insideBottomLeft.left, insideBottomLeft.top)) {
            labelPosition = insideBottomLeft;
        }
    }
    if (!labelPosition) {
        var outsideTopRight = {
            top: top - LABEL_HEIGHT - LABEL_PADDING,
            left: left + rect.width - LABEL_PADDING - LABEL_WIDTH
        };
        if (outsideTopRight.top >= bounds.top &&
            outsideTopRight.left + LABEL_WIDTH <= bounds.right && !context.isLabelAt(outsideTopRight.left, outsideTopRight.top)) {
            labelPosition = outsideTopRight;
        }
    }
    if (!labelPosition) {
        var outsideBottomRight = {
            top: top + rect.height + LABEL_PADDING,
            left: left + rect.width - LABEL_PADDING - LABEL_WIDTH
        };
        if (outsideBottomRight.top + LABEL_HEIGHT <= bounds.bottom &&
            outsideBottomRight.left + LABEL_WIDTH <= bounds.right && !context.isLabelAt(outsideBottomRight.left, outsideBottomRight.top)) {
            labelPosition = outsideBottomRight;
        }
    }
    if (!labelPosition) {
        var outsideBottomLeft = {
            top: top + rect.height + LABEL_PADDING,
            left: left - LABEL_WIDTH - LABEL_PADDING
        };
        if (outsideBottomLeft.top + LABEL_HEIGHT <= bounds.bottom &&
            outsideBottomLeft.left >= bounds.left && !context.isLabelAt(outsideBottomLeft.left, outsideBottomLeft.top)) {
            labelPosition = outsideBottomLeft;
        }
    }
    if (!labelPosition) {
        labelPosition = {
            top: Math.min(bounds.bottom - LABEL_HEIGHT, Math.max(bounds.top, top + LABEL_PADDING)),
            left: Math.min(bounds.right - LABEL_WIDTH, Math.max(bounds.left, left + LABEL_PADDING))
        };
    }
    label.style.top = labelPosition.top + "px";
    label.style.left = labelPosition.left + "px";
    container.appendChild(overlay);
    container.appendChild(label);
    context.addLabel(label);
    element.dataset[HIGHLIGHT_ID_KEY] = index.toString();
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
function buildDomTree(node, highlight, iframe, context) {
    var _a, _b, _c;
    var _d, _e;
    if (node.nodeType === Node.TEXT_NODE) {
        var textContent = (_d = node.textContent) === null || _d === void 0 ? void 0 : _d.trim();
        if (textContent && isTextNodeVisible(node)) {
            return {
                type: DomNodeType.TEXT,
                text: textContent,
                children: [],
                attributes: {}
            };
        }
        return null;
    }
    if (node.nodeType === Node.ELEMENT_NODE) {
        var element_1 = node;
        if (!isElementIncluded(element_1))
            return null;
        var nodeData = {
            type: DomNodeType.ELEMENT,
            tag: element_1.tagName.toLowerCase(),
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
            context.highlightId++;
            nodeData.id = context.highlightId;
            highlightElement(element_1, context, iframe);
        }
        if (element_1.shadowRoot) {
            nodeData.isShadowRoot = true;
            var shadowChildren_1 = [];
            Array.from(element_1.shadowRoot.childNodes).forEach(function (child) {
                var childNode = buildDomTree(child, highlight, iframe, context);
                if (childNode)
                    shadowChildren_1.push(childNode);
            });
            (_a = nodeData.children).push.apply(_a, shadowChildren_1);
        }
        if (element_1.tagName === "IFRAME") {
            try {
                var iframeDoc = element_1.contentDocument;
                if (iframeDoc && iframeDoc.body) {
                    var iframeChildren_1 = [];
                    Array.from(iframeDoc.body.childNodes).forEach(function (child) {
                        var childNode = buildDomTree(child, highlight, element_1, context);
                        if (childNode)
                            iframeChildren_1.push(childNode);
                    });
                    (_b = nodeData.children).push.apply(_b, iframeChildren_1);
                }
            }
            catch (_f) {
            }
        }
        else {
            var children_1 = [];
            Array.from(node.childNodes).forEach(function (child) {
                var childNode = buildDomTree(child, highlight, iframe, context);
                if (childNode)
                    children_1.push(childNode);
            });
            (_c = nodeData.children).push.apply(_c, children_1);
        }
        return nodeData;
    }
    return null;
}
function extractDocumentDomTree(highlight) {
    if (highlight === void 0) { highlight = true; }
    return buildDomTree(document.body, highlight, null, makeHighlightContext(document.body));
}
function cleanupHighlights(scope) {
    if (scope === void 0) { scope = 'all'; }
    if (scope === 'container' || scope === 'all') {
        var container = document.getElementById(HIGHLIGHT_CONTAINER_ID);
        if (container) {
            container.remove();
        }
    }
    if (scope === 'attribute' || scope === 'all') {
        var highlightedElements = document.querySelectorAll("[data-" + HIGHLIGHT_ID_KEY + "]");
        highlightedElements.forEach(function (el) {
            delete el.dataset[HIGHLIGHT_ID_KEY];
        });
    }
}
