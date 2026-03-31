const __destack_resource_701041aa = "./assets/page.html";

const __destack_resource_e70ab4ad = "<!DOCTYPE html>\n<html>\n  <head>\n    <link rel=\"stylesheet\" href=\"./styles.css\">\n  </head>\n  <body>\n    <h1>Test Page</h1>\n  </body>\n</html>\n";

const htmlUrl = __destack_resource_701041aa;
const htmlManifest = __destack_resource_e70ab4ad;
if(typeof htmlUrl !== "string") {
    throw new Error("Expected htmlUrl to be a string, got " + typeof htmlUrl);
}
if(typeof htmlManifest !== "object" || !htmlManifest.index || !Array.isArray(htmlManifest.files)) {
    throw new Error("Expected htmlManifest to be an object with index and files array");
}
console.log("✓ File import returned URL:", htmlUrl);
console.log("✓ HTML import returned manifest with", htmlManifest.files.length, "files");
console.log("✓ Both import types work correctly");
//# sourceMappingURL=./entry.ts.map
