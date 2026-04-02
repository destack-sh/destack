export function formatBootstrapMode(mode) {
    return `bootstrap:${mode}`;
}

export function buildPageMessage(label) {
    return `page:${label}`;
}

const page = {
    index: "assets/page.html",
    files: [
        {
            path: "assets/bootstrap.js",
            type: "chunk",
            loader: "js",
            name: "bootstrap",
            input: "src/in/bootstrap.ts",
            isEntry: true,
            isDynamicEntry: false,
            imports: ["../main.js"],
        },
        {
            path: "assets/bootstrap.js.map",
            type: "asset",
            loader: "map",
        },
        {
            path: "assets/page.css",
            type: "asset",
            loader: "css",
            input: "src/in/page.css",
            isEntry: true,
            isDynamicEntry: false,
        },
        {
            path: "assets/page.html",
            type: "asset",
            loader: "html",
            isEntry: true,
            isDynamicEntry: false,
        },
    ],
};
const pageMessage = buildPageMessage("circular-import");
console.log("Main JS loaded page:", page, pageMessage, formatBootstrapMode("module"));
//# sourceMappingURL=./main.js.map
