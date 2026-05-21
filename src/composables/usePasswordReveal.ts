import { ref } from "vue";
import { api } from "../api";
import type { PasswordSummary } from "../api";

export function usePasswordReveal(message: { info: (m: string) => void; error: (m: string) => void }) {
  const revealedPasswords = ref<Map<string, string>>(new Map());
  const revealTimers: Map<string, ReturnType<typeof setTimeout>> = new Map();
  const REVEAL_DURATION = 10; // 秒

  function revealPassword(row: PasswordSummary) {
    // 如果已显示，则隐藏
    if (revealedPasswords.value.has(row.id)) {
      hidePassword(row.id);
      return;
    }
    api.getPassword(row.id).then((entry) => {
      revealedPasswords.value.set(row.id, entry.password);
      // 定时自动隐藏
      const timer = setTimeout(() => hidePassword(row.id), REVEAL_DURATION * 1000);
      revealTimers.set(row.id, timer);
      message.info(`密码已显示，${REVEAL_DURATION}秒后自动隐藏`);
    }).catch((e: any) => {
      message.error(`获取密码失败: ${e}`);
    });
  }

  function hidePassword(id: string) {
    revealedPasswords.value.delete(id);
    const timer = revealTimers.get(id);
    if (timer) {
      clearTimeout(timer);
      revealTimers.delete(id);
    }
  }

  return { revealedPasswords, revealPassword, hidePassword };
}
