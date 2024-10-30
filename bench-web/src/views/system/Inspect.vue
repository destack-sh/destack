<script lang="ts" setup>
import { toCamelName } from "@/language/const";
import { BoxData, ColorShade, NodeType, ObjectType, Orientation, ViewData, ViewType } from "@/proto/wire";
import { unwrapProtoOneOf, type TypedNodeReferenceData } from "@/proto/wiring";
import { useExistingConnection } from "@/system/connection";
import { canvas, inspectionPtr } from "@/system/space";
import { ICON_BY_NODE_TYPE, IconInline } from "@/ui/icon";
import { getInspectionLayout } from "@/ui/inspect";
import { ScrollbarWidth } from "@/ui/layout";
import { toggleHelperViewPin, VIEW_DEFAULT_HEADER_HEIGHT, VIEW_DEFAULT_MAX_WIDTH } from "@/ui/view";
import { computedValue } from "@/utils/ref";
import NodeReference from "@/views/builtins/NodeReference.vue";
import { viewEmits, type ViewExposed } from "@/views/common";
import Scroll from "@/views/containers/Scroll.vue";
import { getViewComponent, hasViewComponent } from "@/views/registry";
import { computed, toRef } from "vue";

const HEADER_HEIGHT = VIEW_DEFAULT_HEADER_HEIGHT;
const MIN_WIDTH = 320;
const MAX_WIDTH = VIEW_DEFAULT_MAX_WIDTH;

const props = defineProps<
  { self: TypedNodeReferenceData<NodeType.VIEW>; id: string; size: Required<Pick<BoxData, "width" | "height">> } & Pick<
    ViewData,
    "name" | "title" | "icon" | "nodePtr"
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = toRef(props, "id");
const nodePtr = computedValue(() => unwrapProtoOneOf(props.nodePtr));
const inspectedPtr = computedValue(() => nodePtr.value ?? inspectionPtr.value);

const { graph: spaceGraph, connection: spaceConnection } = useExistingConnection(self);
const { graph: pkgGraph, connection: pkgConnection } = useExistingConnection(inspectedPtr);
const inspectedNode = pkgGraph.getRef(inspectedPtr);

const inspectionLayout = computed(() => {
  if (inspectedNode.value == null) return null;
  const layout = getInspectionLayout(inspectedNode.value, { exclude: ["icon", "name"] });
  return layout;
});

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self });
</script>
<template>
  <div v-if="inspectedNode && inspectionLayout" class="h-full w-full">
    <!-- Header -->
    <div class="group w-full" :style="{ height: HEADER_HEIGHT + 'px' }">
      <div
        class="mx-auto flex h-full max-w-full flex-row items-center pl-2 pr-2.5"
        :style="{ minWidth: MIN_WIDTH + 'px' }"
      >
        <!-- Node -->
        <NodeReference :node="inspectedNode" :connection="pkgConnection" is-input class="font-medium" />
        <!-- Pin/unpin node -->
        <button
          v-tooltip="{ title: 'Pin node in view', small: true, placement: 'bottom' }"
          :disabled="nodePtr == null && inspectedNode == null"
          class="ml-1.5 hover:text-primary-700"
          :class="nodePtr != null ? 'text-gray-700' : 'text-gray-400'"
          @click="toggleHelperViewPin(spaceConnection.tx, spaceGraph, { self, nodePtr: inspectedNode })"
        >
          <i class="fas mr-1.5" :class="nodePtr == null ? 'fa-unlock' : 'fa-lock'" />
        </button>
        <!-- Meta & Controls  -->
        <div class="ml-auto flex flex-row items-center pl-1.5">
          <IconInline
            v-bind="ICON_BY_NODE_TYPE[inspectedNode.metatype as unknown as NodeType]"
            :shade="ColorShade.S500"
            class="mr-1 w-5 text-gray-500"
          />
          <span class="text-gray-500">{{ toCamelName(ObjectType, inspectedNode.metatype) }}</span>
        </div>
      </div>
    </div>
    <!-- Inspection content -->
    <Scroll
      id="scroll"
      :size="{ width: props.size.width, height: props.size.height - HEADER_HEIGHT }"
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
    </Scroll>
  </div>
  <div v-else class="flex h-full w-full flex-col justify-center text-center">
    <!-- Empty/missing state -->
    <span>
      <i class="fas fa-empty-set text-gray-500" />
      <span class="ml-1.5 text-gray-600">Select Node to Inspect</span>
    </span>
  </div>
</template>
