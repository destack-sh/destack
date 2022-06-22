/* eslint-disable no-console */

import { createPinia } from "pinia";
import { createApp } from "vue";
import { version } from "../../package.json";

import App from "./App.vue";
import router from "./router";

const app = createApp(App);

app.use(createPinia());
app.use(router);

app.mount("#app");

console.group(`%c✨ Project Information`, "color:orangered"); // groupCollapsed

if (import.meta.env.DEV) {
  console.info(`%cVersion: v${version}`, "color:orangered");
}

console.info(`%cEnvironment: ${import.meta.env.MODE}`, "color:orangered");
console.groupEnd();

// prevent opening files that are dragged over the window
window.addEventListener("dragover", (e) => e.preventDefault(), false);
window.addEventListener("drop", (e) => e.preventDefault(), false);
