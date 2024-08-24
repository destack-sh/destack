<script lang="ts" setup>
import { ViewData, NodeType } from "@/proto/wire";
import { unwrapProtoOneOf, type TypedNodeReferenceData } from "@/proto/wiring";
import { makeViewId, viewEmits, type ViewExposed } from "@/views/common";
import { canvas, inspectionPtr } from "@/system/space";
import { computed, ref, toRef, type Ref } from "vue";
import { useExistingConnection, type PreparedGetConnection } from "@/system/connection";
import { VIEW_DEFAULT_HEADER_HEIGHT } from "@/ui/view";
import { useElementSize } from "@vueuse/core";
import { FLOW_GRID_STEP_Y } from "@/language/flow";
import type { PopoverInfoIn } from "@/ui/popover";
import Icon from "@/views/content/Icon.vue";
import { getNodeIcon, IconInline } from "@/ui/icon";
import Inaccessible from "@/views/builtins/Inaccessible.vue";
import { getNativeConstraintProps, guardNativeInput } from "@/ui/view";
import { NAME_CONSTRAINT } from "@/language/const";
import type { TooltipInfo } from "@/ui/tooltip";

const HEADER_HEIGHT = VIEW_DEFAULT_HEADER_HEIGHT;

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; preparedConnection?: PreparedGetConnection } & Partial<
    Pick<ViewData, "name" | "title" | "text" | "icon" | "nodePtr" | "transform" | "variant">
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = makeViewId(props);

const { graph: spaceGraph, connection: spaceConnection } = useExistingConnection(self, { isRequired: false });
const selfView = spaceGraph.getRef(self);
const nodePtr = computed(() => unwrapProtoOneOf(props.nodePtr) as TypedNodeReferenceData<NodeType.STEP>);
const pkgGetConnection = props.preparedConnection ?? useExistingConnection(nodePtr);
const { graph: pkgGraph, connection: pkgConnection } = pkgGetConnection;
const step = pkgGraph.getRef(nodePtr, { ignoreAncestors: props.self == null });

const containerRef: Ref<HTMLElement | null> = ref(null);
const contentRef: Ref<HTMLElement | null> = ref(null);
const headerRef: Ref<HTMLElement | null> = ref(null);
const contentSize = useElementSize(contentRef, undefined, { box: "border-box" });
const paddingHeight = computed(
  () => FLOW_GRID_STEP_Y - ((contentSize.height.value + HEADER_HEIGHT) % FLOW_GRID_STEP_Y),
);

//
// Interaction
//

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id });
</script>
<template>
  <div
    v-if="step"
    ref="containerRef"
    class="rounded border bg-white"
    :class="[nodePtr?.id == inspectionPtr?.id ? 'border-primary-900' : 'border-gray-200']"
  >
    <!-- nocheckin: Step view -->
    <!-- Header -->
    <div
      ref="headerRef"
      class="flex w-full flex-row items-center border-b border-gray-200 px-2"
      :style="{ height: HEADER_HEIGHT + 'px' }"
    >
      <!-- Icon/Name -->
      <div>
        <IconInline
          v-tooltip="{ small: true, text: `Change icon` } as TooltipInfo"
          v-menu="
            (): PopoverInfoIn => ({
              component: Icon,
              placement: 'bottom-right',
              offset: '-referenceWidth',
              props: { modelValue: step!.icon, isInput: true },
              onApply: (newIcon) => pkgConnection.tx.update(step!, { icon: newIcon }),
            })
          "
          v-bind="getNodeIcon(step)"
          class="w-5 rounded py-0.5 text-gray-700 hover:cursor-pointer hover:bg-gray-100 data-[popover=true]:bg-gray-100"
        />
        <input
          ref="nameRef"
          type="text"
          class="w-fit min-w-fit max-w-fit rounded border-0 px-1 font-medium text-gray-700 outline-none ring-0 hover:bg-gray-100 focus:ring-0"
          spellcheck="false"
          :value="step.name"
          :size="step.name.length + 3"
          v-bind="getNativeConstraintProps(NAME_CONSTRAINT)"
          @input="
            guardNativeInput(NAME_CONSTRAINT, $event, step!.name, (newValue) =>
              pkgConnection.tx.update(step!, { name: newValue }, { debounce: 'long' }),
            )
          "
        />
      </div>
      <!-- Controls -->
      <div class="ml-auto pl-2 pr-0.5">
        <!-- ... -->
      </div>
    </div>
    <!-- Content -->
    <div ref="contentRef" class="min-h-20 p-2">
      <!-- ... -->
    </div>
    <!-- Padding (to ensure height is a multiple of the grid) -->
    <div class="w-full" :style="{ height: paddingHeight + 'px' }" />
  </div>
  <div v-else ref="containerRef" class="rounded border border-gray-200 bg-white">
    <Inaccessible :node="nodePtr" :connection="pkgConnection" />
  </div>
</template>
