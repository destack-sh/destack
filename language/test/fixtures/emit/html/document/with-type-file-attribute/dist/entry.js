const htmlUrl = "./assets/page.html";
const htmlManifest = {
    index: "assets/index.html",
    files: [
        {
            path: "assets/index.html",
            type: "asset",
            loader: "html",
            isEntry: true,
            isDynamicEntry: false,
        },
        {
            path: "assets/styles.css",
            type: "asset",
            loader: "css",
            input: "src/styles.css",
            isEntry: true,
            isDynamicEntry: false,
        },
    ],
};
if(typeof htmlUrl !== "string") {
    throw new Error("Expected htmlUrl to be a string, got " + typeof htmlUrl);
}
if(typeof htmlManifest !== "object" || !htmlManifest.index || !Array.isArray(htmlManifest.files)) {
    throw new Error("Expected htmlManifest to be an object with index and files array");
}
console.log("✓ File import returned URL:", htmlUrl);
console.log("✓ HTML import returned manifest with", htmlManifest.files.length, "files");
console.log("✓ Both import types work correctly");
//# sourceMappingURL=./entry.js.map
