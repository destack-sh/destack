<script lang="ts" setup>
import { NodeType, Orientation, RectangleData, ViewData, ViewType } from "@/proto/wire";
import { unwrapProtoOneOf, type TypedNodeReferenceData } from "@/proto/wiring";
import { useExistingConnection } from "@/system/connection";
import { canvas, inspectionPtr } from "@/system/space";
import { getInspectionLayout } from "@/ui/inspect";
import { ScrollbarWidth } from "@/ui/layout";
import { VIEW_DEFAULT_HEADER_HEIGHT, VIEW_DEFAULT_MAX_WIDTH } from "@/ui/view";
import { computedValue } from "@/utils/ref";
import { viewEmits, type ViewExposed } from "@/views/common";
import Scroll from "@/views/containers/Scroll.vue";
import { getViewComponent, hasViewComponent } from "@/views/registry";
import { computed, toRef } from "vue";

const HEADER_HEIGHT = VIEW_DEFAULT_HEADER_HEIGHT;
const MIN_WIDTH = 320;
const MAX_WIDTH = VIEW_DEFAULT_MAX_WIDTH;

const props = defineProps<
  {
    self?: TypedNodeReferenceData<NodeType.VIEW>;
    id: string;
  } & Pick<ViewData, "icon" | "size" | "nodePtr">
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = toRef(props, "id");
const state = canvas.registerView(self, id);

const nodePtr = computedValue(() => unwrapProtoOneOf(props.nodePtr));
const inspectedPtr = computedValue(() => nodePtr.value ?? inspectionPtr.value);

const { graph: pkgGraph, connection: pkgConnection } = useExistingConnection(inspectedPtr);
const inspectedNode = pkgGraph.getRef(inspectedPtr);

const inspectionLayout = computed(() => {
  if (inspectedNode.value == null) return null;
  const layout = getInspectionLayout(inspectedNode.value, { exclude: ["icon", "name"] });
  return layout;
});

defineExpose<ViewExposed>({ self, id });
</script>
<template>
  <div v-if="inspectedNode && inspectionLayout" class="" :class="size == null ? '' : 'h-full w-full'">
    <!-- Inspection content -->
    <component
      :is="size == null ? 'div' : Scroll"
      id="scroll"
      :size="{ width: size?.width, height: (size?.height ?? 0) - HEADER_HEIGHT }"
      :orientation="Orientation.VERTICAL"
      :track-width="ScrollbarWidth.sm"
      track-is-overlay
    >
      <ul class="flex flex-col gap-y-2.5 py-3">
        <template
          v-for="(
            { title, protoName, category, property, viewType, props: viewProps, isFullWidth, read, write }, i
          ) of inspectionLayout.properties"
          :key="i"
        >
          <!-- Category Header -->
          <template v-if="i != 0 && inspectionLayout.properties[i - 1].category != category">
            <!-- Divider -->
            <div class="mx-auto my-2 w-full px-5" :style="{ minWidth: MIN_WIDTH + 'px', maxWidth: MAX_WIDTH + 'px' }">
              <div class="h-[1px] w-full min-w-fit bg-gray-200" />
            </div>
            <!-- Label -->
            <div class="mx-auto w-full px-5" :style="{ minWidth: MIN_WIDTH + 'px', maxWidth: MAX_WIDTH + 'px' }">
              <h4 class="font-semibold text-gray-900">{{ category }}</h4>
            </div>
          </template>
          <!-- Property -->
          <li
            class="mx-auto w-full px-5"
            :class="[isFullWidth ? 'flex flex-col gap-y-0.5' : 'flex flex-row flex-wrap items-center gap-x-[10%]']"
            :style="{ minWidth: MIN_WIDTH + 'px', maxWidth: MAX_WIDTH + 'px' }"
          >
            <!-- Title & Controls -->
            <span class="w-[100px]">
              <span class="max-w-full truncate py-1 font-medium">{{ title }}</span>
            </span>
            <!-- Value -->
            <component
              :is="getViewComponent(viewType)"
              v-if="viewType != null && hasViewComponent(viewType)"
              :id="i + '.value'"
              :class="['ml-auto flex-shrink-0', isFullWidth ? '' : 'text-right']"
              :style="{ width: isFullWidth ? '100%' : 'calc(90% - 100px)' }"
              v-bind="{ ...viewProps, isInput: true }"
              :model-value="read != null ? read(inspectedNode) : (inspectedNode as any)[protoName!]"
              @update:model-value="
                (value: any) => {
                  // not sure how to :DebounceNestedValue properly (different types with different debounce needs)
                  if (write != null) write(pkgConnection.tx, pkgGraph, inspectedNode!, value);
                  else pkgConnection.tx.update(inspectedNode!, { [protoName!]: value }, { debounce: 'short' });
                  inspectionLayout?.onWrite?.(pkgConnection.tx, pkgGraph, inspectedNode!, property);
                }
              "
            />
            <div v-else class="ml-auto text-warning-600">
              {{ viewType != null ? ViewType[viewType] : "No View for Type" }}
            </div>
          </li>
        </template>
      </ul>
    </component>
  </div>
  <div v-else class="flex h-full w-full flex-col justify-center text-center">
    <!-- Empty/missing state -->
  </div>
</template>
