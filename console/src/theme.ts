/** Giao diện tùy biến: màu nhấn, nền sáng/tối, hình nền, độ kính, bo góc. Trả về biến CSS cho khung .app. */
import type { CSSProperties } from "react";
import type { Settings } from "./api";

export const ACCENTS: { name: string; en: string; hex: string }[] = [
  { name: "Xanh ngọc", en: "Teal", hex: "#0a9d94" },
  { name: "Xanh dương", en: "Blue", hex: "#2563eb" },
  { name: "Chàm", en: "Indigo", hex: "#4f46e5" },
  { name: "Tím", en: "Purple", hex: "#9333ea" },
  { name: "Hồng", en: "Pink", hex: "#db2777" },
  { name: "Đỏ", en: "Red", hex: "#dc2626" },
  { name: "Cam", en: "Orange", hex: "#ea580c" },
  { name: "Xanh lá", en: "Green", hex: "#16a34a" },
  { name: "Than chì", en: "Graphite", hex: "#475569" },
];

type Wall = { light: [string, string, string]; dark: [string, string, string] };
export const WALLPAPERS: Record<string, { name: string; en: string } & Wall> = {
  aurora: { name: "Cực quang", en: "Aurora", light: ["#eef3f7", "#dfeef0", "#ebe7f6"], dark: ["#0b1116", "#0b1f22", "#17142a"] },
  ocean: { name: "Đại dương", en: "Ocean", light: ["#eef4fb", "#dbeafe", "#e0f2fe"], dark: ["#0a1020", "#0c1e3a", "#0b2533"] },
  sunset: { name: "Hoàng hôn", en: "Sunset", light: ["#fbf3ee", "#fde4d6", "#fce7f3"], dark: ["#16100e", "#2a1612", "#2a1222"] },
  forest: { name: "Rừng", en: "Forest", light: ["#f0f5ef", "#dcefdc", "#eef3e2"], dark: ["#0c120d", "#10221a", "#18200f"] },
  lavender: { name: "Oải hương", en: "Lavender", light: ["#f4f2fb", "#e9e3fb", "#f6e8f6"], dark: ["#110f1a", "#1d1631", "#24132a"] },
  plain: { name: "Trơn", en: "Plain", light: ["#f2f3f5", "#f2f3f5", "#f2f3f5"], dark: ["#0f1215", "#0f1215", "#0f1215"] },
};

function rgb(hex: string): [number, number, number] {
  const m = /^#?([0-9a-f]{6})$/i.exec(hex.trim());
  const n = m ? parseInt(m[1], 16) : 0x0a9d94;
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}
const toHex = (c: number[]) => "#" + c.map((x) => Math.max(0, Math.min(255, Math.round(x))).toString(16).padStart(2, "0")).join("");
const mix = (a: number[], b: number[], t: number) => a.map((x, i) => x + (b[i] - x) * t);
const lum = ([r, g, b]: number[]) => {
  const f = (v: number) => { v /= 255; return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4; };
  return 0.2126 * f(r) + 0.7152 * f(g) + 0.0722 * f(b);
};

export function resolveMode(s: Settings | null, systemDark: boolean): "light" | "dark" {
  const m = s?.theme_mode || (s?.dark ? "dark" : "light");
  return m === "system" ? (systemDark ? "dark" : "light") : m === "dark" ? "dark" : "light";
}

export function glassLevel(s: Settings | null): number {
  if (!s || s.glass === false) return 0;
  return s.glass_level ?? 60;
}

/** Biến CSS cho khung .app */
export function themeStyle(s: Settings | null, dark: boolean): CSSProperties {
  const base = rgb(s?.accent || "#0a9d94");
  // nền tối: làm màu nhấn sáng hơn cho dễ đọc
  const acc = dark ? mix(base, [255, 255, 255], 0.22) : base;
  const text = dark ? mix(base, [255, 255, 255], 0.45) : mix(base, [0, 0, 0], 0.18);
  const on = lum(acc) > 0.45 ? "#0b1a1a" : "#ffffff";
  const [r, g, b] = acc.map(Math.round);
  const w = WALLPAPERS[s?.wallpaper || "aurora"] ?? WALLPAPERS.aurora;
  const [b1, b2, b3] = dark ? w.dark : w.light;
  const lv = glassLevel(s);
  const alpha = dark ? 0.82 - lv * 0.0045 : 0.9 - lv * 0.0055;
  const blur = Math.round(6 + lv * 0.26);
  const rk = s?.radius === "compact" ? 0.55 : s?.radius === "round" ? 1.35 : 1;
  const glassRgb = dark ? "30, 36, 44" : "255, 255, 255";
  return {
    ["--accent" as string]: toHex(acc),
    ["--accent-600" as string]: toHex(mix(acc, [0, 0, 0], 0.12)),
    ["--accent-text" as string]: toHex(text),
    ["--accent-soft" as string]: `rgba(${r}, ${g}, ${b}, ${dark ? 0.16 : 0.12})`,
    ["--accent-ring" as string]: `rgba(${r}, ${g}, ${b}, .38)`,
    ["--on-accent" as string]: on,
    ["--bg-1" as string]: b1, ["--bg-2" as string]: b2, ["--bg-3" as string]: b3,
    ["--glass" as string]: `rgba(${glassRgb}, ${alpha.toFixed(3)})`,
    ["--blur" as string]: `saturate(180%) blur(${blur}px)`,
    ["--rk" as string]: String(rk),
  };
}
