import { useAppearance } from "@/state/appearance";
import { useElementSize } from "@vueuse/core";
import { inject, provide, ref, watch, type Ref } from "vue";

export type ViewSectionInfo = {
  minHeight?: number;
  sectionRef: Ref<HTMLDivElement | undefined>;
  sectionBodyRef: Ref<HTMLDivElement | undefined>;
  sectionSlotRef: Ref<InstanceType<any> | undefined>;
  sectionBodySize: Ref<{ width: number; height: number } | undefined>;
};

export type ViewSectionGroupApi = {
  registerSection: (idx: number, ref: ViewSectionInfo | undefined) => void;
  heights: Ref<Record<number, number>>;
  focus: (idx: number, f?: "first" | "last") => void;
  blur: () => void;
  navigateUp: (idx: number) => void;
  navigateDown: (idx: number) => void;
  gapY: number;
};

export const VIEW_SECTION_GROUP = "__VIEW_SECTION_GROUP__";

export function provideViewSectionGroup(containerRef: Ref<HTMLDivElement | null>): ViewSectionGroupApi {
  const sections: Record<number, ViewSectionInfo> = {};
  const containerSize = useElementSize(containerRef);
  const heights: Ref<Record<number, number>> = ref({});
  const appearance = useAppearance();
  const gapY = 8;

  function registerSection(idx: number, ref: ViewSectionInfo | undefined) {
    if (ref != undefined) {
      sections[idx] = ref;
    } else {
      delete sections[idx];
    }
  }

  // update heights
  watch(
    () => [Object.values(sections).map((s) => s.sectionBodySize.value?.height), containerSize.height.value],
    () => {
      const numSections = Object.keys(sections).length;
      if (numSections == 0) {
        heights.value = {};
        return;
      }
      const headerHeight = appearance.panelHeaderHeight;
      const availableHeight = containerSize.height.value - (headerHeight + gapY) * numSections; // the 1 extra gap is a bottom margin
      const newBodyHeights: Record<number, number> = {};
      const actualBodyHeights: Record<number, number> = {};
      const targetBodyHeights: Record<number, number> = {};

      // init
      let spareBodyHeight = 0;
      Object.keys(sections).forEach((k) => {
        const idx = parseInt(k);
        const section = sections[idx];
        targetBodyHeights[idx] = availableHeight / numSections; // no grow/shrink for now
        actualBodyHeights[idx] = section.sectionBodySize.value?.height ?? 0;
        newBodyHeights[idx] = Math.max(section.minHeight ?? 0, targetBodyHeights[idx]); // init with max(minHeight, targetHeight)
        if (actualBodyHeights[idx] < targetBodyHeights[idx]) {
          spareBodyHeight += targetBodyHeights[idx] - actualBodyHeights[idx];
        }
      });

      // if larger than available height, scale down
      Object.keys(sections).forEach((k) => {
        const idx = parseInt(k);
        // use up spare height, shrink anything larger than target
        if (actualBodyHeights[idx] > targetBodyHeights[idx]) {
          const shrink = Math.min(actualBodyHeights[idx] - targetBodyHeights[idx], spareBodyHeight);
          spareBodyHeight -= shrink;
          newBodyHeights[idx] += shrink;
        }
      });

      heights.value = newBodyHeights;
    }
  );

  function focus(idx: number, f?: "first" | "last") {
    const section = sections[idx];
    if (section == null) return;
    section.sectionSlotRef.value?.focus(f);
  }

  function blur() {
    for (const section of Object.values(sections)) {
      section.sectionSlotRef.value?.blur();
    }
  }

  function navigateUp(idx: number) {
    const prev = Object.keys(sections)
      .map((k) => parseInt(k))
      .sort((a, b) => a - b)
      .find((k) => k < idx);
    if (prev != null) {
      focus(prev, "last");
    }
  }

  function navigateDown(idx: number) {
    const next = Object.keys(sections)
      .map((k) => parseInt(k))
      .sort((a, b) => a - b)
      .find((k) => k > idx);
    if (next != null) {
      focus(next, "first");
    }
  }

  const api = {
    registerSection,
    heights,
    focus,
    blur,
    navigateUp,
    navigateDown,
    gapY,
  };
  provide(VIEW_SECTION_GROUP, api);
  return api;
}

export function useViewSectionGroup(): ViewSectionGroupApi {
  return inject(VIEW_SECTION_GROUP) as ViewSectionGroupApi;
}
