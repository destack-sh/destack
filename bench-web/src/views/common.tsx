// TODO :Architecture: figure out proper all-encompassing event system/bus
//  (we want to capture, replay, etc.)
export const VIEW_EMITS = {};

export function viewEmits() {
  return VIEW_EMITS;
}
