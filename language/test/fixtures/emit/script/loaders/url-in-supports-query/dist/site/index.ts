const __destack_resource_1894f456 = "../assets/style.css";
if(typeof document !== "undefined") {
    const __destack_stylesheet_link_1894f456 = document.createElement("link");
    __destack_stylesheet_link_1894f456.rel="stylesheet";
    __destack_stylesheet_link_1894f456.href=__destack_resource_1894f456;
    document.head.appendChild(__destack_stylesheet_link_1894f456);
}

export function describeSupportQuery(status) {
    return `supports:${status}`;
}

console.log("url-in-supports-query", describeSupportQuery("ready"));
//# sourceMappingURL=./index.ts.map
