<script lang="ts" setup>
import { BoxData, ColorShade, NodeType, ObjectType, Orientation, ViewData, ViewType } from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import { useExistingConnection } from "@/system/connection";
import { ICON_BY_NODE_TYPE, IconInline } from "@/system/icon";
import { getInspectionLayout, getNodeSubtype, toCamelName } from "@/system/lang";
import { canvas, inspectionPtr } from "@/system/space";
import { ScrollbarWidth } from "@/utils/layout";
import NodeCrumb from "@/views/builtins/NodeCrumb.vue";
import { DEFAULT_HEADER_HEIGHT, DEFAULT_MAX_WIDTH, DEFAULT_MIN_WIDTH } from "@/views/canvas";
import { viewEmits, type ViewExposed } from "@/views/common";
import Scroll from "@/views/containers/Scroll.vue";
import { getViewComponent, hasViewComponent } from "@/views/registry";
import { computed, toRef } from "vue";

const HEADER_HEIGHT = DEFAULT_HEADER_HEIGHT;
const MIN_WIDTH = DEFAULT_MIN_WIDTH;
const MAX_WIDTH = DEFAULT_MAX_WIDTH;

const props = defineProps<
  { self: TypedNodeReferenceData<NodeType.VIEW>; size: Required<Pick<BoxData, "width" | "height">> } & Pick<
    ViewData,
    "name" | "title" | "text" | "icon" | "nodePtr"
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");

const { graph: spaceGraph } = useExistingConnection(self);
const { graph: pkgGraph, connection: pkgConnection } = useExistingConnection(inspectionPtr);
const node = pkgGraph.getRef(inspectionPtr);
const nodeMetatype = computed(() => node.value?.metatype);
const nodeSubtype = computed(() => (node.value != null ? getNodeSubtype(node.value) : null));
const ancestors = pkgGraph.getAncestorsRef(node, { includeSelf: true });

const inspectionLayout = computed(() => {
  if (nodeMetatype.value == null) return null;
  const layout = getInspectionLayout(nodeMetatype.value, nodeSubtype.value, {
    exclude: ["icon", "name"] /* separate in header */,
  });
  return layout;
});

canvas.registerView(self);
defineExpose<ViewExposed>({ self });
</script>
<template>
  <div v-if="node && inspectionLayout" class="h-full w-full bg-white">
    <!-- Header -->
    <div class="group w-full" :style="{ height: HEADER_HEIGHT + 'px' }">
      <div
        class="mx-auto flex h-full max-w-full flex-row items-center pl-2 pr-2.5"
        :style="{ minWidth: MIN_WIDTH + 'px' }"
      >
        <!-- Node -->
        <NodeCrumb :node="node" :connection="pkgConnection" class="font-medium" />
        <!-- Meta & Controls  -->
        <div class="ml-auto flex flex-row items-center pl-1.5">
          <IconInline
            v-bind="ICON_BY_NODE_TYPE[node.metatype as unknown as NodeType]"
            :shade="ColorShade.S500"
            class="mr-1 w-5 text-gray-500"
          />
          <span class="text-gray-500">{{ toCamelName(ObjectType, node.metatype) }}</span>
        </div>
      </div>
    </div>
    <!-- Inspection content -->
    <Scroll
      :size="{ width: props.size.width, height: props.size.height - DEFAULT_HEADER_HEIGHT }"
      :orientation="Orientation.VERTICAL"
      :track-width="ScrollbarWidth.sm"
      track-is-overlay
    >
      <ul class="flex flex-col gap-y-2.5 py-3">
        <template
          v-for="(
            { title, protoName, category, property, viewType, props: viewProps, isFullWidth, read, write }, i
          ) of inspectionLayout.properties"
          :key="property.id"
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
            :class="[isFullWidth ? 'flex flex-col' : 'flex flex-row flex-wrap items-center gap-x-[10%]']"
            :style="{ minWidth: MIN_WIDTH + 'px', maxWidth: MAX_WIDTH + 'px' }"
          >
            <!-- Title & Controls -->
            <span class="w-[100px]">
              <span class="max-w-full truncate py-1 font-medium text-gray-700">{{ title }}</span>
            </span>
            <!-- Value -->
            <component
              :is="getViewComponent(viewType)"
              v-if="viewType != null && hasViewComponent(viewType)"
              :class="['ml-auto flex-shrink-0', isFullWidth ? '' : 'text-right']"
              :style="{ width: isFullWidth ? '100%' : 'calc(90% - 100px)' }"
              v-bind="{ ...viewProps, isInput: true }"
              :model-value="read != null ? read(node) : (node as any)[protoName!]"
              @update:model-value="
                (value: any) => {
                  // not sure how to :DebounceNestedValue properly (different types with different debounce needs)
                  if (write != null) write(pkgConnection.tx, node!, value);
                  else pkgConnection.tx.update(node!, { [protoName!]: value }, { debounce: 'short' });
                  inspectionLayout?.onWrite?.(pkgConnection.tx, pkgGraph, node!, property);
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
  <div v-else class="flex h-full w-full flex-col justify-center bg-white text-center">
    <!-- Empty/missing state -->
    <span>
      <i class="fas fa-empty-set text-gray-500" />
      <span class="ml-1.5 text-gray-600">Select Node to Inspect</span>
    </span>
  </div>
</template>
