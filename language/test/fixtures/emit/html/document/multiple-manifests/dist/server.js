const homeHtml = {
    index: "assets/home.html",
    files: [
        {
            path: "assets/home.css",
            type: "asset",
            loader: "css",
            input: "src/home.css",
            isEntry: true,
            isDynamicEntry: false,
        },
        {
            path: "assets/home.html",
            type: "asset",
            loader: "html",
            isEntry: true,
            isDynamicEntry: false,
        },
        {
            path: "assets/home.js",
            type: "chunk",
            loader: "js",
            name: "home",
            input: "src/home.ts",
            isEntry: true,
            isDynamicEntry: false,
        },
        {
            path: "assets/home.js.map",
            type: "asset",
            loader: "map",
        },
    ],
};
const aboutHtml = {
    index: "assets/about.html",
    files: [
        {
            path: "assets/about.css",
            type: "asset",
            loader: "css",
            input: "src/about.css",
            isEntry: true,
            isDynamicEntry: false,
        },
        {
            path: "assets/about.html",
            type: "asset",
            loader: "html",
            isEntry: true,
            isDynamicEntry: false,
        },
        {
            path: "assets/about.js",
            type: "chunk",
            loader: "js",
            name: "about",
            input: "src/about.ts",
            isEntry: true,
            isDynamicEntry: false,
        },
        {
            path: "assets/about.js.map",
            type: "asset",
            loader: "map",
        },
    ],
};
console.log("Home manifest:", homeHtml);
console.log("About manifest:", aboutHtml);
//# sourceMappingURL=./server.js.map
