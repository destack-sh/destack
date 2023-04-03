import { createSharedComposable } from "@vueuse/shared";
import { defineStore } from "pinia";
import { onBeforeMount, watch } from "vue";

export type Theme = "light" | "dark";

export const useAppearanceState = defineStore("appearance", {
  state: () => ({
    fullscreen: false,
    theme: "light" as Theme,
    textSmall: true,
    fontMono: false,
    inlineMetrics: true,
  }),
});

function _useAppearance() {
  const appearance = useAppearanceState();

  // auto persist and recover state (local storage for now)
  watch(
    appearance.$state,
    () => {
      // persist
      localStorage.setItem("appearance", JSON.stringify(appearance.$state));

      // apply dark mode
      if (
        appearance.theme === "dark" ||
        (!("theme" in localStorage) && window.matchMedia("(prefers-color-scheme: dark)").matches)
      ) {
        document.documentElement.classList.add("dark");
      } else {
        document.documentElement.classList.remove("dark");
      }
    },
    { deep: true }
  );

  // load on mounted (supposed to be used at root level)
  onBeforeMount(() => {
    // load state from local storage
    const state = localStorage.getItem("appearance");
    if (state) {
      try {
        appearance.$state = JSON.parse(state);
        console.log("restored appearance");
      } catch (e) {
        console.error(`failed to restore appearance`, e);
      }
    }
  });

  return appearance;
}

export const useAppearance = createSharedComposable(_useAppearance);
