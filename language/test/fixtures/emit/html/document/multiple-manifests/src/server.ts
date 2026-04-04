import homeHtml from "./home.html";
import aboutHtml from "./about.html";

if (typeof homeHtml !== "string") {
  throw new Error("Expected homeHtml to be an HTML string");
}

if (typeof aboutHtml !== "string") {
  throw new Error("Expected aboutHtml to be an HTML string");
}

console.log("Home HTML:", homeHtml);
console.log("About HTML:", aboutHtml);
