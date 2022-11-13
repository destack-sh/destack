/* eslint-disable no-console */

import { createPinia } from "pinia";
import { createApp, h, provide } from "vue";
import { version } from "../../package.json";

import { createMetaManager } from "vue-meta";
import App from "./App.vue";
import { hydrate } from "./hydrate";
import router from "./router";
import { DefaultApolloClient } from "@vue/apollo-composable";
import { ApolloClient, InMemoryCache } from "@apollo/client/core";

async function init() {
  const apolloClient = new ApolloClient({
    uri: "http://localhost:8000/graphql",
    cache: new InMemoryCache(),
  });

  const app = createApp({
    setup() {
      provide(DefaultApolloClient, apolloClient);
    },
    render: () => h(App),
  });

  app.use(createPinia());
  app.use(router);
  app.use(createMetaManager());

  app.mount("#app");

  console.group(`%cProject Information`, "color:orangered"); // groupCollapsed

  if (import.meta.env.DEV) {
    console.info(`%cVersion: v${version}`, "color:orangered");
  }

  console.info(`%cEnvironment: ${import.meta.env.MODE}`, "color:orangered");
  console.groupEnd();

  // start loading
  await hydrate();

  // prevent opening files that are dragged over the window
  window.addEventListener("dragover", (e) => e.preventDefault(), false);
  window.addEventListener("drop", (e) => e.preventDefault(), false);
}

init();
