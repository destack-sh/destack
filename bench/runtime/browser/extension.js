var DomNodeType;
(function (DomNodeType) {
    DomNodeType[DomNodeType["TEXT"] = 1] = "TEXT";
    DomNodeType[DomNodeType["ELEMENT"] = 2] = "ELEMENT";
})(DomNodeType || (DomNodeType = {}));
var LABEL_WIDTH = 20;
var LABEL_HEIGHT = 16;
var LABEL_PADDING = 0;
var CELL_SIZE = 50;
var VIEWPORT_MARGIN = 0;
var HIGHLIGHT_ID_KEY = "benchHighlightId";
function iou(rect1, rect2) {
    var intersection = this.intersection(rect1, rect2);
    var union = this.union(rect1, rect2);
    return intersection / union;
}
function intersection(rect1, rect2) {
    var left = Math.max(rect1.left, rect2.left);
    var top = Math.max(rect1.top, rect2.top);
    var right = Math.min(rect1.right, rect2.right);
    var bottom = Math.min(rect1.bottom, rect2.bottom);
    var width = right - left;
    var height = bottom - top;
    return width * height;
}
function union(rect1, rect2) {
    var left = Math.min(rect1.left, rect2.left);
    var top = Math.min(rect1.top, rect2.top);
    var right = Math.max(rect1.right, rect2.right);
    var bottom = Math.max(rect1.bottom, rect2.bottom);
    var width = right - left;
    var height = bottom - top;
    return width * height;
}
var ElementGrid = (function () {
    function ElementGrid() {
        this.grid = new Map();
    }
    ElementGrid.prototype.cellCoords = function (x, y) {
        return [Math.floor(x / CELL_SIZE), Math.floor(y / CELL_SIZE)];
    };
    ElementGrid.prototype.insertCell = function (cx, cy, cellData) {
        if (!this.grid.has(cx)) {
            this.grid.set(cx, new Map());
        }
        var col = this.grid.get(cx);
        if (!col.has(cy)) {
            col.set(cy, []);
        }
        col.get(cy).push(cellData);
    };
    ElementGrid.prototype.removeCell = function (cx, cy, element) {
        var col = this.grid.get(cx);
        if (!col)
            return;
        var cell = col.get(cy);
        if (!cell)
            return;
        var index = cell.findIndex(function (c) { return c.element === element; });
        if (index !== -1) {
            cell.splice(index, 1);
        }
    };
    ElementGrid.prototype.add = function (element, rect) {
        var _a = this.cellCoords(rect.left, rect.top), startCx = _a[0], startCy = _a[1];
        var _b = this.cellCoords(rect.right, rect.bottom), endCx = _b[0], endCy = _b[1];
        for (var cx = startCx; cx <= endCx; cx++) {
            for (var cy = startCy; cy <= endCy; cy++) {
                this.insertCell(cx, cy, { element: element, rect: rect });
            }
        }
    };
    ElementGrid.prototype.remove = function (element, rect) {
        var _a = this.cellCoords(rect.left, rect.top), startCx = _a[0], startCy = _a[1];
        var _b = this.cellCoords(rect.right, rect.bottom), endCx = _b[0], endCy = _b[1];
        for (var cx = startCx; cx <= endCx; cx++) {
            for (var cy = startCy; cy <= endCy; cy++) {
                this.removeCell(cx, cy, element);
            }
        }
    };
    ElementGrid.prototype.hasIntersection = function (left, top, width, height) {
        var _a = this.cellCoords(left, top), startCx = _a[0], startCy = _a[1];
        var _b = this.cellCoords(left + width, top + height), endCx = _b[0], endCy = _b[1];
        for (var cx = startCx; cx <= endCx; cx++) {
            var col = this.grid.get(cx);
            if (!col)
                continue;
            for (var cy = startCy; cy <= endCy; cy++) {
                var cell = col.get(cy);
                if (!cell)
                    continue;
                for (var _i = 0, cell_1 = cell; _i < cell_1.length; _i++) {
                    var rect = cell_1[_i].rect;
                    if (left < rect.right && left + width > rect.left && top < rect.bottom && top + height > rect.top) {
                        return true;
                    }
                }
            }
        }
        return false;
    };
    ElementGrid.prototype.getOverlappingElement = function (query, minIoU) {
        var _a;
        var _b = this.cellCoords(query.left, query.top), startCx = _b[0], startCy = _b[1];
        var _c = this.cellCoords(query.right, query.bottom), endCx = _c[0], endCy = _c[1];
        var bestIoU = 0;
        var bestElement = null;
        for (var cx = startCx; cx <= endCx; cx++) {
            for (var cy = startCy; cy <= endCy; cy++) {
                var cell = (_a = this.grid.get(cx)) === null || _a === void 0 ? void 0 : _a.get(cy);
                if (!cell)
                    continue;
                for (var _i = 0, cell_2 = cell; _i < cell_2.length; _i++) {
                    var _d = cell_2[_i], rect = _d.rect, element = _d.element;
                    var iouScore = iou(rect, query);
                    if (iouScore > bestIoU) {
                        bestIoU = iouScore;
                        bestElement = element;
                    }
                }
            }
        }
        if (bestIoU < minIoU)
            return null;
        return bestElement;
    };
    return ElementGrid;
}());
var HighlightContext = (function () {
    function HighlightContext(root) {
        this.root = root;
        this.rootRect = root.getBoundingClientRect();
        this.highlightId = 0;
        this.labelGrid = new ElementGrid();
        this.elementGrid = new ElementGrid();
        this.rectByElement = new Map();
        this.nodeByElement = new Map();
    }
    HighlightContext.prototype.addElement = function (element, node) {
        var rect = element.getBoundingClientRect();
        this.elementGrid.add(element, rect);
        this.rectByElement.set(element, rect);
        this.nodeByElement.set(element, node);
    };
    HighlightContext.prototype.removeElement = function (element) {
        var rect = this.rectByElement.get(element);
        if (!rect)
            return;
        this.elementGrid.remove(element, rect);
        this.rectByElement["delete"](element);
        this.nodeByElement["delete"](element);
    };
    HighlightContext.prototype.getNodeAt = function (rect, minIoU) {
        var element = this.elementGrid.getOverlappingElement(rect, minIoU);
        if (!element)
            return null;
        var node = this.nodeByElement.get(element);
        if (!node)
            throw new Error("no node found for element");
        return node;
    };
    HighlightContext.prototype.addLabel = function (label) {
        var rect = label.getBoundingClientRect();
        this.labelGrid.add(label, rect);
    };
    HighlightContext.prototype.isLabelAt = function (left, top) {
        return this.labelGrid.hasIntersection(left, top, LABEL_WIDTH, LABEL_HEIGHT);
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
var ATTRIBUTE_WHITELIST = new Set(["href", "src", "alt", "title", "placeholder", "value", "aria-label"]);
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
    return base;
}
function highlightElement(element, rect, context, iframe) {
    var index = context.highlightId;
    var container = getHighlightContainer();
    var baseColor = getHighlightColor(index);
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
    overlay.style.outline = outlineWidth + "px solid " + baseColor + "CC";
    overlay.style.outlineOffset = -outlineWidth / 2 + "px";
    overlay.style.backgroundColor = baseColor + "33";
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
    var shouldPlaceOutside = rect.width < LABEL_WIDTH * 3 || rect.height < LABEL_HEIGHT * 2;
    var bounds = {
        top: VIEWPORT_MARGIN,
        right: window.innerWidth - VIEWPORT_MARGIN,
        bottom: window.innerHeight - VIEWPORT_MARGIN,
        left: VIEWPORT_MARGIN
    };
    var labelPosition;
    if (!shouldPlaceOutside) {
        var insideTopRight = {
            top: top,
            left: left + rect.width - LABEL_WIDTH - LABEL_PADDING
        };
        if (insideTopRight.top >= bounds.top &&
            insideTopRight.left + LABEL_WIDTH <= bounds.right &&
            !context.isLabelAt(insideTopRight.left, insideTopRight.top)) {
            labelPosition = insideTopRight;
        }
    }
    if (!shouldPlaceOutside && !labelPosition) {
        var insideBottomLeft = {
            top: top + rect.height - LABEL_HEIGHT - LABEL_PADDING,
            left: left + LABEL_PADDING
        };
        if (insideBottomLeft.top >= bounds.top &&
            insideBottomLeft.left + LABEL_WIDTH <= bounds.right &&
            !context.isLabelAt(insideBottomLeft.left, insideBottomLeft.top)) {
            labelPosition = insideBottomLeft;
        }
    }
    if (!labelPosition) {
        var outsideTopRight = {
            top: top - LABEL_HEIGHT - LABEL_PADDING,
            left: left + rect.width - LABEL_PADDING - LABEL_WIDTH
        };
        if (outsideTopRight.top >= bounds.top &&
            outsideTopRight.left + LABEL_WIDTH <= bounds.right &&
            !context.isLabelAt(outsideTopRight.left, outsideTopRight.top)) {
            labelPosition = outsideTopRight;
        }
    }
    if (!labelPosition) {
        var outsideBottomRight = {
            top: top + rect.height + LABEL_PADDING,
            left: left + rect.width - LABEL_PADDING - LABEL_WIDTH
        };
        if (outsideBottomRight.top + LABEL_HEIGHT <= bounds.bottom &&
            outsideBottomRight.left + LABEL_WIDTH <= bounds.right &&
            !context.isLabelAt(outsideBottomRight.left, outsideBottomRight.top)) {
            labelPosition = outsideBottomRight;
        }
    }
    if (!labelPosition) {
        var outsideBottomLeft = {
            top: top + rect.height + LABEL_PADDING,
            left: left - LABEL_WIDTH - LABEL_PADDING
        };
        if (outsideBottomLeft.top + LABEL_HEIGHT <= bounds.bottom &&
            outsideBottomLeft.left >= bounds.left &&
            !context.isLabelAt(outsideBottomLeft.left, outsideBottomLeft.top)) {
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
            if (sibling.nodeType === Node.ELEMENT_NODE && sibling.nodeName === current.nodeName) {
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
var MAX_TEXT_LENGTH = 300;
function trimText(text) {
    if (!text)
        return undefined;
    text = text.trim();
    if (text.length === 0)
        return undefined;
    if (text.length <= MAX_TEXT_LENGTH)
        return text;
    var half = Math.floor(MAX_TEXT_LENGTH / 2);
    return text.slice(0, half) + "..." + text.slice(-half);
}
function extract(node, highlight, iframe, context) {
    var _a, _b, _c;
    var _d;
    if (node.nodeType === Node.TEXT_NODE) {
        var textContent = trimText(node.textContent);
        if (textContent && isTextNodeVisible(node)) {
            return { type: DomNodeType.TEXT, text: textContent };
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
            text: trimText(element_1.textContent)
        };
        var attributeNames = ((_d = element_1.getAttributeNames) === null || _d === void 0 ? void 0 : _d.call(element_1)) || [];
        for (var _i = 0, attributeNames_1 = attributeNames; _i < attributeNames_1.length; _i++) {
            var name_1 = attributeNames_1[_i];
            if (nodeData.attributes === undefined) {
                nodeData.attributes = {};
            }
            var attrVal = element_1.getAttribute(name_1);
            if (attrVal !== null && ATTRIBUTE_WHITELIST.has(name_1)) {
                nodeData.attributes[name_1] = attrVal;
            }
        }
        var isInteractive = isInteractiveElement(element_1);
        var isVisible = isElementVisible(element_1);
        var isTop = isTopElement(element_1);
        if (isInteractive)
            nodeData.isInteractive = true;
        if (isVisible)
            nodeData.isVisible = true;
        if (isTop)
            nodeData.isTop = true;
        if (highlight && isInteractive && isVisible && isTop) {
            var rect = element_1.getBoundingClientRect();
            var overlappingNode = context.getNodeAt(rect, 0.8);
            context.addElement(element_1, nodeData);
            if (overlappingNode == null && iou(rect, context.rootRect) < 0.4) {
                context.highlightId++;
                nodeData.id = context.highlightId;
                highlightElement(element_1, rect, context, iframe);
                nodeData.isHighlighted = true;
            }
        }
        if (element_1.shadowRoot) {
            nodeData.isShadowRoot = true;
            var shadowChildren_1 = [];
            Array.from(element_1.shadowRoot.childNodes).forEach(function (child) {
                var childNode = extract(child, highlight, iframe, context);
                if (childNode)
                    shadowChildren_1.push(childNode);
            });
            if (nodeData.children === undefined) {
                nodeData.children = [];
            }
            (_a = nodeData.children).push.apply(_a, shadowChildren_1);
        }
        if (element_1.tagName === "IFRAME") {
            try {
                var iframeDoc = element_1.contentDocument;
                if (iframeDoc && iframeDoc.body) {
                    var iframeChildren_1 = [];
                    Array.from(iframeDoc.body.childNodes).forEach(function (child) {
                        var childNode = extract(child, highlight, element_1, context);
                        if (childNode)
                            iframeChildren_1.push(childNode);
                    });
                    if (nodeData.children === undefined) {
                        nodeData.children = [];
                    }
                    (_b = nodeData.children).push.apply(_b, iframeChildren_1);
                }
            }
            catch (_e) {
            }
        }
        else {
            var children_1 = [];
            Array.from(node.childNodes).forEach(function (child) {
                var childNode = extract(child, highlight, iframe, context);
                if (childNode)
                    children_1.push(childNode);
            });
            if (nodeData.children === undefined) {
                nodeData.children = [];
            }
            (_c = nodeData.children).push.apply(_c, children_1);
        }
        return nodeData;
    }
    return null;
}
function highlight() {
    var root = extract(document.body, true, null, makeHighlightContext(document.body));
    var interactiveNodes = [];
    function walk(node) {
        if (node.isHighlighted) {
            var miniNode = { id: node.id, tag: node.tag };
            if (node.text) {
                miniNode.text = node.text;
            }
            if (node.attributes && Object.keys(node.attributes).length > 0) {
                miniNode.attributes = node.attributes;
            }
            interactiveNodes.push(miniNode);
        }
        if (node.children) {
            for (var _i = 0, _a = node.children; _i < _a.length; _i++) {
                var child = _a[_i];
                walk(child);
            }
        }
    }
    if (root) {
        walk(root);
    }
    return interactiveNodes;
}
function cleanup(scope) {
    if (scope === void 0) { scope = "all"; }
    if (scope === "container" || scope === "all") {
        var container = document.getElementById(HIGHLIGHT_CONTAINER_ID);
        if (container) {
            container.remove();
        }
    }
    if (scope === "attribute" || scope === "all") {
        var highlightedElements = document.querySelectorAll("[data-" + HIGHLIGHT_ID_KEY + "]");
        highlightedElements.forEach(function (el) {
            delete el.dataset[HIGHLIGHT_ID_KEY];
        });
    }
}
