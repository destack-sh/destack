/* eslint-disable no-console */

import { createApp, h, provide } from "vue";
import { version } from "../../package.json";

import { ApolloClient, InMemoryCache } from "@apollo/client/core";
import { DefaultApolloClient } from "@vue/apollo-composable";
import { createMetaManager } from "vue-meta";
import App from "./App.vue";
import router from "./router";

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

  app.use(router);
  app.use(createMetaManager());

  app.mount("#app");

  console.group(`%cProject Information`, "color:orangered"); // groupCollapsed

  if (import.meta.env.DEV) {
    console.info(`%cVersion: v${version}`, "color:orangered");
  }

  console.info(`%cEnvironment: ${import.meta.env.MODE}`, "color:orangered");
  console.groupEnd();

  // prevent opening files that are dragged over the window
  window.addEventListener("dragover", (e) => e.preventDefault(), false);
  window.addEventListener("drop", (e) => e.preventDefault(), false);
}

init();
