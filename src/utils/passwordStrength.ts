/**
 * 密码强度评估（移植自 Python 版 zhmm/core/password_strength.py）
 *
 * 纯函数、零依赖，适合 GUI 实时提示。
 */

export enum StrengthLevel {
  VERY_WEAK = 0,
  WEAK = 1,
  FAIR = 2,
  STRONG = 3,
  VERY_STRONG = 4,
}

const LEVEL_LABEL: Record<StrengthLevel, string> = {
  [StrengthLevel.VERY_WEAK]: "极弱",
  [StrengthLevel.WEAK]: "弱",
  [StrengthLevel.FAIR]: "一般",
  [StrengthLevel.STRONG]: "强",
  [StrengthLevel.VERY_STRONG]: "极强",
};

export interface StrengthResult {
  /** 0-100 分数 */
  score: number;
  /** 等级枚举 */
  level: StrengthLevel;
  /** 中文等级标签 */
  label: string;
  /** 一条改进建议（可为空） */
  hint: string;
}

// 常见弱密码子串（全部小写）
const COMMON_WEAK: string[] = [
  "password",
  "passwd",
  "qwerty",
  "admin",
  "root",
  "letmein",
  "welcome",
  "monkey",
  "dragon",
  "master",
  "login",
  "iloveyou",
  "abc123",
  "123456",
  "111111",
  "000000",
  "888888",
  "123123",
];

// 键盘行（小写），用于检测连续 3+ 字符的键盘序
const KEYBOARD_ROWS: string[] = [
  "qwertyuiop",
  "asdfghjkl",
  "zxcvbnm",
  "1234567890",
];

const REPEAT_RE = /(.)\1\1/;

function charsetSize(password: string): number {
  let size = 0;
  if (/[a-z]/.test(password)) size += 26;
  if (/[A-Z]/.test(password)) size += 26;
  if (/[0-9]/.test(password)) size += 10;
  if (/[!@#$%^&*()\-_=+\[\]{};:'",.<>/?\\|`~]/.test(password)) size += 32;
  // 非 ASCII（中文、emoji 等）
  if (/[^\x00-\x7F]/.test(password)) size += 50;
  return size;
}

function hasSequence(password: string): boolean {
  if (password.length < 3) return false;

  // 码点连续
  for (let i = 0; i < password.length - 2; i++) {
    const a = password.charCodeAt(i);
    const b = password.charCodeAt(i + 1);
    const c = password.charCodeAt(i + 2);
    if (b - a === 1 && c - b === 1) return true;
    if (a - b === 1 && b - c === 1) return true;
  }

  // 键盘序
  const lower = password.toLowerCase();
  for (const row of KEYBOARD_ROWS) {
    for (let i = 0; i <= row.length - 3; i++) {
      if (lower.includes(row.slice(i, i + 3))) return true;
    }
    // 反向键盘序
    const rev = row.split("").reverse().join("");
    for (let i = 0; i <= rev.length - 3; i++) {
      if (lower.includes(rev.slice(i, i + 3))) return true;
    }
  }
  return false;
}

function hasRepeat(password: string): boolean {
  return REPEAT_RE.test(password);
}

function scoreToLevel(score: number): StrengthLevel {
  if (score < 20) return StrengthLevel.VERY_WEAK;
  if (score < 40) return StrengthLevel.WEAK;
  if (score < 60) return StrengthLevel.FAIR;
  if (score < 80) return StrengthLevel.STRONG;
  return StrengthLevel.VERY_STRONG;
}

function buildHint(
  _password: string,
  length: number,
  charset: number,
  hasSeq: boolean,
  hasRep: boolean,
  hitCommon: boolean
): string {
  if (length < 8) return "建议长度 ≥ 8";
  if (hitCommon) return "含常见弱密码子串，建议替换";
  if (charset < 26) return "建议混用大小写 / 数字 / 符号";
  if (hasSeq) return "含连续字符或键盘序，建议打乱顺序";
  if (hasRep) return "含重复字符，建议减少重复";
  if (charset < 52 && length < 12) return "增加字符类型或长度可显著提升强度";
  return "";
}

/**
 * 评估密码强度。
 *
 * 空串返回 score=0、level=VERY_WEAK。
 */
export function assessStrength(password: string): StrengthResult {
  if (!password) {
    return {
      score: 0,
      level: StrengthLevel.VERY_WEAK,
      label: LEVEL_LABEL[StrengthLevel.VERY_WEAK],
      hint: "",
    };
  }

  const length = password.length;
  const charset = charsetSize(password);

  // 基础熵值（bits） = length * log2(charset)
  const entropy = charset > 0 ? length * Math.log2(charset) : 0;
  // 映射到 0-100：经验系数 1.2，使熵 ~64 bit 落在 75 附近
  let scoreF = entropy * 1.2;

  // --- 惩罚项 ---
  const uniqueRatio = new Set(password).size / length;
  if (uniqueRatio < 0.5) {
    scoreF *= 0.7;
  }

  const hasSeq = hasSequence(password);
  if (hasSeq) scoreF -= 15;

  const hasRep = hasRepeat(password);
  if (hasRep) scoreF -= 10;

  const lower = password.toLowerCase();
  const hitCommon = COMMON_WEAK.some((w) => lower.includes(w));
  if (hitCommon) {
    scoreF = Math.min(scoreF, 25);
  }

  // --- 长度硬下限打底 ---
  if (length < 6) {
    scoreF = Math.min(scoreF, 15);
  } else if (length < 8) {
    scoreF = Math.min(scoreF, 30);
  }

  const score = Math.max(0, Math.min(100, Math.round(scoreF)));
  const level = scoreToLevel(score);
  const hint = buildHint(password, length, charset, hasSeq, hasRep, hitCommon);

  return { score, level, label: LEVEL_LABEL[level], hint };
}
