import { createSharedComposable } from "@vueuse/shared";
import { defineStore } from "pinia";
import { onBeforeMount, watch } from "vue";

export type Theme = "light" | "dark";

export type EditorAppearance = {
  contentWidth?: number;
  contentMarginX?: number;
  headerHeight?: number;
};

export const CONTENT_WIDTH_NARROW = 800;
export const CONTENT_WIDTH_WIDE = 3200;
export const CONTENT_MARGIN_X_NARROW = 72;
export const CONTENT_MARGIN_X_WIDE = 48;

export const useAppearanceState = defineStore("appearance", {
  state: () => ({
    fullscreen: false,
    theme: "light" as Theme,
    textSmall: true,
    font: "sans" as "sans" | "serif" | "mono",
    inlineMetrics: false,
    contentWidth: 800,
    contentMarginX: 70,
    benchHeaderHeight: 52,
    editorHeaderHeight: 28,
  }),
  getters: {
    fontMono(state) {
      return state.font === "mono";
    },
    fontSerif(state) {
      return state.font === "serif";
    },
    contentWidthWithMargin(state) {
      return state.contentWidth + 2 * state.contentMarginX;
    },
    contentWidthAsFixed(state) {
      return {
        width: `${state.contentWidth}px`,
      };
    },
    contentWidthAsMaxWidth(state) {
      return {
        maxWidth: `${state.contentWidth}px`,
      };
    },
    contentMarginXAsPaddingX(state) {
      return {
        paddingLeft: `${state.contentMarginX}px`,
        paddingRight: `${state.contentMarginX}px`,
      };
    },
    contentMarginXAsMarginX(state) {
      return {
        marginLeft: `${state.contentMarginX}px`,
        marginRight: `${state.contentMarginX}px`,
      };
    },
    baseClass(state) {
      return {
        "text-sm placeholder:text-sm": state.textSmall,
        "text-md placeholder:text-md": !state.textSmall,
        "font-mono": state.font == "mono",
        "font-serif": state.font == "serif",
        "font-sans": state.font == "sans",
      };
    },
    baseClassUnsized(state) {
      return {
        "font-mono": state.font == "mono",
        "font-serif": state.font == "serif",
        "font-sans": state.font == "sans",
      };
    },
  },
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
