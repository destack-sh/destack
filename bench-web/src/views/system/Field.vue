<script lang="ts" setup>
import { NAME_CONSTRAINT } from "@/language/const";
import { NodeType, Orientation, Variant, ViewData } from "@/proto/wire";
import { unwrapProtoOneOf, type TypedNodeReferenceData } from "@/proto/wiring";
import { useExistingConnection, type PreparedGetConnection } from "@/system/connection";
import { canvas } from "@/system/space";
import type { ActionMapImplementation } from "@/ui/action";
import { IconInline, getNodeIcon } from "@/ui/icon";
import { type PopoverInfoIn } from "@/ui/popover";
import type { TooltipInfo } from "@/ui/tooltip";
import { getNativeConstraintProps, guardNativeNameInput } from "@/ui/view";
import Inaccessible from "@/views/builtins/Inaccessible.vue";
import { makeViewId, viewEmits, type ViewExposed } from "@/views/common";
import Icon from "@/views/content/Icon.vue";
import { computed, nextTick, ref, toRef } from "vue";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; preparedConnection?: PreparedGetConnection } & Pick<
    ViewData,
    "variant" | "nodePtr" | "orientation"
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = makeViewId(props);

const fieldRef = ref<HTMLElement | null>(null);
const nameRef = ref<HTMLElement | null>(null);

const nodePtr = computed(() => unwrapProtoOneOf(props.nodePtr) as TypedNodeReferenceData<NodeType.FIELD>);
const { graph: pkgGraph, connection: pkgConnection } = props.preparedConnection ?? useExistingConnection(nodePtr);
const field = pkgGraph.getRef(nodePtr, { ignoreAncestors: props.self == null });
const isInspected = computed(() => canvas.isInspected(nodePtr.value));
const isHighlighted = computed(() => canvas.isHighlighted(nodePtr.value));

// actions
const actions: Partial<ActionMapImplementation<"common">> & ActionMapImplementation<"type"> = {
  // common
  "common.edit.rename": {
    action: () => {
      nextTick(() => nameRef.value!.focus());
    },
  },
  // type
  "type.edit.isList": {
    isChecked: () => field.value?.isList ?? false,
    action: () => {
      if (field.value == null) return;
      pkgConnection.tx.update(field.value!, { isList: !field.value!.isList });
    },
  },
  "type.edit.isRequired": {
    isChecked: () => field.value?.isRequired ?? false,
    action: () => {
      if (field.value == null) return;
      pkgConnection.tx.update(field.value!, { isRequired: !field.value!.isRequired });
    },
  },
  "type.edit.isSecret": {
    isChecked: () => field.value?.isSecret ?? false,
    action: () => {
      if (field.value == null) return;
      pkgConnection.tx.update(field.value!, { isSecret: !field.value!.isSecret });
    },
  },
};

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id, actions });
</script>
<template>
  <div
    v-if="field"
    ref="fieldRef"
    class="flex w-fit items-center gap-x-1.5 rounded px-1.5 py-[3px] transition-colors duration-75"
    :class="[
      orientation != Orientation.HORIZONTAL_REVERSED ? 'flex-row' : 'flex-row-reverse',
      variant != Variant.STEALTH ? 'border border-gray-200 bg-gray-100' : 'hover:bg-gray-100',
      isInspected
        ? variant != Variant.STEALTH
          ? 'border-primary-900'
          : 'text-primary-900'
        : isHighlighted
          ? variant != Variant.STEALTH
            ? 'border-primary-400'
            : 'text-primary-700'
          : '',
    ]"
  >
    <!-- TODO :UX: Field is annoying (should be double-click to edit, change type in contextmenu, indicate metadata, ...) -->
    <IconInline
      v-tooltip="{ small: true, text: `Change icon` } as TooltipInfo"
      v-menu="
        (): PopoverInfoIn => ({
          component: Icon,
          placement: 'bottom-right',
          offset: '-referenceWidth',
          props: { modelValue: field!.icon },
          onApply: (newIcon) => pkgConnection.tx.update(field!, { icon: newIcon }),
        })
      "
      v-bind="getNodeIcon(field)"
      class="w-5 rounded p-0.5 hover:cursor-pointer hover:bg-gray-100 data-[popover=true]:bg-gray-100"
      :class="
        isInspected && variant == Variant.STEALTH
          ? 'text-primary-900'
          : isHighlighted && variant == Variant.STEALTH
            ? 'text-primary-700'
            : 'text-gray-700'
      "
    />
    <input
      ref="nameRef"
      type="text"
      class="w-fit min-w-fit max-w-fit truncate rounded border-0 bg-transparent font-medium outline-none ring-0 hover:bg-gray-100 focus:ring-0"
      :class="orientation == Orientation.HORIZONTAL_REVERSED ? 'text-right' : ''"
      spellcheck="false"
      :value="field.name"
      :size="(field.name?.length ?? 0) + 3"
      v-bind="getNativeConstraintProps(NAME_CONSTRAINT)"
      @input="
        guardNativeNameInput($event, field.name, (newValue) =>
          pkgConnection.tx.update(field!, { name: newValue }, { debounce: 'long' }),
        )
      "
    />
  </div>
  <Inaccessible v-else class="bg-white" :node="nodePtr" :connection="pkgConnection" />
</template>
