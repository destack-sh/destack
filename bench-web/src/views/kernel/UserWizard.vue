<script lang="ts" setup>
import { makeTypeInfo } from "@/language/field";
import {
  NodeType,
  ObjectType,
  Region,
  UserWizardViewStage,
  Variant,
  ViewData,
  type NodeReferenceData,
} from "@/proto/wire";
import { packProtoJson, type TypedNodeReferenceData } from "@/proto/wiring";
import { benchPtr } from "@/system/client";
import { useExistingConnection } from "@/system/connection";
import { canvas, goToBench } from "@/system/space";
import { logIn, signUp, user } from "@/system/user";
import { getViewComponentChildren, isVueInstanceOf } from "@/ui/view";
import { makeIcon } from "@/ui/icon";
import { viewEmits, type FocusAnchor, type ViewExposed } from "@/views/common";
import NativeInput from "@/views/content/NativeInput.vue";
import Button from "@/views/controls/Button.vue";
import { ref, toRef, type Ref } from "vue";
import { useViewState } from "@/ui/view";

const props = defineProps<{ self: TypedNodeReferenceData<NodeType.VIEW> } & Pick<ViewData, "title" | "valuePacked">>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");

const { graph: spaceGraph } = useExistingConnection(toRef(props, "self"));

const { state, updateState, useStateProp, packStateUpdate } = useViewState({
  selfPtr: self,
  graph: spaceGraph,
  stateType: ObjectType.USER_WIZARD_VIEW_STATE,
  props,
  emit,
});
const stage = useStateProp(canvas.tx, "stage", UserWizardViewStage.LOG_IN);
const name: Ref<string> = ref("");
const slug: Ref<string> = ref("");
const email: Ref<string> = ref("");
const password: Ref<string> = ref("");
const region: Ref<Region> = ref(Region.FRANKFURT);
const isActive = ref(false);
const lastError: Ref<string | null> = ref(null);

function clear() {
  name.value = "";
  slug.value = "";
  email.value = "";
  password.value = "";
}

function switchStage() {
  const selfNode = spaceGraph.getOrError(self.value);
  if (stage.value == UserWizardViewStage.LOG_IN) {
    canvas.tx().update(selfNode, {
      title: "Sign Up",
      valuePacked: packProtoJson(packStateUpdate({ stage: UserWizardViewStage.SIGN_UP })),
    });
  } else if (stage.value == UserWizardViewStage.SIGN_UP) {
    canvas.tx().update(selfNode, {
      title: "Log In",
      valuePacked: packProtoJson(packStateUpdate({ stage: UserWizardViewStage.LOG_IN })),
    });
  } else {
    throw new Error(`unexpected registration stage: ${stage.value}`);
  }
}

async function submit() {
  isActive.value = true;
  lastError.value = null;
  try {
    if (stage.value == UserWizardViewStage.SIGN_UP) {
      await signUp({ name: name.value, slug: slug.value, email: email.value }, password.value);
    } else if (stage.value == UserWizardViewStage.LOG_IN) {
      const { user } = await logIn({ slug: slug.value }, password.value);
      // if we're outside a Bench and have a Bench, go home
      if (user.mainBenchPtr != null && benchPtr.value == null) {
        await goToBench({ bench: user.mainBenchPtr as TypedNodeReferenceData<NodeType.BENCH> });
      }
    } else {
      throw new Error(`unexpected registration stage: ${stage.value}`);
    }
    clear();
  } catch (e) {
    lastError.value = (e as Error).message ?? "Unknown error";
  } finally {
    isActive.value = false;
  }
}

const instance = canvas.registerView(self);
function focus(anchor?: FocusAnchor | NodeReferenceData) {
  const childViews = getViewComponentChildren(instance);
  if (anchor != "bottom") {
    return childViews.find((v) => isVueInstanceOf(v, NativeInput));
  } else {
    return childViews.reverse().find((v) => isVueInstanceOf(v, Button));
  }
}

defineExpose<ViewExposed>({ self, focus });
</script>
<template>
  <div class="mx-auto mt-24 h-fit min-w-80 max-w-96 rounded border border-gray-200 bg-white px-9 py-7 text-gray-900">
    <!-- Header -->
    <div>
      <h2 class="text-2xl font-semibold">{{ title }}</h2>
      <p class="mt-2 text-gray-500">
        <span v-if="user">You're logged in and good to go.</span>
        <span v-else-if="stage == UserWizardViewStage.LOG_IN">Log into an existing Bench account.</span>
        <span v-else-if="stage == UserWizardViewStage.SIGN_UP">Create a new Bench account.</span>
      </p>
    </div>
    <!-- Data -->
    <div v-if="!user" class="mt-5 flex w-full flex-col gap-y-3">
      <NativeInput
        v-if="stage == UserWizardViewStage.SIGN_UP"
        v-model="name"
        :icon="makeIcon({ faName: 'fas fa-user' })"
        name="Name"
        title="Name"
        :variant="Variant.PRIMARY"
        is-input
      />
      <NativeInput
        v-model="slug"
        :icon="makeIcon({ faName: 'fas fa-at' })"
        name="slug"
        title="Username"
        :variant="Variant.PRIMARY"
        is-input
      />
      <NativeInput
        v-if="stage == UserWizardViewStage.SIGN_UP"
        v-model="email"
        :icon="makeIcon({ faName: 'fas fa-at' })"
        name="Email"
        title="Email"
        :variant="Variant.PRIMARY"
        is-input
      />
      <!-- TODO :UX: add passowrd feedback (see https://zxcvbn-ts.github.io/zxcvbn/) -->
      <NativeInput
        v-model="password"
        :icon="makeIcon({ faName: 'fas fa-key' })"
        name="Password"
        title="Password"
        :variant="Variant.PRIMARY"
        is-input
        :value-type="makeTypeInfo({ isSecret: true })"
      />
    </div>
    <!-- Actions -->
    <div class="mt-7">
      <Button
        v-if="!user"
        name="Submit"
        :icon="makeIcon({ faName: 'fas fa-arrow-right-from-bracket' })"
        :title="stage === UserWizardViewStage.LOG_IN ? 'Log in' : 'Sign up'"
        class="w-full"
        :is-disabled="isActive"
        :is-loading="isActive"
        @click="submit"
      />
      <Button
        v-if="!user"
        name="Switch"
        :icon="makeIcon({ faName: 'fas fa-shuffle' })"
        :title="stage === UserWizardViewStage.LOG_IN ? 'Sign up' : 'Log in'"
        class="mt-2 w-full"
        :variant="Variant.COMPACT"
        @click="() => switchStage()"
      />
    </div>
    <!-- Error -->
    <div v-if="lastError" class="mt-3 flex w-full flex-col gap-y-1 border-t border-t-gray-200 pt-3">
      <div class="flex flex-row items-center gap-x-2">
        <i class="fas fa-circle-exclamation text-danger-600" />
        <span class="text-danger-600">Error</span>
      </div>
      <div class="flex flex-row items-center gap-x-2">
        <span class="text-gray-500">{{ lastError }}</span>
      </div>
    </div>
  </div>
</template>
