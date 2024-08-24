<script lang="ts" setup>
import { NAME_CONSTRAINT, toCamelName } from "@/language/const";
import { FLOW_GRID_STEP_Y, FLOW_PORT_SIZE, STEP_CONTEXT_ACTIONS, useFlowContext } from "@/language/flow";
import { Alignment, NodeType, Orientation, PortSide, PortType, StepType, Variant, ViewData } from "@/proto/wire";
import { toNodeRefOneOf, unwrapProtoOneOf, type TypedNodeReferenceData } from "@/proto/wiring";
import { useExistingConnection, type PreparedGetConnection } from "@/system/connection";
import { canvas, inspectionPtr } from "@/system/space";
import type { ActionMapImplementation } from "@/ui/action";
import { getNodeIcon, IconInline } from "@/ui/icon";
import { menuActionsLike, type PopoverInfo, type PopoverInfoIn } from "@/ui/popover";
import type { TooltipInfo } from "@/ui/tooltip";
import { getNativeConstraintProps, guardNativeInput, VIEW_DEFAULT_HEADER_HEIGHT } from "@/ui/view";
import Inaccessible from "@/views/builtins/Inaccessible.vue";
import { makeViewId, viewEmits, type ViewExposed } from "@/views/common";
import Code from "@/views/content/Code.vue";
import Icon from "@/views/content/Icon.vue";
import Text from "@/views/content/Text.vue";
import Field from "@/views/system/Field.vue";
import { useElementSize } from "@vueuse/core";
import { computed, nextTick, ref, toRef, type Ref } from "vue";

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
    class="group/step rounded border bg-white transition-colors duration-75"
    :class="[stepPtr?.id == inspectionPtr?.id ? 'border-primary-900' : 'border-gray-200 hover:border-gray-300']"
  >
    <!-- Header -->
    <div
      ref="headerRef"
      class="flex w-full flex-row items-center border-b border-gray-200 px-2"
      :style="{ height: HEADER_HEIGHT + 'px' }"
    >
      <!-- Icon/Name -->
      <div class="flex-shrink-0">
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
          // ensure ports are aligned with grid (offset by half a step so that the lines connect in the middle)
          marginTop: FLOW_GRID_STEP_Y - (HEADER_HEIGHT % FLOW_GRID_STEP_Y) - FLOW_GRID_STEP_Y / 2 + 'px',
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
            class="absolute rounded-sm border bg-white transition-colors duration-75"
            :class="[
              stepPtr?.id == inspectionPtr?.id
                ? 'border-primary-900'
                : 'border-gray-200 group-hover/step:border-gray-300',
            ]"
            :style="{
              height: FLOW_PORT_SIZE + 'px',
              width: FLOW_PORT_SIZE + 'px',
              left: port.side == PortSide.INCOMING ? -FLOW_PORT_SIZE / 2 + 'px' : undefined,
              right: port.side == PortSide.OUTGOING ? -FLOW_PORT_SIZE / 2 + 'px' : undefined,
              top: FLOW_GRID_STEP_Y / 2 - FLOW_PORT_SIZE / 2 + 'px',
            }"
          />
          <!-- Port content -->
          <div v-if="port.type == PortType.RUN" class="px-2.5">
            <i class="fas fa-play w-5 text-center text-gray-700" />
          </div>
          <div v-else-if="port.type == PortType.FIELD" class="px-1">
            <Field
              :node-ptr="toNodeRefOneOf(port.field!)"
              :variant="Variant.STEALTH"
              :orientation="port.side == PortSide.INCOMING ? Orientation.HORIZONTAL : Orientation.HORIZONTAL_REVERSED"
            />
          </div>
          <div v-else>
            <span class="text-danger-600">{{ toCamelName(PortType, port.type) }}</span>
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
    <div class="w-full" :style="{ height: paddingHeight + 'px' }" />
  </div>
  <div v-else ref="containerRef" class="rounded border border-gray-200 bg-white">
    <Inaccessible :node="stepPtr" :connection="pkgConnection" />
  </div>
</template>
