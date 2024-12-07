<script lang="ts" setup>
import { SINK_STEP_TYPES } from "@/language/const";
import { NAME_TYPE } from "@/language/field";
import {
  DEFAULT_TEXT_BY_STEP_TYPE,
  FLOW_PORT_SIZE,
  getStepSides,
  STEP_CONTEXT_ACTIONS,
  STEP_SIZE,
  useFlowContext,
} from "@/language/flow";
import { isRunActive } from "@/language/session";
import {
  ColorShade,
  FailStepData,
  FieldType,
  NodeType,
  Orientation,
  PortSide,
  StepType,
  Variant,
  ViewData,
} from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import { runtime } from "@/system/runtime";
import { canvas } from "@/system/space";
import type { ActionMapImplementation } from "@/ui/action";
import { getNodeIcon, IconInline } from "@/ui/icon";
import { menuActionsLike, PopoverInfoIn, type PopoverInfo } from "@/ui/popover";
import { getNodeColorHex, getRunColorHex } from "@/ui/style";
import { focusInElement } from "@/ui/view";
import RunStatus from "@/views/builtins/RunStatus.vue";
import { viewEmits, type ViewExposed } from "@/views/common";
import Icon from "@/views/content/Icon.vue";
import NativeInput from "@/views/content/NativeInput.vue";
import Text from "@/views/content/Text.vue";
import { MaybeElement } from "@vueuse/core";
import { computed, nextTick, ref, toRef, type Ref } from "vue";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; id: string } & Partial<
    Pick<ViewData, "name" | "title" | "icon" | "nodePtr" | "transform" | "variant">
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = toRef(props, "id");
const state = canvas.registerView(self, id);

const stepPtr = computed(() => props.nodePtr as TypedNodeReferenceData<NodeType.STEP>);
const flowCtx = useFlowContext();
const stepState = flowCtx.stepsStates.value[stepPtr.value.id!]; // must exist
const { step, subnode, fields, delegatePtr, delegate, delegateFields } = stepState;
const isInspected = computed(() => canvas.isInspected(stepPtr.value));
const isHighlighted = computed(() => canvas.isHighlighted(stepPtr.value));
const isSelected = computed(() => state.isSelected(stepPtr.value));
const sides = computed(() => (step.value != null ? getStepSides(step.value) : []));

const nameRef: Ref<InstanceType<typeof NativeInput> | null> = ref(null);
const containerRef: Ref<HTMLElement | null> = ref(null);

// run
const lastRuns = computed(() => runtime.focusedRunTree.getLastActiveRuns({ ck: stepPtr.value?.ck }));
const lastRun = computed(() => runtime.focusedRunTree.getLastActiveRun({ ck: stepPtr.value?.ck }));

//
// Interaction
//

// actions
const actions: Partial<ActionMapImplementation<"common">> & ActionMapImplementation<"step"> = {
  // common
  "common.edit.rename": {
    isEnabled: () => step.value?.type != StepType.TEXT,
    action: () => {
      nextTick(() => focusInElement(nameRef.value as MaybeElement));
    },
  },
};

defineExpose<ViewExposed>({ self, id, actions });
</script>
<template>
  <div
    v-if="step"
    ref="containerRef"
    class="group/step rounded border outline outline-1 transition-colors duration-150"
    :class="[
      isInspected || isHighlighted || isSelected ? 'border-gray-400 bg-gray-100' : 'border-gray-200 bg-white',
      lastRun != null && isRunActive(lastRun) ? '' : 'outline-transparent',
    ]"
    :style="{
      borderColor: lastRun != null ? getRunColorHex(lastRun.status) : '',
      outlineColor: lastRun != null && isRunActive(lastRun) ? getRunColorHex(lastRun.status) : '',
    }"
    @mouseup="(e) => flowCtx.endDragging(e, { kind: 'step', step: step! })"
  >
    <!-- Ports -->
    <div
      v-for="side in ['top', 'bottom', 'left', 'right']"
      v-if="!SINK_STEP_TYPES.includes(step.type)"
      class="absolute"
      :class="[
        side == 'top' ? '-top-2.5 left-1/2 -translate-x-1/2' : '',
        side == 'bottom' ? '-bottom-2.5 left-1/2 -translate-x-1/2' : '',
        side == 'left' ? '-left-1.5 top-1/2 -translate-y-1/2' : '',
        side == 'right' ? '-right-1.5 top-1/2 -translate-y-1/2' : '',
      ]"
    >
      <button
        class="relative cursor-crosshair rounded-2xl border bg-white opacity-0 outline-none transition-colors duration-150 hover:bg-gray-100 group-hover/step:opacity-100"
        :class="[isInspected || isHighlighted ? 'border-gray-400' : 'border-gray-200']"
        :style="{
          width: (side == 'top' || side == 'bottom' ? FLOW_PORT_SIZE * 2 : FLOW_PORT_SIZE) + 'px',
          height: (side == 'top' || side == 'bottom' ? FLOW_PORT_SIZE : FLOW_PORT_SIZE * 2) + 'px',
        }"
        @mousedown="(e) => flowCtx.startDragging(e, { kind: 'port', step: step!, side: PortSide.OUTGOING })"
        @mouseup="(e) => flowCtx.endDragging(e, { kind: 'port', step: step!, side: PortSide.INCOMING })"
      />
    </div>

    <!-- Regular step -->
    <div
      v-if="step.type != StepType.TEXT"
      ref="bodyRef"
      class="mx-1 flex w-full flex-row gap-x-2.5 py-1"
      :style="{
        height: STEP_SIZE.height + 'px',
      }"
    >
      <!-- NOTE :Incomplete: support :DelegateNodes -->
      <!-- Icon -->
      <div
        v-menu="
          (): PopoverInfoIn => ({
            component: Icon,
            placement: 'bottom-right',
            offset: '-referenceWidth',
            props: { modelValue: step?.icon, isInput: true },
            isEnabled: true,
            onApply: (newIcon) => flowCtx.tx.update(step!, { icon: newIcon }),
          })
        "
        v-tooltip="{ small: true, text: `Change icon` }"
        class="flex h-10 w-10 flex-shrink-0 items-center justify-center rounded hover:cursor-pointer"
        :style="{
          backgroundColor: getNodeColorHex(step, ColorShade.S300),
        }"
      >
        <IconInline ref="iconRef" v-bind="getNodeIcon(step)" class="rounded text-center text-lg text-gray-700" />
      </div>
      <!-- Main -->
      <div
        class="flex flex-1 flex-col"
        :style="{
          maxWidth: `calc(100% - 60px)`,
        }"
      >
        <!-- Header -->
        <div class="flex flex-row gap-x-1.5">
          <!-- Name -->
          <NativeInput
            id="name"
            ref="nameRef"
            class="flex-shrink-0 font-medium transition-colors duration-150"
            placeholder="Step"
            is-input
            :value-type="NAME_TYPE"
            :variant="Variant.STEALTH"
            :model-value="step.name"
            @update:model-value="
              (newValue) => flowCtx.tx.update(step!, { name: newValue as string }, { debounce: 'long' })
            "
          />
          <!-- Link (if delegate) -->
          <button
            v-if="delegate"
            class="rounded text-gray-400 hover:bg-gray-100 hover:text-gray-700"
            @click.stop="canvas.goToNode(delegate)"
          >
            <i class="fas fa-arrow-up-right" />
          </button>
          <!-- Controls/Meta -->
          <div class="ml-auto flex flex-row pl-2 pr-1.5">
            <!-- Run status -->
            <button v-if="lastRun != null" class="rounded hover:bg-gray-100">
              <RunStatus :run="lastRun" :orientation="Orientation.HORIZONTAL_REVERSED" />
            </button>
          </div>
        </div>
        <!-- Body -->
        <div class="flex max-w-full flex-row items-center gap-x-1 truncate text-gray-700">
          <!-- Delegate -->
          <!-- ...? -->
          <!-- Fields -->
          <!-- NOTE :Incomplete: better Step body -->
          <span
            v-for="field in fields.filter((f) => f.type == FieldType.INPUT)"
            :key="field.id"
            class="truncate transition-colors duration-75"
            :class="canvas.isHighlighted(field) ? 'text-gray-700' : 'text-gray-400'"
          >
            {{ field.name }}
          </span>
          <i v-if="fields.length != 0" class="fas fa-arrow-right text-xs text-gray-400" />
          <span
            v-for="field in fields.filter((f) => f.type == FieldType.OUTPUT)"
            :key="field.id"
            class="truncate transition-colors duration-75"
            :class="canvas.isHighlighted(field) ? 'text-gray-700' : 'text-gray-400'"
          >
            {{ field.name }}
          </span>
          <span v-if="fields.length == 0" class="text-gray-400">
            <template v-if="step.type == StepType.FAIL && (subnode as FailStepData).errorTitle != null">
              {{ (subnode as FailStepData).errorTitle }}
            </template>
            <template v-else>{{ DEFAULT_TEXT_BY_STEP_TYPE[step.type] ?? "No fields" }}</template>
          </span>
        </div>
      </div>
    </div>
    <!-- Text -->
    <div v-else ref="bodyRef" class="relative px-3 py-1">
      <!-- Content (:StepHeight) -->
      <Text
        id="text"
        class=""
        placeholder="Text..."
        is-input
        :variant="Variant.STEALTH"
        :model-value="step.text"
        @update:model-value="(newText) => flowCtx.tx.update(step!, { text: newText }, { debounce: 'long' })"
      />
    </div>

    <!-- Floating Menu -->
    <div class="absolute -left-5 top-0 flex -translate-x-1 flex-row gap-x-1.5">
      <button
        v-menu="
          (): PopoverInfo => ({
            kind: 'menu',
            placement: 'bottom-left',
            offset: 'referenceWidth',
            items: menuActionsLike(STEP_CONTEXT_ACTIONS, { context: { triggerNode: stepPtr } }),
          })
        "
        class="text-gray-400 opacity-0 transition-colors duration-75 hover:text-gray-700 group-hover/step:opacity-100 data-[popover=true]:text-gray-700 data-[popover=true]:opacity-100"
      >
        <i class="fas fa-ellipsis-vertical w-5 text-center" />
      </button>
    </div>

    <!-- NOTE :UX: show last step output/error here? -->
  </div>
  <div v-else ref="containerRef" class="rounded border border-gray-200 bg-white">
    <!-- should never be rendered by containing flow -->
    <span class="text-danger-600">???</span>
  </div>
</template>
