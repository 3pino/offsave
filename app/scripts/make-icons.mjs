// アプリのアイコンを、リポジトリ直下の icon.svg（Lucide の 24x24 の線画）から作る。
// 色は、画面と同じ @3pino/design-system の色（ダークの surface と accent）を、色相 HUE で出したもの。
// 使い方: node scripts/make-icons.mjs && npx tauri icon src-tauri/icons/icon-manifest.json
// 必要なもの: inkscape（SVG を PNG にする）
import { execFileSync } from 'node:child_process';
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

/** 画面（src/app.css）の --hue と同じ値にする */
const HUE = 144;

const root = join(dirname(fileURLToPath(import.meta.url)), '..', '..');
const out = join(root, 'app', 'src-tauri', 'icons');
mkdirSync(out, { recursive: true });

/** oklch(L C h) を #rrggbb にする（sRGB の範囲に収める） */
function oklchToHex(L, C, h) {
  const a = C * Math.cos((h * Math.PI) / 180);
  const b = C * Math.sin((h * Math.PI) / 180);
  const l = (L + 0.3963377774 * a + 0.2158037573 * b) ** 3;
  const m = (L - 0.1055613458 * a - 0.0638541728 * b) ** 3;
  const s = (L - 0.0894841775 * a - 1.291485548 * b) ** 3;
  const lin = [
    4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
    -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
    -0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s,
  ];
  const gamma = (x) => {
    const c = Math.max(0, Math.min(1, x));
    return c <= 0.0031308 ? 12.92 * c : 1.055 * c ** (1 / 2.4) - 0.055;
  };
  return '#' + lin.map((v) => Math.round(gamma(v) * 255).toString(16).padStart(2, '0')).join('');
}

// design-system の tokens.css のダーク: --surface = oklch(0.21 0.015 hue)、--accent = oklch(0.74 0.13 hue)
const BG = oklchToHex(0.21, 0.015, HUE);
const FG = oklchToHex(0.74, 0.13, HUE);

// icon.svg の線画の中身（<svg> の中の要素）
const source = readFileSync(join(root, 'icon.svg'), 'utf8');
const inner = source.replace(/^[\s\S]*?<svg[^>]*>/, '').replace(/<\/svg>\s*$/, '').trim();

const svg = (body) => `<svg xmlns="http://www.w3.org/2000/svg" width="1024" height="1024" viewBox="0 0 1024 1024">\n${body}\n</svg>\n`;
const art = (offset, scale, color) =>
  `  <g transform="translate(${offset} ${offset}) scale(${scale})" fill="none" stroke="${color}" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">\n    ${inner.replace(/\n\s*/g, '\n    ')}\n  </g>`;

const files = {
  // 角丸の正方形（デスクトップ・iOS 風の通常のアイコン）
  'app-icon': svg(`  <rect width="1024" height="1024" rx="224" fill="${BG}"/>\n${art(192, 640 / 24, FG)}`),
  // Android の適応型アイコン（前景は中央の 66% に収める）
  'app-icon-fg': svg(art(248, 528 / 24, FG)),
  'app-icon-bg': svg(`  <rect width="1024" height="1024" fill="${BG}"/>`),
  'app-icon-monochrome': svg(art(248, 528 / 24, '#ffffff')),
};

for (const [name, body] of Object.entries(files)) {
  writeFileSync(join(out, `${name}.svg`), body);
  execFileSync('inkscape', [join(out, `${name}.svg`), '-o', join(out, `${name}.png`), '-w', '1024', '-h', '1024'], { stdio: 'ignore' });
}

writeFileSync(
  join(out, 'icon-manifest.json'),
  JSON.stringify(
    {
      default: 'app-icon.png',
      bg_color: BG,
      android_bg: 'app-icon-bg.png',
      android_fg: 'app-icon-fg.png',
      android_fg_scale: 100,
      android_monochrome: 'app-icon-monochrome.png',
    },
    null,
    2,
  ) + '\n',
);
// ブラウザのタブ・デスクトップのウィンドウで使う小さなアイコン（背景なしの線画。明るい・暗いどちらの地でも見える中間の明るさ）
const FAVICON = oklchToHex(0.6, 0.13, HUE);
mkdirSync(join(root, 'app', 'public'), { recursive: true });
writeFileSync(
  join(root, 'app', 'public', 'icon.svg'),
  `<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="${FAVICON}" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">\n  ${inner.replace(/\n\s*/g, '\n  ')}\n</svg>\n`,
);
console.log(`hue ${HUE}: 背景 ${BG}、線 ${FG}、ファビコン ${FAVICON}`);
