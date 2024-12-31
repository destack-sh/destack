/**
 * Our in-Browser runtime.
 * Build with `tsc browser.ts --module none --removeComments`.
 */

/**
* A DOM node.
*/
type DomNode = {
	type: "TEXT_NODE" | "ELEMENT_NODE";
	tagName?: string;
	index?: number;
	text?: string;
	attributes?: Record<string, string>;
	xpath?: string;
	isInteractive?: boolean;
	isVisible?: boolean;
	isTop?: boolean;
	isShadowRoot?: boolean;
	children: DomNode[];
};

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
function getHighlightColor(index: number): { base: string; background: string } {
	const colors = [
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
	const base = colors[index % colors.length];
	return { base, background: `${base}1A` };
}

/**
* Highlights the given element by drawing an overlay and a label at the element's position.
* It uses the passed highlight index to distinguish multiple highlighted elements.
*/
function highlightElement(element: Element, index: number, iframe: HTMLIFrameElement | null): void {
	const container = getHighlightContainer();
	const { base, background } = getHighlightColor(index);
	
	// position calculation
	const rect = element.getBoundingClientRect();
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
	overlay.style.outline = `2px solid ${base}`;
	overlay.style.backgroundColor = background;
	overlay.style.pointerEvents = "none";
	overlay.style.boxSizing = "border-box";
	overlay.style.top = `${top}px`;
	overlay.style.left = `${left}px`;
	overlay.style.width = `${rect.width}px`;
	overlay.style.height = `${rect.height}px`;
	overlay.style.zIndex = "2147483641";
	container.appendChild(overlay);
	
	// label
	const label = document.createElement("div");
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
	
	const labelWidth = 20;
	const labelHeight = 16;
	const labelPadding = 0;
	const viewportMargin = 4; // minimum pixels from viewport edge
	
	// determine if label should be placed outside based on container size
	const shouldPlaceOutside = rect.width < labelWidth * 3 || 
		rect.height < labelHeight * 2;
	
	// calculate viewport bounds with margin
	const bounds = {
		top: viewportMargin,
		right: window.innerWidth - viewportMargin,
		bottom: window.innerHeight - viewportMargin,
		left: viewportMargin
	};
	
	// try positions in clockwise order
	let labelPosition;
	if (!shouldPlaceOutside) {
		// 1. try inside top-right
		const insideTopRight = {
			top: top + labelPadding,
			left: left + rect.width - labelWidth - labelPadding
		};
		if (insideTopRight.top >= bounds.top && 
			insideTopRight.left + labelWidth <= bounds.right) {
			labelPosition = insideTopRight;
		}
	}
	if (!labelPosition) {
		// 2. try outside top-right
		const outsideTopRight = {
			top: top - labelHeight - labelPadding,
			left: left + rect.width - labelPadding - labelWidth
		};
		if (outsideTopRight.top >= bounds.top &&
			outsideTopRight.left + labelWidth <= bounds.right) {
			labelPosition = outsideTopRight;
		}
	}
	if (!labelPosition) {
		// 3. try outside bottom-right
		const outsideBottomRight = {
			top: top + rect.height + labelPadding,
			left: left + rect.width - labelPadding - labelWidth
		};
		if (outsideBottomRight.top + labelHeight <= bounds.bottom &&
			outsideBottomRight.left + labelWidth <= bounds.right) {
			labelPosition = outsideBottomRight;
		}
	}
	if (!labelPosition) {
		// fallback: place where it fits best while respecting viewport bounds
		labelPosition = {
			top: Math.min(bounds.bottom - labelHeight,
				Math.max(bounds.top, top + labelPadding)),
			left: Math.min(bounds.right - labelWidth,
				Math.max(bounds.left, left + labelPadding))
		};
	}
	
	label.style.top = `${labelPosition.top}px`;
	label.style.left = `${labelPosition.left}px`;
	container.appendChild(label);
	
	element.setAttribute("bench-highlight-id", `bench-highlight-${index}`);
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
	  if (stopAtBoundary && 
		 (current.parentNode instanceof ShadowRoot || current.parentNode instanceof HTMLIFrameElement)) {
		break;
	  }
  
	  // count how many siblings have the same tag name before this one
	  let index = 0;
	  let sibling = current.previousSibling;
	  while (sibling) {
		if (sibling.nodeType === Node.ELEMENT_NODE &&
			sibling.nodeName === current.nodeName) {
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
	highlightIndex: number,
): [DomNode | null, number] {
	if (node.nodeType === Node.TEXT_NODE) {
		const textContent = node.textContent?.trim();
		if (textContent && isTextNodeVisible(node as Text)) {
			return [{
				type: "TEXT_NODE",
				text: textContent,
				children: [],
				attributes: {},
			}, highlightIndex];
		}
		return [null, highlightIndex];
	}
	
	if (node.nodeType === Node.ELEMENT_NODE) {
		const element = node as Element;
		if (!isElementIncluded(element)) return [null, highlightIndex];
		
		const nodeData: DomNode = {
			type: "ELEMENT_NODE",
			tagName: element.tagName.toLowerCase(),
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
		nodeData.isInteractive = isInteractive;
		nodeData.isVisible = isVisible;
		nodeData.isTop = isTop;
		
		// highlight
		if (highlight && isInteractive && isVisible && isTop) {
			nodeData.index = highlightIndex;
			highlightElement(element, highlightIndex, iframe);
			highlightIndex++;
		}
		
		// handle shadow roots
		if (element.shadowRoot) {
			nodeData.isShadowRoot = true;
			let currentIndex = highlightIndex;
			const shadowChildren: DomNode[] = [];
			Array.from(element.shadowRoot.childNodes).forEach(child => {
				const [childNode, newIndex] = buildDomTree(child, highlight, iframe, currentIndex);
				if (childNode) shadowChildren.push(childNode);
				currentIndex = newIndex;
			});
			nodeData.children.push(...shadowChildren);
			highlightIndex = currentIndex;
		}
		
		// iframes
		if (element.tagName === "IFRAME") {
			try {
				const iframeDoc = (element as HTMLIFrameElement).contentDocument;
				if (iframeDoc && iframeDoc.body) {
					let currentIndex = highlightIndex;
					const iframeChildren: DomNode[] = [];
					Array.from(iframeDoc.body.childNodes).forEach(child => {
						const [childNode, newIndex] = buildDomTree(child, highlight, element as HTMLIFrameElement, currentIndex);
						if (childNode) iframeChildren.push(childNode);
						currentIndex = newIndex;
					});
					nodeData.children.push(...iframeChildren);
					highlightIndex = currentIndex;
				}
			} catch {
				// ignore iframe access errors
			}
		} else {
			let currentIndex = highlightIndex;
			const children: DomNode[] = [];
			Array.from(node.childNodes).forEach(child => {
				const [childNode, newIndex] = buildDomTree(child, highlight, iframe, currentIndex);
				if (childNode) children.push(childNode);
				currentIndex = newIndex;
			});
			nodeData.children.push(...children);
			highlightIndex = currentIndex;
		}
		
		return [nodeData, highlightIndex];
	}
	
	return [null, highlightIndex];
}

/**
* Extract the DOM tree from the current document's body.
*/
function extractDocumentDomTree(highlight = true): DomNode | null {
	const [tree] = buildDomTree(document.body, highlight, null, 0);
	return tree;
}

/**
 * Clean up any highlights from the DOM.
 */
function cleanupHighlights(): void {
	 // remove the highlight container and all its contents
	 const container = document.getElementById(HIGHLIGHT_CONTAINER_ID);
	 if (container) {
		 container.remove();
	 }
	 // remove highlight attributes from elements
	 const highlightedElements = document.querySelectorAll('[bench-highlight-id^="bench-highlight-"]');
	 highlightedElements.forEach(el => {
		 el.removeAttribute('bench-highlight-id');
	 });
}
