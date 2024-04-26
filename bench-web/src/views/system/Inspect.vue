<script lang="ts" setup>
import {
  BoxData,
  ColorShade,
  NodeType,
  ObjectType,
  Orientation,
  PROPERTY_ENUM_BY_TYPE,
  ViewData,
  ViewType,
} from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import { useExistingConnection } from "@/system/connection";
import { ICON_BY_NODE_TYPE, IconInline, getNodeIcon } from "@/system/icon";
import { getInspectionLayout, toCamelName } from "@/system/lang";
import { canvas, inspectionPtr } from "@/system/space";
import { ScrollbarWidth } from "@/utils/layout";
import type { OverlayMenuInfoIn } from "@/utils/menu";
import { getViewComponent } from "@/views/registry";
import { viewEmits, type ViewExposed } from "@/views/common";
import Scroll from "@/views/containers/Scroll.vue";
import { computed, toRef } from "vue";

const MIN_WIDTH = 320;
const MAX_WIDTH = 540;
const HEADER_HEIGHT = 40;

const props = defineProps<
  { self: TypedNodeReferenceData<NodeType.VIEW>; size: Required<Pick<BoxData, "width" | "height">> } & Pick<
    ViewData,
    "name" | "title" | "text" | "icon" | "nodePtr"
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");

const { graph: spaceGraph, connection: spaceConnection } = useExistingConnection(self);
const { graph: pkgGraph, connection: pkgConnection } = useExistingConnection(inspectionPtr);
const node = pkgGraph.getRef(inspectionPtr);
const nodeMetatype = computed(() => node.value?.metatype);
const nodeProperties = computed(() => (nodeMetatype.value != null ? PROPERTY_ENUM_BY_TYPE[nodeMetatype.value] : null));
const ancestors = pkgGraph.getAncestorsRef(node, { includeSelf: true });
const path = computed(() => ancestors.value.slice().reverse());

const inspectionLayout = computed(() => {
  if (nodeMetatype.value == null) return null;
  const layout = getInspectionLayout(nodeMetatype.value, { exclude: ["icon", "name"] /* separate in header */ });
  return layout;
});

canvas.registerView(self);
defineExpose<ViewExposed>({ self });
</script>
<template>
  <div v-if="node && inspectionLayout" class="h-full w-full bg-white">
    <!-- Header -->
    <div class="group w-full border-b border-gray-200" :style="{ height: HEADER_HEIGHT + 'px' }">
      <div
        class="mx-auto flex h-full max-w-full flex-row items-center pl-4 pr-5"
        :style="{ minWidth: MIN_WIDTH + 'px', maxWidth: MAX_WIDTH + 'px' }"
      >
        <!-- Icon -->
        <IconInline
          v-bind="getNodeIcon(node)"
          class="w-6 rounded border border-transparent p-1 text-gray-700 hover:cursor-pointer hover:bg-primary-100 data-[menu=true]:border-primary-900 data-[menu=true]:bg-primary-100"
          v-menu="
            (): OverlayMenuInfoIn => ({
              component: ViewType.ICON,
              placement: 'bottom-right',
              offset: '-referenceWidth',
              props: { modelValue: getNodeIcon(node!) },
              isEnabled: nodeProperties != null && 'icon' in nodeProperties,
              onApply: (newIcon) => pkgConnection.tx.update(node!, { icon: newIcon }),
            })
          "
        />
        <!-- Name -->
        <input
          class="ml-1 truncate rounded border-0 px-1 py-0.5 font-medium outline-none ring-0 hover:bg-primary-100 hover:text-primary-900 focus:ring-0"
          spellcheck="false"
          :value="'name' in node ? node.name : toCamelName(ObjectType, node.metatype)"
          :disabled="!('name' in node)"
          @input="
            (event) => pkgConnection.tx.updateDebounced(node!, { name: (event.target as HTMLInputElement).value })
          "
        />
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
      :size="{ width: props.size.width, height: props.size.height - HEADER_HEIGHT }"
      :orientation="Orientation.VERTICAL"
      :track-width="ScrollbarWidth.sm"
      track-is-overlay
    >
      <ul class="flex flex-col gap-y-2.5 py-3">
        <template
          v-for="(
            { title, protoName, category, property, viewType, props, isFullWidth, read, write }, i
          ) of inspectionLayout.properties"
          :key="property.id"
        >
          <!-- Category Header -->
          <div v-if="i != 0 && inspectionLayout.properties[i - 1].category != category" class="mt-2">
            <div class="mb-3 h-[1px] w-full min-w-fit bg-gray-200" />
            <div
              class="mx-auto px-5 font-semibold text-gray-900"
              :style="{ minWidth: MIN_WIDTH + 'px', maxWidth: MAX_WIDTH + 'px' }"
            >
              {{ category }}
            </div>
          </div>
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
              v-if="viewType != null && getViewComponent(viewType)"
              :is="getViewComponent(viewType)"
              class="ml-auto flex-shrink-0"
              :style="{ width: isFullWidth ? '100%' : 'calc(90% - 100px)' }"
              v-bind="props"
              :modelValue="read != null ? read(node) : (node as any)[protoName!]"
              @update:modelValue="
                (value: any) => {
                  if (write != null) write(pkgConnection.tx, node!, value);
                  else pkgConnection.tx.updateDebounced(node!, { [protoName!]: value });
                }
              "
            />
            <div v-else class="ml-auto text-danger-600">
              {{ viewType != null ? ViewType[viewType] : "???" }}
            </div>
          </li>
        </template>
      </ul>
    </Scroll>
  </div>
  <div v-else class="flex h-full w-full flex-col justify-center bg-white text-center">
    <!-- Empty/missing state -->
    <i class="fas fa-empty-set text-gray-500" />
    <span class="text-gray-600">Select Node to Inspect</span>
  </div>
</template>
