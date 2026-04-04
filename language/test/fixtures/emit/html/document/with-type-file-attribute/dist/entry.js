const __destack_resource_701041aa = "./assets/page.html";

const __destack_resource_e70ab4ad = "<!DOCTYPE html>\n<html>\n  <head>\n    <link rel=\"stylesheet\" href=\"./styles.css\">\n  </head>\n  <body>\n    <h1>Test Page</h1>\n  </body>\n</html>\n";

const htmlUrl = __destack_resource_701041aa;
const htmlDocument = __destack_resource_e70ab4ad;
if(typeof htmlUrl !== "string") {
    throw new Error("Expected htmlUrl to be a string, got " + typeof htmlUrl);
}
if(typeof htmlDocument !== "string" || !htmlDocument.includes("<h1>Test Page</h1>")) {
    throw new Error("Expected htmlDocument to be an HTML string");
}
console.log("✓ File import returned URL:", htmlUrl);
console.log("✓ HTML import returned text:", htmlDocument);
console.log("✓ Both import types work correctly");
//# sourceMappingURL=./entry.js.map
