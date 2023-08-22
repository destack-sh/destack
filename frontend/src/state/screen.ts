import type { Panel, PanelContext } from "@/state/bench";
import { computed, ref, type Ref } from "vue";

export function useTiling<T extends Panel>(panel: PanelContext<T>) {
  const gridStepX = ref(36); // p-9
  const gridStepY = ref(18); // p-4.5

  function getTileWidth(targetWidth?: number) {
    return Math.min(
      targetWidth ?? panel.panel.value.contentWidth,
      panel.size.value.width - 2 * panel.panel.value.contentMarginX
    );
  }

  function getTileOffsetX(targetWidth?: number) {
    return (panel.size.value.width - getTileWidth(targetWidth)) / 2;
  }

  function getTilePositionX(targetWidth?: number) {
    const tileWidth = getTileWidth(targetWidth);
    const tileOffsetX = getTileOffsetX(targetWidth);
    return {
      width: tileWidth + "px",
      marginLeft: tileOffsetX + "px",
      marginRight: tileOffsetX + "px",
    };
  }

  const baseTilePositionX = computed(() => getTilePositionX());

  return {
    gridStepX,
    gridStepY,
    getTileWidth,
    getTileOffsetX,
    getTilePositionX,
    baseTilePositionX,
  };
}
