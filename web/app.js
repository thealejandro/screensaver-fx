'use strict';

/* Demo en vivo de Screensaver FX: el motor de ssfx-core compilado a wasm32-unknown-unknown
 * (crate ssfx-wasm) pintado en un canvas con requestAnimationFrame. Sin frameworks. */

const $ = (s) => document.querySelector(s);
const canvas = $('#screen');
const ctx = canvas.getContext('2d');

let wasm = null;   // exports del módulo
let memory = null; // WebAssembly.Memory
let engine = 0;    // puntero del motor o 0
let scratch = 0;   // zona de 64 B para WallTime/tamaño/destino
let seed = BigInt(Math.floor(Math.random() * Number.MAX_SAFE_INTEGER));
let es = (navigator.language || 'en').toLowerCase().startsWith('es');
let fxIds = [];    // ids de efecto, en el orden del motor
let themeIds = []; // ids de tema

const decoder = new TextDecoder();
const encoder = new TextEncoder();

function cstr(ptr) {
  const u8 = new Uint8Array(memory.buffer);
  let end = ptr;
  while (u8[end] !== 0) end += 1;
  return decoder.decode(u8.subarray(ptr, end));
}

/* La firma del motor: ssfxw_new(w, h, ini, ini_len, seed: u64, lang_es). */
function buildEngine(ini) {
  if (!wasm) return;
  const bytes = encoder.encode(ini);
  const ptr = wasm.ssfxw_alloc(Math.max(1, bytes.length));
  new Uint8Array(memory.buffer, ptr, bytes.length).set(bytes);
  const e = wasm.ssfxw_new(canvas.width, canvas.height, bytes.length ? ptr : 0, bytes.length, seed, es ? 1 : 0);
  wasm.ssfxw_dealloc(ptr, Math.max(1, bytes.length));
  if (engine) wasm.ssfxw_free(engine);
  engine = e;
}

function currentIni() {
  const fx = $('#fx').value;
  const theme = $('#theme').value;
  return `effects=${fx || 'all'}\ntheme=${theme}\n`;
}

function sizeCanvas() {
  const r = canvas.getBoundingClientRect();
  // Tope de ~2 Mpx: por encima el motor ya pinta celdas grandes y un lienzo 4K solo encarece cada
  // fotograma; el navegador lo amplía (pixelado, ver style.css) sin coste por fotograma.
  const MAX_PX = 1920 * 1080;
  let dpr = Math.min(window.devicePixelRatio || 1, 2);
  dpr = Math.min(dpr, Math.sqrt(MAX_PX / Math.max(1, r.width * r.height)));
  canvas.width = Math.max(1, Math.round(r.width * dpr));
  canvas.height = Math.max(1, Math.round(r.height * dpr));
}

/* Un canvas auxiliar del tamaño exacto del búfer: putImageData exige un ImageData de
 * buffer_w x buffer_h y drawImage lo escala al rectángulo destino que manda el motor. */
const off = document.createElement('canvas');
const offCtx = off.getContext('2d');
let img = null;
let rgbaPtr = 0; // búfer RGBA en la memoria del módulo
let rgbaLen = 0;

// Fuera de la vista (pestaña oculta o la demo fuera de pantalla en la página que la incrusta) no se
// dibuja: no gasta CPU ni batería.
let onScreen = true;
new IntersectionObserver((e) => { onScreen = e[e.length - 1].isIntersecting; }).observe(canvas);

function frame(nowMs) {
  requestAnimationFrame(frame);
  if (!engine || !onScreen || document.hidden) return;
  const t = nowMs / 1000; // monótono, origen irrelevante para el motor

  const ints = new Int32Array(memory.buffer, scratch, 13);
  const d = new Date();
  ints.set([d.getFullYear(), d.getMonth() + 1, d.getDate(), (d.getDay() + 6) % 7,
            d.getHours(), d.getMinutes(), d.getSeconds()], 0);

  if (!wasm.ssfxw_tick(engine, t, scratch)) return;

  const px = wasm.ssfxw_pixels(engine, scratch + 28, scratch + 32);
  const w = ints[7];
  const h = ints[8];
  if (!px || w <= 0 || h <= 0) return;
  const scaledDown = wasm.ssfxw_dest(engine, scratch + 36);
  const [dx, dy, dw, dh] = ints.slice(9, 13);

  // El motor convierte a RGBA en la memoria del módulo y el ImageData es una vista de esa zona:
  // sin bucle por píxel en JS. La vista se rehace si cambia el tamaño o crece la memoria.
  if (off.width !== w || off.height !== h) {
    off.width = w;
    off.height = h;
  }
  const len = w * h * 4;
  if (len !== rgbaLen) {
    if (rgbaPtr) wasm.ssfxw_dealloc(rgbaPtr, rgbaLen);
    rgbaPtr = wasm.ssfxw_alloc(len);
    rgbaLen = len;
    img = null;
  }
  wasm.ssfxw_rgba(engine, rgbaPtr);
  if (!img || img.data.buffer !== memory.buffer) {
    img = new ImageData(new Uint8ClampedArray(memory.buffer, rgbaPtr, len), w, h);
  }
  offCtx.putImageData(img, 0, 0);

  ctx.imageSmoothingEnabled = scaledDown === 1;
  ctx.fillStyle = '#000';
  ctx.fillRect(0, 0, canvas.width, canvas.height);
  ctx.drawImage(off, dx, dy, dw, dh);
}

function fillSelects() {
  const fxSel = $('#fx');
  const themeSel = $('#theme');
  const prevFx = fxSel.value;
  const prevTheme = themeSel.value;
  fxSel.textContent = '';
  themeSel.textContent = '';

  fxSel.add(new Option(es ? 'Todos (barajados)' : 'All (shuffled)', ''));
  fxIds = [];
  for (let i = 0; i < wasm.ssfx_fx_count(); i += 1) {
    const id = cstr(wasm.ssfx_fx_id(i));
    fxIds.push(id);
    fxSel.add(new Option(cstr(wasm.ssfx_fx_name(i, es ? 1 : 0)), id));
  }

  themeSel.add(new Option(es ? 'Aleatorio' : 'Random', 'random'));
  themeSel.add(new Option('Auto', 'auto'));
  themeIds = [];
  for (let i = 0; i < wasm.ssfx_theme_count(); i += 1) {
    const id = cstr(wasm.ssfx_theme_id(i));
    themeIds.push(id);
    themeSel.add(new Option(cstr(wasm.ssfx_theme_name(i)), id));
  }

  if ([...fxSel.options].some((o) => o.value === prevFx)) fxSel.value = prevFx;
  if ([...themeSel.options].some((o) => o.value === prevTheme)) themeSel.value = prevTheme;
}

function fillGallery() {
  const grid = $('#grid');
  grid.textContent = '';
  const base = 'https://raw.githubusercontent.com/thealejandro/screensaver-fx/media/fx/';
  fxIds.forEach((id, i) => {
    const fig = document.createElement('figure');
    fig.className = 'cell';
    const imgEl = document.createElement('img');
    imgEl.src = `${base}${id}.gif`;
    imgEl.alt = id;
    imgEl.loading = 'lazy';
    imgEl.decoding = 'async';
    const cap = document.createElement('figcaption');
    cap.textContent = cstr(wasm.ssfx_fx_name(i, es ? 1 : 0));
    fig.append(imgEl, cap);
    fig.addEventListener('click', () => {
      $('#fx').value = id;
      buildEngine(currentIni());
      $('#demo').scrollIntoView({ behavior: 'smooth' });
    });
    grid.append(fig);
  });
}

/* Toggle de idioma: el HTML trae el inglés como texto y el español en data-es. */
function applyLang() {
  document.documentElement.lang = es ? 'es' : 'en';
  $('#lang').textContent = es ? 'EN' : 'ES';
  document.querySelectorAll('[data-es]').forEach((el) => {
    if (el.dataset.en === undefined) el.dataset.en = el.textContent;
    el.textContent = es ? el.dataset.es : el.dataset.en;
  });
  if (wasm) {
    fillSelects();
    fillGallery();
    buildEngine(currentIni());
  }
}

async function loadWasm() {
  const res = await fetch('ssfx_wasm.wasm');
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  const { instance } = await WebAssembly.instantiate(await res.arrayBuffer(), {});
  wasm = instance.exports;
  memory = wasm.memory;
  scratch = wasm.ssfxw_alloc(64);
}

async function boot() {
  $('#lang').addEventListener('click', () => {
    es = !es;
    applyLang();
  });

  sizeCanvas();
  try {
    await loadWasm();
  } catch (err) {
    $('#wasm-error').hidden = false;
    return;
  }

  applyLang(); // aplica el idioma del navegador, llena los selects y la galería
  requestAnimationFrame(frame);

  $('#fx').addEventListener('change', () => buildEngine(currentIni()));
  $('#theme').addEventListener('change', () => buildEngine(currentIni()));

  $('#fs').addEventListener('click', () => {
    $('#demo').requestFullscreen?.();
  });
  document.addEventListener('keydown', () => {
    if (document.fullscreenElement) document.exitFullscreen();
  });

  let resizeTimer = 0;
  new ResizeObserver(() => {
    clearTimeout(resizeTimer);
    resizeTimer = setTimeout(() => {
      sizeCanvas();
      buildEngine(currentIni());
    }, 150);
  }).observe(canvas);
}

boot();
