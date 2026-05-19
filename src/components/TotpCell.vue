<script setup lang="ts">
import { onMounted, onUnmounted, ref, watch } from "vue";
import { api } from "../api";
import { copyAndScheduleClear } from "../settings";

const props = defineProps<{ id: string }>();

const code = ref("------");
const remaining = ref(0);
const error = ref(false);
let timer: number | null = null;

async function refresh() {
  try {
    const r = await api.generateTotp(props.id);
    code.value = r.code;
    remaining.value = r.remaining_seconds;
    error.value = false;
  } catch {
    error.value = true;
    code.value = "------";
  }
}

function tick() {
  if (remaining.value > 1) {
    remaining.value -= 1;
  } else {
    refresh();
  }
}

onMounted(() => {
  refresh();
  timer = window.setInterval(tick, 1000);
});

onUnmounted(() => {
  if (timer != null) window.clearInterval(timer);
});

watch(() => props.id, refresh);

async function copy() {
  if (error.value) return;
  await copyAndScheduleClear(code.value);
}
</script>

<template>
  <div class="totp-cell" :class="{ 'totp-error': error }" @click="copy" title="点击复制验证码">
    <span class="totp-code">{{ code }}</span>
    <span class="totp-left">{{ remaining }}s</span>
  </div>
</template>

<style scoped>
.totp-cell {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  user-select: none;
}
.totp-code {
  font-weight: 600;
  letter-spacing: 1px;
}
.totp-left {
  font-size: 12px;
  color: var(--n-text-color-3, #999);
}
.totp-error .totp-code {
  color: var(--n-error-color, #d03050);
}
</style>
