<script lang="ts" setup>
import { ViewData, NodeType, StepType, Variant, PortSide, BenchType, PortType } from "@/proto/wire";
import { unwrapProtoOneOf, type TypedNodeReferenceData } from "@/proto/wiring";
import { makeViewId, viewEmits, type ViewExposed } from "@/views/common";
import { canvas, inspectionPtr } from "@/system/space";
import { computed, ref, toRef, type Ref } from "vue";
import { useExistingConnection, type PreparedGetConnection } from "@/system/connection";
import { VIEW_DEFAULT_HEADER_HEIGHT } from "@/ui/view";
import { useElementSize } from "@vueuse/core";
import { FLOW_GRID_STEP_Y, FLOW_PORT_SIZE, STEP_CONTEXT_ACTIONS, useFlowContext } from "@/language/flow";
import { menuActionsLike, type PopoverInfo, type PopoverInfoIn } from "@/ui/popover";
import Icon from "@/views/content/Icon.vue";
import { getNodeIcon, IconInline } from "@/ui/icon";
import Inaccessible from "@/views/builtins/Inaccessible.vue";
import { getNativeConstraintProps, guardNativeInput } from "@/ui/view";
import { NAME_CONSTRAINT } from "@/language/const";
import type { TooltipInfo } from "@/ui/tooltip";
import type { ActionMapImplementation } from "@/ui/action";
import { nextTick } from "vue";
import Code from "@/views/content/Code.vue";
import Text from "@/views/content/Text.vue";
import Picker from "@/views/content/Picker.vue";
import { makeTypeInfo } from "@/language/field";
import Field from "@/views/system/Field.vue";

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
const stepPtr = computed(() => unwrapProtoOneOf(props.nodePtr) as TypedNodeReferenceData<NodeType.STEP>);
const pkgGetConnection = props.preparedConnection ?? useExistingConnection(stepPtr);
const { graph: pkgGraph, connection: pkgConnection } = pkgGetConnection;
const ctx = useFlowContext();

const step = pkgGraph.getRef(stepPtr, { ignoreAncestors: props.self == null });
const nodePtr = computed(() => step.value?.nodePtr as TypedNodeReferenceData<NodeType.BLOCK | NodeType.STEP> | null);
const node = pkgGraph.getRef(nodePtr);
const nodeFields = pkgGraph.getChildrenRef(node, NodeType.FIELD);
const fields = pkgGraph.getChildrenRef(step, NodeType.FIELD);
const ports = computed(() =>
  step.value != null
    ? ctx.getPorts(step.value, { fields: fields.value, node: node.value!, nodeFields: nodeFields.value })
    : { incoming: [], outgoing: [] },
);

const nameRef: Ref<HTMLInputElement | null> = ref(null);
const containerRef: Ref<HTMLElement | null> = ref(null);
const bodyRef: Ref<HTMLElement | null> = ref(null);
const headerRef: Ref<HTMLElement | null> = ref(null);
const contentSize = useElementSize(bodyRef, undefined, { box: "border-box" });
const paddingHeight = computed(
  () => FLOW_GRID_STEP_Y - ((contentSize.height.value + HEADER_HEIGHT) % FLOW_GRID_STEP_Y),
);

//
// Interaction
//

const actions: Partial<ActionMapImplementation<"common">> & ActionMapImplementation<"step"> = {
  // common
  "common.edit.rename": {
    action: () => {
      nextTick(() => nameRef.value!.focus());
    },
  },
};

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id, actions });
</script>
<template>
  <div
    v-if="step"
    ref="containerRef"
    class="rounded border bg-white"
    :class="[stepPtr?.id == inspectionPtr?.id ? 'border-primary-900' : 'border-gray-200']"
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
          class="ml-0.5 w-fit min-w-fit max-w-fit rounded border-0 px-1 font-medium text-gray-700 outline-none ring-0 hover:bg-gray-100 focus:ring-0"
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
        <button
          v-menu="
            (): PopoverInfo => ({
              kind: 'menu',
              placement: 'bottom-left',
              offset: 'referenceWidth',
              items: menuActionsLike(STEP_CONTEXT_ACTIONS, { context: { triggerNode: stepPtr } }),
            })
          "
          class="rounded text-gray-400 hover:bg-gray-100 hover:text-primary-900 data-[popover=true]:bg-gray-100 data-[popover=true]:text-primary-900"
        >
          <i class="fas fa-ellipsis-v w-5 text-center" />
        </button>
      </div>
    </div>

    <!-- Body -->
    <div ref="bodyRef" class="">
      <!-- Ports -->
      <div
        class="relative w-full"
        :style="{
          marginTop: '6px',
          height: FLOW_GRID_STEP_Y * Math.max(ports.incoming.length, ports.outgoing.length) + 'px',
        }"
      >
        <!-- Incoming & outgoing ports -->
        <div
          v-for="port in [...ports.incoming, ...ports.outgoing]"
          :key="`${port.side}-${port.idx}`"
          class="absolute flex w-20 items-center"
          :class="[port.side == PortSide.INCOMING ? 'justify-start' : 'justify-end']"
          :style="{
            height: FLOW_GRID_STEP_Y + 'px',
            left: port.side == PortSide.INCOMING ? '0px' : undefined,
            right: port.side == PortSide.OUTGOING ? '0px' : undefined,
            top: port.idx * FLOW_GRID_STEP_Y + 'px',
          }"
        >
          <!-- Actual 'port' -->
          <button
            class="absolute rounded-sm border bg-white"
            :class="[stepPtr?.id == inspectionPtr?.id ? 'border-primary-900' : 'border-gray-200']"
            :style="{
              height: FLOW_PORT_SIZE + 'px',
              width: FLOW_PORT_SIZE + 'px',
              left: port.side == PortSide.INCOMING ? -FLOW_PORT_SIZE / 2 + 'px' : undefined,
              right: port.side == PortSide.OUTGOING ? -FLOW_PORT_SIZE / 2 + 'px' : undefined,
              top: FLOW_GRID_STEP_Y / 2 - FLOW_PORT_SIZE / 2 + 'px',
            }"
          />
          <!-- Port content -->
          <div v-if="port.type == PortType.RUN">
            <i class="fas fa-bolt text-gray-400" />
          </div>
        </div>
      </div>

      <!-- Content -->
      <div v-if="[StepType.TEXT, StepType.CODE].includes(step.type)" class="mt-1 border-t border-gray-200 pt-1">
        <Text
          v-if="step.type == StepType.TEXT"
          class="px-3"
          is-input
          :variant="Variant.STEALTH"
          :model-value="step.text"
          @update:model-value="(newText) => pkgConnection.tx.update(step!, { text: newText }, { debounce: 'long' })"
        />
        <Code
          v-else-if="step.type == StepType.CODE"
          class=""
          is-input
          :variant="Variant.STEALTH"
          :model-value="step.code"
          @update:model-value="(newCode) => pkgConnection.tx.update(step!, { code: newCode }, { debounce: 'long' })"
        />
      </div>
    </div>

    <!-- Padding (to ensure height is a multiple of the grid) -->
    <div class="w-full border-t-gray-200" :style="{ height: paddingHeight + 'px' }" />
  </div>
  <div v-else ref="containerRef" class="rounded border border-gray-200 bg-white">
    <Inaccessible :node="stepPtr" :connection="pkgConnection" />
  </div>
</template>
