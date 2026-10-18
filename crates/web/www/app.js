// Mnemonic — web page logic. All encoding, decoding and grading happens in the
// Rust core (compiled to WebAssembly); this file only handles the UI.

import init, * as core from "./pkg/mnemonic_web.js?v=__BUILD__";

const $ = (sel, root = document) => root.querySelector(sel);
const $$ = (sel, root = document) => [...root.querySelectorAll(sel)];
const el = (tag, cls, text) => {
  const n = document.createElement(tag);
  if (cls) n.className = cls;
  if (text != null) n.textContent = text;
  return n;
};

let ready = false;
const api = {
  encode: (d) => JSON.parse(core.encode(d)),
  decode: (s) => JSON.parse(core.decode(s)),
  pegs: () => JSON.parse(core.pegs()),
  challenge: (n) => JSON.parse(core.challenge(n)),
  grade: (n, toWords, a) => core.grade(n, toWords, a),
};

// ---------- Routing (#tab or #tab=value) ----------

const TABS = ["encode", "decode", "practice", "pegs"];
let tab = "encode";

function parseHash() {
  const raw = location.hash.slice(1);
  const i = raw.indexOf("=");
  const name = i < 0 ? raw : raw.slice(0, i);
  const value = i < 0 ? "" : decodeURIComponent(raw.slice(i + 1));
  return { name: TABS.includes(name) ? name : "encode", value };
}

function setHash(name, value = "") {
  const h = value ? `#${name}=${encodeURIComponent(value)}` : `#${name}`;
  if (location.hash !== h) history.replaceState(null, "", h);
}

const finePointer = matchMedia("(pointer: fine)").matches;

function showTab(name, { focus = true } = {}) {
  tab = name;
  for (const a of $$(".tabs a")) a.setAttribute("aria-selected", String(a.dataset.tab === name));
  for (const p of $$("[data-panel]")) p.hidden = p.dataset.panel !== name;
  if (name === "pegs") renderPegs();
  if (focus && finePointer) {
    const target = { encode: "#enc-input", decode: "#dec-input", pegs: "#peg-filter" }[name];
    if (target) $(target).focus();
    else if (name === "practice" && !$("#prac-play").hidden) $("#prac-input").focus();
  }
}

function route() {
  const { name, value } = parseHash();
  if (name === "encode" && value) $("#enc-input").value = value.replace(/\D/g, "");
  if (name === "decode" && value) $("#dec-input").value = value;
  showTab(name);
  if (name === "decode") fitDecInput();
  refresh();
}

function refresh() {
  if (!ready) return;
  if (tab === "encode") renderEncode();
  if (tab === "decode") renderDecode();
}

// ---------- Small helpers ----------

let toastTimer;
function toast(msg) {
  const t = $("#toast");
  t.textContent = msg;
  t.classList.add("show");
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => t.classList.remove("show"), 1600);
}

async function copy(text, msg) {
  try {
    await navigator.clipboard.writeText(text);
  } catch {
    const ta = el("textarea");
    ta.value = text;
    document.body.append(ta);
    ta.select();
    document.execCommand("copy");
    ta.remove();
  }
  toast(msg);
}

function shake(node) {
  node.classList.remove("shake");
  void node.offsetWidth;
  node.classList.add("shake");
}

/** A word with its digits underneath, coloured by part of speech. */
function wordNode(text, code, pos) {
  const w = el("span", "word");
  if (pos) w.dataset.pos = pos;
  // Keep trailing punctuation visually separate from the word itself.
  const [, body, punct] = text.match(/^(.*?)([,.]?)$/);
  const t = el("span", "t", body);
  if (punct) t.append(el("span", "p", punct));
  w.append(t, el("span", "c", code ?? "?"));
  w.title = pos ? `${pos} · ${code}` : code ? code : "not a peg word";
  return w;
}

/** Update `container` to show `items`, reusing unchanged words so only the
 *  words that actually changed animate in. */
function renderWords(container, items) {
  const old = [...container.children];
  items.forEach((item, i) => {
    const key = `${item.text}|${item.code}|${item.pos}`;
    if (old[i]?.dataset.key === key) return;
    const n = wordNode(item.text, item.code, item.pos);
    n.dataset.key = key;
    if (!item.code) n.classList.add("unknown");
    if (old[i]) old[i].replaceWith(n);
    else container.append(n);
  });
  for (const n of old.slice(items.length)) n.remove();
}

// ---------- Encode ----------

const encInput = $("#enc-input");

function renderEncode() {
  const digits = encInput.value;
  const out = $("#enc-out");
  const has = digits.length > 0;
  $("#enc-empty").hidden = has;
  $("#enc-toolbar").hidden = !has;
  setHash("encode", digits);
  if (!has) {
    out.replaceChildren();
    return;
  }
  const r = api.encode(digits);
  if (!r.ok) {
    out.replaceChildren(el("p", "error", r.error));
    return;
  }
  // The core's pretty sentence has exactly one token per word (capitalized,
  // with clause commas and the final period attached).
  const tokens = r.pretty.split(" ");
  if (out.firstElementChild?.classList.contains("error")) out.replaceChildren();
  renderWords(out, r.words.map((w, i) => ({ text: tokens[i], code: w.code, pos: w.pos })));
  const n = digits.length;
  $("#enc-meta").textContent = `${n} digit${n === 1 ? "" : "s"} · ${r.words.length} word${r.words.length === 1 ? "" : "s"}`;
  out.dataset.pretty = r.pretty;
}

encInput.addEventListener("input", () => {
  const clean = encInput.value.replace(/\D/g, "");
  if (clean !== encInput.value) {
    const pos = encInput.selectionStart - (encInput.value.length - clean.length);
    encInput.value = clean;
    encInput.setSelectionRange(pos, pos);
  }
  renderEncode();
});

$("#enc-empty").addEventListener("click", (e) => {
  const ex = e.target.closest("[data-example]");
  if (!ex) return;
  encInput.value = ex.dataset.example;
  renderEncode();
  encInput.focus();
});

$("#enc-copy").addEventListener("click", () => copy($("#enc-out").dataset.pretty, "Sentence copied"));
$("#enc-copy-link").addEventListener("click", () => copy(location.href, "Link copied"));

// ---------- Decode ----------

const decInput = $("#dec-input");

function renderDecode() {
  const text = decInput.value;
  const has = text.trim().length > 0;
  setHash("decode", text.trim());
  $("#dec-empty").hidden = has;
  const err = $("#dec-error");
  if (!has) {
    $("#dec-out").textContent = "";
    $("#dec-words").replaceChildren();
    $("#dec-toolbar").hidden = true;
    err.hidden = true;
    return;
  }
  const r = api.decode(text);
  renderWords($("#dec-words"), r.words.map((w) => ({ text: w.text.toLowerCase(), code: w.code })));
  if (r.ok) {
    $("#dec-out").textContent = r.digits;
    $("#dec-meta").textContent = `${r.digits.length} digit${r.digits.length === 1 ? "" : "s"}`;
    err.hidden = true;
  } else {
    $("#dec-out").textContent = "";
    const unknown = r.words.filter((w) => !w.code).map((w) => `“${w.text}”`);
    err.textContent = unknown.length
      ? `${unknown.join(", ")} ${unknown.length === 1 ? "isn't a peg word" : "aren't peg words"} — check the Pegs tab.`
      : r.error;
    err.hidden = false;
  }
  $("#dec-toolbar").hidden = !r.ok;
}

// Grow the sentence box with its content.
function fitDecInput() {
  decInput.style.height = "auto";
  decInput.style.height = `${decInput.scrollHeight}px`;
}
decInput.addEventListener("input", () => {
  fitDecInput();
  renderDecode();
});
decInput.addEventListener("keydown", (e) => {
  if (e.key === "Enter") e.preventDefault(); // one line; Enter does nothing
});
$("#dec-copy").addEventListener("click", () => copy($("#dec-out").textContent, "Number copied"));

// ---------- Practice ----------

const prac = { toWords: false, digits: 4, rounds: 10, session: null, timer: 0 };

function describe() {
  $("#prac-desc").textContent = prac.toWords
    ? "You'll see a number; write any peg words that encode it."
    : "You'll see a sentence; type the number it stands for.";
}

$$(".segmented [data-dir]").forEach((b) =>
  b.addEventListener("click", () => {
    prac.toWords = b.dataset.dir === "to-words";
    $$(".segmented [data-dir]").forEach((x) => x.setAttribute("aria-checked", String(x === b)));
    describe();
  }),
);

$$(".stepper").forEach((s) => {
  const [minus, plus] = s.querySelectorAll("button");
  const out = s.querySelector("output");
  const step = (d) => {
    const key = s.dataset.key;
    prac[key] = Math.min(+s.dataset.max, Math.max(+s.dataset.min, prac[key] + d));
    out.textContent = prac[key];
  };
  minus.addEventListener("click", () => step(-1));
  plus.addEventListener("click", () => step(1));
});

function randomDigits(n) {
  let s = "";
  const buf = new Uint8Array(1);
  while (s.length < n) {
    crypto.getRandomValues(buf);
    if (buf[0] < 250) s += buf[0] % 10; // reject to avoid modulo bias
  }
  return s;
}

const fmtClock = (ms) => {
  const s = Math.floor(ms / 1000);
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
};

function practiceView(which) {
  for (const id of ["setup", "play", "done"]) $(`#prac-${id}`).hidden = id !== which;
}

function startPractice() {
  if (!ready) return;
  prac.session = { index: 0, results: [], started: performance.now() };
  $("#prac-feedback").textContent = "";
  $("#prac-feedback").className = "feedback";
  practiceView("play");
  clearInterval(prac.timer);
  prac.timer = setInterval(() => {
    $("#prac-clock").textContent = fmtClock(performance.now() - prac.session.started);
  }, 250);
  $("#prac-clock").textContent = "0:00";
  nextRound();
}

function nextRound() {
  const s = prac.session;
  s.current = api.challenge(randomDigits(prac.digits));
  $("#prac-round").textContent = `Round ${s.index + 1} of ${prac.rounds}`;
  $("#prac-bar").style.width = `${(s.index / prac.rounds) * 100}%`;
  const prompt = $("#prac-prompt");
  prompt.textContent = prac.toWords ? s.current.number : s.current.sentence;
  prompt.classList.toggle("digits", prac.toWords);
  // Restart the entrance animation for each new prompt.
  prompt.style.animation = "none";
  void prompt.offsetWidth;
  prompt.style.animation = "";
  const input = $("#prac-input");
  input.value = "";
  input.placeholder = prac.toWords ? "Type the words" : "Type the number";
  input.inputMode = prac.toWords ? "text" : "numeric";
  input.focus();
}

$("#prac-form").addEventListener("submit", (e) => {
  e.preventDefault();
  const s = prac.session;
  const input = $("#prac-input");
  const answer = input.value.trim();
  if (!answer) return shake(input);
  const { number, sentence } = s.current;
  const correct = api.grade(number, prac.toWords, answer);
  const prompt = prac.toWords ? number : sentence;
  const expected = prac.toWords ? sentence : number;
  s.results.push({ prompt, answer, expected, correct });

  const fb = $("#prac-feedback");
  fb.className = `feedback ${correct ? "ok" : "bad"}`;
  fb.replaceChildren();
  if (correct) fb.textContent = "Correct.";
  else fb.append("Not quite — ", el("b", null, prompt.replace(/\.$/, "")), " → ", el("b", null, expected));

  s.index += 1;
  if (s.index >= prac.rounds) finishPractice();
  else nextRound();
});

function finishPractice() {
  clearInterval(prac.timer);
  const s = prac.session;
  const elapsed = performance.now() - s.started;
  const n = s.results.length;
  const right = s.results.filter((r) => r.correct).length;
  $("#prac-score").textContent = right;
  $("#prac-of").textContent = `/ ${n}`;
  $("#prac-stats").textContent = n
    ? `${fmtClock(elapsed)} total · ${(elapsed / n / 1000).toFixed(1)}s per round · ${Math.round((right / n) * 100)}% correct`
    : "No rounds answered.";
  $("#prac-review").replaceChildren(
    ...s.results.map((r) => {
      const li = el("li", r.correct ? "ok" : "bad");
      li.append(
        el("span", "mark", r.correct ? "✓" : "✗"),
        el("span", "q", r.prompt),
        el("span", "a", r.correct ? `You: ${r.answer}` : `You: ${r.answer} · Answer: ${r.expected}`),
      );
      return li;
    }),
  );
  practiceView("done");
  $("#prac-again").focus();
}

$("#prac-start").addEventListener("click", startPractice);
$("#prac-again").addEventListener("click", startPractice);
$("#prac-quit").addEventListener("click", finishPractice);
$("#prac-settings").addEventListener("click", () => practiceView("setup"));

// ---------- Pegs ----------

let pegData = null;

function escapeHtml(s) {
  return s.replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);
}

function highlight(word, q) {
  const i = q ? word.indexOf(q) : -1;
  if (i < 0) return escapeHtml(word);
  return `${escapeHtml(word.slice(0, i))}<mark>${escapeHtml(word.slice(i, i + q.length))}</mark>${escapeHtml(word.slice(i + q.length))}`;
}

function pegCard(code, words, q) {
  const items = words.map(([pos, w]) => `<li data-pos="${pos}" title="${pos}"><span>${highlight(w, q)}</span></li>`).join("");
  return `<article class="peg"><div class="code">${code}</div><ul>${items}</ul></article>`;
}

function renderPegs() {
  if (!ready) return;
  pegData ??= api.pegs();
  const q = $("#peg-filter").value.trim().toLowerCase();
  const byDigits = /^\d+$/.test(q);
  const match = (code, words) =>
    !q || (byDigits ? code.startsWith(q) : words.some(([, w]) => w.includes(q)));

  const pairs = pegData.pairs
    .map((p) => [p.code, [["verb", p.verb], ["adjective", p.adjective], ["noun", p.noun]]])
    .filter(([c, w]) => match(c, w));
  const singles = pegData.singles
    .map((s) => [s.code, [["verb", s.word]]])
    .filter(([c, w]) => match(c, w));

  const hq = byDigits ? "" : q;
  let html = pairs.map(([c, w]) => pegCard(c, w, hq)).join("");
  if (singles.length) {
    html += `<p class="field-label section-title">Single digits — end an odd-length number</p>`;
    html += singles.map(([c, w]) => pegCard(c, w, hq)).join("");
  }
  $("#peg-grid").innerHTML = html || `<p class="hint none">No pegs match “${escapeHtml(q)}”.</p>`;
}

$("#peg-filter").addEventListener("input", renderPegs);

// ---------- Theme ----------

$(".theme-toggle").addEventListener("click", () => {
  const root = document.documentElement;
  const dark = root.dataset.theme
    ? root.dataset.theme === "dark"
    : matchMedia("(prefers-color-scheme: dark)").matches;
  root.dataset.theme = dark ? "light" : "dark";
  try {
    localStorage.setItem("theme", root.dataset.theme);
  } catch {}
});

// ---------- Boot ----------

for (const a of $$(".tabs a, .wordmark")) {
  a.addEventListener("click", (e) => {
    e.preventDefault();
    const name = a.dataset.tab ?? "encode";
    const value = name === "encode" ? encInput.value : name === "decode" ? decInput.value.trim() : "";
    setHash(name, value);
    showTab(name);
    refresh();
  });
}
window.addEventListener("hashchange", route);

describe();
route();

await init({ module_or_path: new URL("./pkg/mnemonic_web_bg.wasm?v=__BUILD__", import.meta.url) });
ready = true;
refresh();
if (tab === "pegs") renderPegs();
