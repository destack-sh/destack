import { createSharedComposable } from "@vueuse/shared";
import { defineStore } from "pinia";
import { onBeforeMount, watch } from "vue";

export type Theme = "light" | "dark";

export type PanelAppearance = {
  wide?: boolean;
};

export const CONTENT_WIDTH_NARROW = 880;
export const CONTENT_WIDTH_WIDE = 3200;
export const CONTENT_MARGIN_X_NARROW = 32;
export const CONTENT_MARGIN_X_WIDE = 32;

export type Font = "sans" | "serif" | "mono";

export const useAppearanceState = defineStore("appearance", {
  state: () => ({
    fullscreen: false,
    theme: "light" as Theme,
    textSmall: true,
    font: "sans" as Font,
    contentWide: false,
    benchHeaderHeight: 52,
    panelHeaderHeight: 28,
  }),
  getters: {
    fontMono(state) {
      return state.font === "mono";
    },
    fontSerif(state) {
      return state.font === "serif";
    },
    contentWidth(state) {
      return state.contentWide ? CONTENT_WIDTH_WIDE : CONTENT_WIDTH_NARROW;
    },
    contentMarginX(state) {
      return state.contentWide ? CONTENT_MARGIN_X_WIDE : CONTENT_MARGIN_X_NARROW;
    },
    // :ContentSizeProps
    contentWidthWithMargin() {
      return (this.contentWidth as any) + 2 * (this.contentMarginX as any); // no idea why TS is complaining
    },
    contentWidthAsFixed() {
      return {
        width: `${this.contentWidth}px`,
      };
    },
    contentWidthAsMaxWidth() {
      return {
        maxWidth: `${this.contentWidth}px`,
      };
    },
    contentMarginXAsPaddingX() {
      return {
        paddingLeft: `${this.contentMarginX}px`,
        paddingRight: `${this.contentMarginX}px`,
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
      } catch (e) {
        console.error(`failed to restore appearance`, e);
      }
    }
  });

  return appearance;
}

export const useAppearance = createSharedComposable(_useAppearance);
