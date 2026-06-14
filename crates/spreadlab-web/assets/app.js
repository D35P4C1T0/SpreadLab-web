const statOrder = [
  ["hp", "HP"],
  ["atk", "Atk"],
  ["def", "Def"],
  ["spa", "SpA"],
  ["spd", "SpD"],
  ["spe", "Spe"],
];
const natureEffects = {
  Adamant: ["atk", "spa"], Bold: ["def", "atk"], Brave: ["atk", "spe"],
  Calm: ["spd", "atk"], Careful: ["spd", "spa"], Gentle: ["spd", "def"],
  Hasty: ["spe", "def"], Impish: ["def", "spa"], Jolly: ["spe", "spa"],
  Lax: ["def", "spd"], Lonely: ["atk", "def"], Mild: ["spa", "def"],
  Modest: ["spa", "atk"], Naive: ["spe", "spd"], Naughty: ["atk", "spd"],
  Quiet: ["spa", "spe"], Rash: ["spa", "spd"], Relaxed: ["def", "spe"],
  Sassy: ["spd", "spe"], Timid: ["spe", "atk"],
};
let moveTypes = {
  "Iron Head": "Steel",
  "Knock Off": "Dark",
  "Sucker Punch": "Dark",
  "Swords Dance": "Normal",
  "Dazzling Gleam": "Fairy",
  "Draining Kiss": "Fairy",
  "Calm Mind": "Psychic",
  "Protect": "Normal",
};
let speciesTypes = {};
let speciesAbilities = {};
const storageKey = "spreadlab.webui.state.v1";
let restoredState = null;

function parseSet(text) {
  const lines = text.split(/\r?\n/).map((line) => line.trim()).filter(Boolean);
  const [rawName = "Unknown", rawItem = "None"] = (lines[0] || "Unknown").split("@").map((part) => part.trim());
  const data = { name: rawName, item: rawItem || "None", ability: "None", abilityOn: null, status: "Healthy", nature: "Hardy", moves: [], sps: {} };
  for (const line of lines.slice(1)) {
    if (line.startsWith("Ability:")) data.ability = line.replace("Ability:", "").trim();
    else if (/^Ability On:/i.test(line)) data.abilityOn = parseBool(line.replace(/^Ability On:/i, "").trim());
    else if (/^Status:/i.test(line)) data.status = displayStatus(line.replace(/^Status:/i, "").trim());
    else if (line.endsWith("Nature")) data.nature = line.replace("Nature", "").trim();
    else if (/^(SPs|EVs):/i.test(line)) {
      for (const part of line.replace(/^(SPs|EVs):/i, "").split("/")) {
        const match = part.trim().match(/^(\d+)\s+(HP|Atk|Def|SpA|SpD|Spe)$/i);
        if (match) data.sps[match[2].toLowerCase()] = Number(match[1]);
      }
    } else if (line.startsWith("-")) data.moves.push(line.replace(/^-+\s*/, ""));
  }
  return data;
}

function normalizedSetText(text) {
  return text
    .replace(/^EVs:/gim, "SPs:")
    .replace(/^Floette-Mega\s*(?:@\s*Floettite)?\s*$/gim, "Mega Floette")
    .replace(/^(.+?)\s+@\s+Floettite\s*$/gim, "$1");
}

function setWithAbilityState(text, cardKey) {
  const normalized = normalizedSetText(text);
  const toggle = document.querySelector(`[data-set-card="${cardKey}"] [data-ability-toggle]`);
  if (!toggle) return normalized;
  return replaceOrInsertAfter(
    normalized,
    /^Ability On:/i,
    `Ability On: ${toggle.checked ? "true" : "false"}`,
    /^Ability:/i
  );
}

function setWithStatus(text, cardKey) {
  const status = document.querySelector(`[data-set-card="${cardKey}"] [data-status-select]`)?.value || "Healthy";
  if (normalizeName(status) === "healthy") return text.replace(/^Status:.*\n?/gim, "");
  return replaceOrInsertAfter(text, /^Status:/i, `Status: ${status}`, /^Ability On:|^Ability:/i);
}

function buildSpsLine(card) {
  const parts = [];
  for (const [key, label] of statOrder) {
    const input = card.querySelector(`[data-sp-key="${key}"]`);
    const value = Number(input?.value || 0);
    if (value > 0) parts.push(`${value} ${label}`);
  }
  return parts.length ? `SPs: ${parts.join(" / ")}` : "SPs: 0 HP";
}

function replaceOrInsertLine(text, matcher, replacement) {
  const lines = text.split(/\r?\n/);
  const index = lines.findIndex((line) => matcher.test(line.trim()));
  if (index >= 0) lines[index] = replacement;
  else lines.splice(Math.min(3, lines.length), 0, replacement);
  return lines.join("\n");
}

function replaceOrInsertAfter(text, matcher, replacement, afterMatcher) {
  const lines = text.split(/\r?\n/);
  const index = lines.findIndex((line) => matcher.test(line.trim()));
  if (index >= 0) lines[index] = replacement;
  else {
    const after = lines.findIndex((line) => afterMatcher.test(line.trim()));
    lines.splice(after >= 0 ? after + 1 : Math.min(2, lines.length), 0, replacement);
  }
  return lines.join("\n");
}

function rewriteCardSet(card) {
  const editor = card.querySelector(".raw-editor");
  if (!editor) return;
  const nature = card.querySelector("[data-card-nature]")?.value;
  let text = card.querySelector("[data-sp-key]")
    ? replaceOrInsertLine(editor.value, /^SPs:/i, buildSpsLine(card))
    : editor.value;
  if (nature && nature !== "Any") text = replaceOrInsertLine(text, / Nature$/i, `${nature} Nature`);
  editor.value = text;
  syncRawEditor(editor);
}

function renderCard(card, parsed) {
  const name = card.querySelector('[data-field="name"]');
  if (name) name.innerHTML = `${parsed.name} <em>${card.dataset.setCard === "attacker" ? "♂" : "♀"}</em>`;
  renderTypes(card, parsed.name);
  card.querySelector('[data-field="ability"]')?.replaceChildren(document.createTextNode(parsed.ability));
  const abilityToggle = card.querySelector("[data-ability-toggle]");
  if (abilityToggle && parsed.abilityOn !== null) abilityToggle.checked = parsed.abilityOn;
  card.querySelector('[data-field="item"]')?.replaceChildren(document.createTextNode(parsed.item));
  const itemSprite = card.querySelector("[data-item-sprite]");
  if (itemSprite) {
    itemSprite.dataset.itemSprite = parsed.item;
    itemSprite.alt = parsed.item;
    itemSprite.src = `/api/item-sprite/${encodeURIComponent(parsed.item || "None")}`;
  }

  const nature = card.querySelector("[data-card-nature]");
  if (nature && nature.value !== "Any" && [...nature.options].some((option) => option.value === parsed.nature)) nature.value = parsed.nature;
  const status = card.querySelector("[data-status-select]");
  if (status && [...status.options].some((option) => option.value === parsed.status)) status.value = parsed.status;

  const img = card.querySelector("[data-sprite-name]");
  if (img) {
    img.dataset.spriteName = parsed.name;
    img.alt = `${parsed.name} sprite`;
    img.onerror = () => {
      if (img.dataset.fallbackApplied) return;
      img.dataset.fallbackApplied = "true";
      img.src = "/api/sprite/__missingno";
    };
    delete img.dataset.fallbackApplied;
    img.src = `/api/sprite/${encodeURIComponent(parsed.name)}`;
  }

  const moves = card.querySelector('[data-field="moves"]');
  if (moves && parsed.moves.length) {
    moves.innerHTML = parsed.moves.map((move, index) =>
      `<button class="move ${index === 0 ? "selected" : ""}" type="button" data-move="${escapeAttr(move)}">${escapeHtml(move)} <span class="${typeClass(moveType(move))}">${escapeHtml(moveType(move))}</span><label class="crit-toggle"><input type="checkbox" data-crit-move="${escapeAttr(move)}"/>Crit</label></button>`
    ).join("");
    const moveInput = document.querySelector('[name="move_name"]');
    if (moveInput && card.dataset.setCard === "attacker") moveInput.value = parsed.moves[0];
  }

  for (const [key] of statOrder) {
    const value = parsed.sps[key] || 0;
    card.querySelectorAll(`[data-sp-key="${key}"]`).forEach((input) => {
      input.value = value;
    });
  }
  applyNatureClasses(card);
  syncAbilityEffects();
}

async function loadMoveTypes() {
  try {
    const response = await fetch("/api/move-types");
    if (!response.ok) return;
    moveTypes = { ...moveTypes, ...(await response.json()) };
  } catch (_) {}
}

async function loadSpeciesTypes() {
  try {
    const response = await fetch("/api/species-types");
    if (!response.ok) return;
    const data = await response.json();
    speciesTypes = Object.fromEntries(Object.entries(data).map(([name, types]) => [normalizeName(name), types]));
  } catch (_) {}
}

async function loadSpeciesAbilities() {
  try {
    const response = await fetch("/api/species-abilities");
    if (!response.ok) return;
    const data = await response.json();
    speciesAbilities = Object.fromEntries(Object.entries(data).map(([name, abilities]) => [normalizeName(name), abilities]));
  } catch (_) {}
}

function primaryAbility(name) {
  const abilities = speciesAbilities[normalizeName(name)] || speciesAbilities[normalizeName(megaAlias(name))] || [];
  return abilities[0] || "";
}

function renderTypes(card, name) {
  const target = card.querySelector('[data-field="types"]');
  if (!target) return;
  const types = speciesTypes[normalizeName(name)] || speciesTypes[normalizeName(megaAlias(name))] || [];
  if (!types.length) {
    target.innerHTML = `<span class="type type-unknown">Unknown</span>`;
    return;
  }
  target.innerHTML = types.map((type) => `<span class="type ${typeClass(type)}">${escapeHtml(type)}</span>`).join("");
}

function moveType(move) {
  return moveTypes[move] || "Unknown";
}

function typeClass(type) {
  return `type-badge type-${String(type).toLowerCase().replace(/[^a-z0-9]+/g, "-")}`;
}

function syncRawEditor(editor) {
  syncRawEditorValue(editor);
  const wasFocused = document.activeElement === editor;
  const selectionStart = editor.selectionStart;
  const selectionEnd = editor.selectionEnd;
  const card = editor.closest("[data-set-card]");
  if (card) renderCard(card, parseSet(editor.value));
  if (wasFocused) {
    const max = editor.value.length;
    editor.setSelectionRange(Math.min(selectionStart, max), Math.min(selectionEnd, max));
  }
}

function syncRawEditorValue(editor) {
  const hidden = document.querySelector(`[name="${editor.dataset.hiddenTarget}"]`);
  if (hidden) hidden.value = editor.value;
}

function applyNatureClasses(card) {
  const nature = card.querySelector("[data-card-nature]")?.value || "Hardy";
  const [boost, nerf] = natureEffects[nature] || [];
  card.querySelectorAll("[data-sp-key]").forEach((input) => {
    input.classList.toggle("nature-boost", input.dataset.spKey === boost);
    input.classList.toggle("nature-nerf", input.dataset.spKey === nerf);
  });
}

function initToggles() {
  document.querySelectorAll("[data-toggle-group] label").forEach((label) => {
    const input = label.querySelector("input");
    const sync = () => label.classList.toggle("is-on", input.checked);
    sync();
    label.addEventListener("click", () => setTimeout(() => {
      delete input.dataset.auto;
      if (input.type === "radio") {
        document.querySelectorAll(`input[name="${input.name}"]`).forEach((peer) => {
          peer.closest("label")?.classList.toggle("is-on", peer.checked);
        });
      } else sync();
      saveState();
      autoRun();
    }, 0));
  });
}

function initSpBoxes() {
  document.querySelectorAll("[data-sp-key]").forEach((input) => {
    input.addEventListener("input", () => {
      input.value = Math.max(0, Math.min(32, Number(input.value || 0)));
      const card = input.closest("[data-set-card]");
      if (!card) return;
      card.querySelectorAll(`[data-sp-key="${input.dataset.spKey}"]`).forEach((peer) => {
        if (peer !== input) peer.value = input.value;
      });
      rewriteCardSet(card);
      saveState();
      autoRun();
    });
  });
}

function initBoostBoxes() {
  document.querySelectorAll("[data-boost-key]").forEach((input) => {
    input.addEventListener("input", () => {
      delete input.dataset.auto;
      delete input.dataset.base;
      if (input.value === "" || input.value === "-") return;
      input.value = Math.max(-6, Math.min(6, Number(input.value || 0)));
      saveState();
    });
    input.addEventListener("blur", () => {
      const parsed = Number(input.value);
      input.value = Number.isFinite(parsed) ? Math.max(-6, Math.min(6, parsed)) : 0;
      saveState();
      autoRun();
    });
  });
}

function initAbilityToggles() {
  document.querySelectorAll("[data-ability-toggle]").forEach((toggle) => {
    toggle.addEventListener("change", () => {
      const card = toggle.closest("[data-set-card]");
      const editor = card?.querySelector(".raw-editor");
      if (!card || !editor) return;
      editor.value = replaceOrInsertAfter(
        editor.value,
        /^Ability On:/i,
        `Ability On: ${toggle.checked ? "true" : "false"}`,
        /^Ability:/i
      );
      syncRawEditor(editor);
      syncAbilityEffects();
      saveState();
      autoRun();
    });
  });
}

function initNatures() {
  document.querySelectorAll("[data-card-nature]").forEach((select) => {
    select.addEventListener("change", () => {
      const card = select.closest("[data-set-card]");
      if (card) rewriteCardSet(card);
      saveState();
      autoRun();
    });
  });
}

function initStatuses() {
  document.querySelectorAll("[data-status-select]").forEach((select) => {
    select.addEventListener("change", () => {
      const card = select.closest("[data-set-card]");
      const editor = card?.querySelector(".raw-editor");
      if (!card || !editor) return;
      editor.value = setWithStatus(editor.value, card.dataset.setCard);
      syncRawEditor(editor);
      saveState();
      autoRun();
    });
  });
}

function initRawEditors() {
  document.querySelectorAll(".raw-editor").forEach((editor) => {
    syncRawEditor(editor);
    let renderTimer = null;
    editor.addEventListener("input", () => {
      syncRawEditorValue(editor);
      saveState();
      clearTimeout(renderTimer);
      renderTimer = setTimeout(() => {
        syncRawEditor(editor);
        autoRun();
      }, 350);
    });
    editor.addEventListener("blur", () => {
      clearTimeout(renderTimer);
      syncRawEditor(editor);
      autoRun();
    });
  });
  document.querySelectorAll(".raw-toggle").forEach((button) => {
    button.addEventListener("click", () => button.closest("[data-set-card]")?.classList.toggle("show-raw"));
  });
}

function initMoves() {
  document.addEventListener("click", (event) => {
    const move = event.target.closest(".move");
    if (!move) return;
    if (event.target.closest(".crit-toggle")) {
      saveState();
      autoRun();
      return;
    }
    move.parentElement.querySelectorAll(".move").forEach((node) => node.classList.remove("selected"));
    move.classList.add("selected");
    const moveInput = document.querySelector('[name="move_name"]');
    if (moveInput && move.dataset.move) moveInput.value = move.dataset.move;
    saveState();
    autoRun();
  });
}

function initPersistentInputs() {
  document.querySelectorAll('[name="hp_percent"], [name="max_ko_chance"], [name="min_ko_chance"], [name="limit"], [name="hit_goal"]').forEach((input) => {
    input.addEventListener("input", saveState);
    input.addEventListener("change", () => {
      saveState();
      autoRun();
    });
  });
}

function initSwap() {
  document.querySelector(".swap-action")?.addEventListener("click", () => {
    const attacker = document.querySelector('[data-set-card="attacker"] .raw-editor');
    const defender = document.querySelector('[data-set-card="defender"] .raw-editor');
    if (!attacker || !defender) return;
    [attacker.value, defender.value] = [defender.value, attacker.value];
    syncRawEditor(attacker);
    syncRawEditor(defender);
    saveState();
  });
}

function fieldPayload() {
  syncAbilityEffects();
  const value = (name) => document.querySelector(`input[name="${name}"]:checked`)?.value || "None";
  const checked = (name) => document.querySelector(`input[name="${name}"]`)?.checked || false;
  const boostValue = (name) => {
    const parsed = Number(document.querySelector(`[name="${name}"]`)?.value || 0);
    return Number.isFinite(parsed) ? parsed : 0;
  };
  const boosts = (prefix) => ({
    attack: boostValue(`${prefix}_attack`),
    defense: boostValue(`${prefix}_defense`),
    special_attack: boostValue(`${prefix}_special_attack`),
    special_defense: boostValue(`${prefix}_special_defense`),
    speed: boostValue(`${prefix}_speed`),
  });
  return {
    format: value("format"), terrain: value("terrain"), weather: value("weather"),
    gravity: checked("gravity"), fairy_aura: checked("fairy_aura"), protect: checked("protect"),
    helping_hand: checked("helping_hand"), attacker_tailwind: false,
    defender_tailwind: false, defender_reflect: checked("defender_reflect"),
    defender_light_screen: checked("defender_light_screen"), defender_aurora_veil: checked("defender_aurora_veil"),
    defender_friend_guard: checked("defender_friend_guard"), attacker_boosts: boosts("attacker"), defender_boosts: boosts("defender"),
  };
}

function currentPayload() {
  const hitGoal = Math.max(1, Math.min(3, number("hit_goal", 1)));
  const base = {
    attacker_set: setWithStatus(setWithAbilityState(document.querySelector('[name="attacker_set"]')?.value || "", "attacker"), "attacker"),
    defender_set: setWithStatus(setWithAbilityState(document.querySelector('[name="defender_set"]')?.value || "", "defender"), "defender"),
    move_name: document.querySelector('[name="move_name"]')?.value || "",
    move_times_affected: 0,
    critical: selectedCrit(),
    field: fieldPayload(),
  };
  const path = location.pathname;
  if (path.includes("ko")) return { path: "/api/ko", body: { ...base, min_ko_chance: koChance("min_ko_chance", 16), nature: natureValue("attacker"), optimize_nature: optimizeNature("attacker"), limit: number("limit", 10) } };
  if (path.includes("optimize")) return { path: "/api/optimize/defensive", body: { benchmarks: [base], full_spend: false, locked: lockedStats(), limit: number("limit", 10) } };
  if (hitGoal > 1) {
    return {
      path: "/api/survive-sequence",
      body: {
        defender_set: base.defender_set,
        hits: Array.from({ length: hitGoal }, () => ({
          attacker_set: base.attacker_set,
          move_name: base.move_name,
          move_times_affected: base.move_times_affected,
          critical: base.critical,
          field: base.field,
        })),
        max_ko_chance: koChance("max_ko_chance", 2),
        hp_percent: number("hp_percent", 100),
        nature: natureValue("defender"),
        optimize_nature: optimizeNature("defender"),
        limit: number("limit", 10),
      },
    };
  }
  return { path: "/api/survive", body: { ...base, max_ko_chance: koChance("max_ko_chance", 2), hp_percent: number("hp_percent", 100), nature: natureValue("defender"), optimize_nature: optimizeNature("defender"), limit: number("limit", 10) } };
}

function lockedStats() {
  const read = (name) => {
    const value = document.querySelector(`[name="${name}"]`)?.value;
    return value === "" || value == null ? null : Number(value);
  };
  return { hp: read("lock_hp"), attack: read("lock_attack"), defense: read("lock_defense"), special_attack: read("lock_special_attack"), special_defense: read("lock_special_defense"), speed: read("lock_speed") };
}

function natureValue(cardKey) {
  const value = document.querySelector(`[data-set-card="${cardKey}"] [data-card-nature]`)?.value || "";
  return value && value !== "Any" ? value : null;
}

function optimizeNature(cardKey) {
  return document.querySelector(`[data-set-card="${cardKey}"] [data-card-nature]`)?.value === "Any";
}

function number(name, fallback) {
  return Number(document.querySelector(`[name="${name}"]`)?.value || fallback);
}

function koChance(name, fallbackRolls) {
  const rolls = Number(document.querySelector(`[name="${name}"]`)?.value || fallbackRolls);
  return Math.max(0, Math.min(16, rolls)) / 16;
}

function collectState() {
  const fieldInput = (selector) => document.querySelector(selector);
  return {
    attacker: document.querySelector('[data-set-card="attacker"] .raw-editor')?.value || "",
    defender: document.querySelector('[data-set-card="defender"] .raw-editor')?.value || "",
    move: document.querySelector('[name="move_name"]')?.value || "",
    hpPercent: fieldInput('[name="hp_percent"]')?.value || "100",
    maxKo: fieldInput('[name="max_ko_chance"]')?.value || "2",
    minKo: fieldInput('[name="min_ko_chance"]')?.value || "16",
    hitGoal: fieldInput('[name="hit_goal"]')?.value || "1",
    limit: fieldInput('[name="limit"]')?.value || "10",
    attackerNature: fieldInput('[data-set-card="attacker"] [data-card-nature]')?.value || "",
    defenderNature: fieldInput('[data-set-card="defender"] [data-card-nature]')?.value || "",
    attackerAbilityOn: fieldInput('[data-set-card="attacker"] [data-ability-toggle]')?.checked ?? true,
    defenderAbilityOn: fieldInput('[data-set-card="defender"] [data-ability-toggle]')?.checked ?? true,
    attackerStatus: fieldInput('[data-set-card="attacker"] [data-status-select]')?.value || "Healthy",
    defenderStatus: fieldInput('[data-set-card="defender"] [data-status-select]')?.value || "Healthy",
    crits: Object.fromEntries([...document.querySelectorAll("[data-crit-move]")].map((input) => [input.dataset.critMove, input.checked])),
    boosts: Object.fromEntries([...document.querySelectorAll("[data-boost-key]")].map((input) => [input.name, input.value])),
    field: {
      format: document.querySelector('input[name="format"]:checked')?.value || "Doubles",
      terrain: document.querySelector('input[name="terrain"]:checked')?.value || "None",
      weather: document.querySelector('input[name="weather"]:checked')?.value || "None",
      checks: Object.fromEntries(["fairy_aura", "gravity", "protect", "helping_hand", "defender_aurora_veil", "defender_reflect", "defender_light_screen", "defender_friend_guard"].map((name) => [name, fieldInput(`[name="${name}"]`)?.checked || false])),
    },
  };
}

function saveState() {
  try {
    localStorage.setItem(storageKey, JSON.stringify(collectState()));
  } catch (_) {}
}

function restoreState() {
  try {
    const state = JSON.parse(localStorage.getItem(storageKey) || "null");
    if (!state) return;
    restoredState = state;
    setValue('[data-set-card="attacker"] .raw-editor', state.attacker);
    setValue('[data-set-card="defender"] .raw-editor', state.defender);
    setValue('[name="move_name"]', state.move);
    setValue('[name="hp_percent"]', state.hpPercent);
    setValue('[name="max_ko_chance"]', normalizeKoRollValue(state.maxKo, 2));
    setValue('[name="min_ko_chance"]', normalizeKoRollValue(state.minKo, 16));
    setValue('[name="hit_goal"]', state.hitGoal);
    setValue('[name="limit"]', state.limit);
    setValue('[data-set-card="attacker"] [data-card-nature]', state.attackerNature);
    setValue('[data-set-card="defender"] [data-card-nature]', state.defenderNature);
    setChecked('[data-set-card="attacker"] [data-ability-toggle]', state.attackerAbilityOn);
    setChecked('[data-set-card="defender"] [data-ability-toggle]', state.defenderAbilityOn);
    setValue('[data-set-card="attacker"] [data-status-select]', state.attackerStatus);
    setValue('[data-set-card="defender"] [data-status-select]', state.defenderStatus);
    for (const [name, value] of Object.entries(state.boosts || {})) setValue(`[name="${cssEscape(name)}"]`, value);
    setRadio("format", state.field?.format);
    setRadio("terrain", state.field?.terrain);
    setRadio("weather", state.field?.weather);
    for (const [name, checked] of Object.entries(state.field?.checks || {})) setChecked(`[name="${cssEscape(name)}"]`, checked);
    for (const [move, checked] of Object.entries(state.crits || {})) setChecked(`[data-crit-move="${cssEscape(move)}"]`, checked);
  } catch (_) {}
}

function applyRestoredDynamicState() {
  const state = restoredState;
  if (!state) return;
  setValue('[data-set-card="attacker"] [data-status-select]', state.attackerStatus);
  setValue('[data-set-card="defender"] [data-status-select]', state.defenderStatus);
  for (const [move, checked] of Object.entries(state.crits || {})) setChecked(`[data-crit-move="${cssEscape(move)}"]`, checked);
  document.querySelectorAll("[data-status-select]").forEach((select) => {
    const card = select.closest("[data-set-card]");
    const editor = card?.querySelector(".raw-editor");
    if (card && editor) editor.value = setWithStatus(editor.value, card.dataset.setCard);
  });
  document.querySelectorAll(".raw-editor").forEach(syncRawEditor);
}

function setValue(selector, value) {
  if (value == null || value === "") return;
  const input = document.querySelector(selector);
  if (input) input.value = value;
}

function normalizeKoRollValue(value, fallbackRolls) {
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) return String(fallbackRolls);
  if (parsed >= 0 && parsed <= 1 && !Number.isInteger(parsed)) return String(Math.round(parsed * 16));
  return String(Math.max(0, Math.min(16, Math.round(parsed))));
}

function setChecked(selector, checked) {
  const input = document.querySelector(selector);
  if (input && typeof checked === "boolean") input.checked = checked;
}

function setRadio(name, value) {
  if (!value) return;
  const input = document.querySelector(`input[name="${cssEscape(name)}"][value="${cssEscape(value)}"]`);
  if (input) input.checked = true;
}

function cssEscape(value) {
  return String(value).replace(/["\\]/g, "\\$&");
}

async function initRun() {
  document.querySelector("form.workspace")?.addEventListener("submit", async (event) => {
    event.preventDefault();
    const panel = document.querySelector(".results-panel");
    panel.classList.add("loading");
    try {
      const { path, body } = currentPayload();
      const response = await fetch(path, { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify(body) });
      const data = await response.json();
      if (!response.ok) throw new Error(data.error || response.statusText);
      panel.innerHTML = renderResults(data);
      initResultTabs();
      initShare();
    } catch (error) {
      panel.innerHTML = `<div class="results-head"><b>Error</b><span>0 results</span></div><article class="best-card error-card"><h2>Run failed</h2><p>${escapeHtml(error.message)}</p></article>`;
    } finally {
      panel.classList.remove("loading");
    }
  });
}

function renderResults(data) {
  if (data.summary) {
    return resultShell("Damage", "16 rolls", "", `${warningsCard(data.warnings)}${damageCard(data.summary)}<pre class="json-result" data-tab-panel="raw">${escapeHtml((data.rolls || []).join(", "))}</pre>`);
  }
  const matches = Array.isArray(data) ? data : (data.matches || []);
  const best = data.best || matches[0] || null;
  const count = matches.length;
  const bestLabel = best?.sp_line || "No match";
  const body = best ? `${warningsCard(data.warnings)}${bestCard(best)}${damageCard(best.result || best.combined || {})}${matchesTable(matches)}<pre class="json-result" data-tab-panel="raw">${escapeHtml(JSON.stringify(data, null, 2))}</pre>` : `${warningsCard(data.warnings)}<article class="best-card empty-state"><h2>No spread</h2><p>No matching result.</p></article>`;
  return resultShell("Results", `${count} results`, bestLabel, body);
}

function warningsCard(warnings) {
  if (!Array.isArray(warnings) || !warnings.length) return "";
  return `<article class="warning-card">${warnings.map((warning) => `<p>${escapeHtml(warning)}</p>`).join("")}</article>`;
}

function resultShell(title, count, best, body) {
  return `<div class="results-head"><b>${title}</b><span>${count}</span><p>${escapeHtml(best)}</p></div>
<nav class="result-tabs"><button type="button" class="active" data-result-tab="best">Best Spread</button><button type="button" data-result-tab="all">All Results</button><button type="button" data-result-tab="damage">Damage Breakdown</button><button type="button" data-result-tab="raw">Raw Rolls</button></nav>
${body}<div class="result-actions"><button type="button">▣ Copy Set</button><button type="button">⇩ Download JSON</button><button class="share-action" type="button">↗ Share Link</button></div>`;
}

function bestCard(best) {
  const stats = best.final_stats || {};
  return `<article class="best-card" data-tab-panel="best"><h2>Best Spread</h2><div class="best-grid"><div><small>Nature</small><b>${escapeHtml(best.nature || "-")}</b><em>API selected</em></div><div class="spread-big">${escapeHtml(best.sp_line || "-")}<small>Total SP: ${best.total_points ?? "-"} / 66</small></div><div><small>KO Chance</small><b>${percent(best.result?.ko_chance ?? best.combined?.ko_chance)}</b><small>from optimizer</small></div></div>${finalStats(stats)}</article>`;
}

function damageCard(summary) {
  const pmax = Number(summary.percent_max || 0);
  const move = document.querySelector('[name="move_name"]')?.value || "Selected move";
  return `<article class="damage-card" data-tab-panel="damage"><div class="damage-title">vs <b>${escapeHtml(move)}</b> <span>API</span><em>PASS</em></div><div class="damage-grid"><div><small>Damage</small><b>${summary.min_damage ?? "-"} - ${summary.max_damage ?? "-"}</b><span>${fmt(summary.percent_min)}% - ${fmt(summary.percent_max)}%</span></div><div><small>KO Chance</small><b>${percent(summary.ko_chance)}</b><span>calculated</span></div><div><small>Max damage</small><b>${summary.max_damage ?? "-"} HP</b><span>raw HP damage</span></div></div><div class="meter"><span style="width: ${Math.max(0, Math.min(100, pmax))}%"></span></div><p>Goal evaluated by API <strong>Live result</strong></p></article>`;
}

function matchesTable(matches) {
  const rows = matches.slice(0, 12).map((entry) => `<tr><td>${entry.rank ?? "-"}</td><td>${escapeHtml(entry.nature || "-")}</td><td>${escapeHtml(entry.sp_line || "-")}</td><td>${entry.total_points ?? "-"}</td><td>${percent(entry.result?.ko_chance ?? entry.combined?.ko_chance)}</td><td>${entry.result?.min_damage ?? entry.combined?.min_damage ?? "-"} - ${entry.result?.max_damage ?? entry.combined?.max_damage ?? "-"}</td></tr>`).join("");
  return `<article class="table-card" data-tab-panel="all"><h2>All Results</h2><table><tr><th>Rank</th><th>Nature</th><th>SPs</th><th>Total SP</th><th>KO Chance</th><th>Damage</th></tr>${rows}</table></article>`;
}

function finalStats(stats) {
  const map = [["HP", "hp"], ["Atk", "attack"], ["Def", "defense"], ["SpA", "special_attack"], ["SpD", "special_defense"], ["Spe", "speed"]];
  return `<div class="final-stats">${map.map(([label, key]) => `<span>${label}<b>${stats[key] ?? "-"}</b></span>`).join("")}</div>`;
}

function fmt(value) {
  return Number(value || 0).toFixed(1);
}

function percent(value) {
  return typeof value === "number" ? `${(value * 100).toFixed(1)}%` : "-";
}

function initShare() {
  document.querySelector(".share-action")?.addEventListener("click", async () => {
    const payload = {
      a: document.querySelector('[name="attacker_set"]')?.value || "",
      d: document.querySelector('[name="defender_set"]')?.value || "",
      m: document.querySelector('[name="move_name"]')?.value || "",
    };
    const url = `${location.origin}${location.pathname}#${btoa(unescape(encodeURIComponent(JSON.stringify(payload))))}`;
    history.replaceState(null, "", url);
    await navigator.clipboard?.writeText(url).catch(() => {});
  });
  try {
    if (location.hash.length > 1) {
      const payload = JSON.parse(decodeURIComponent(escape(atob(location.hash.slice(1)))));
      const attacker = document.querySelector('[data-set-card="attacker"] .raw-editor');
      const defender = document.querySelector('[data-set-card="defender"] .raw-editor');
      if (attacker && payload.a) attacker.value = payload.a;
      if (defender && payload.d) defender.value = payload.d;
      if (payload.m) document.querySelector('[name="move_name"]').value = payload.m;
    }
  } catch (_) {}
}

function initResultTabs() {
  document.querySelectorAll("[data-result-tab]").forEach((button) => {
    button.addEventListener("click", () => {
      const tab = button.dataset.resultTab;
      document.querySelectorAll("[data-result-tab]").forEach((b) => b.classList.toggle("active", b === button));
      document.querySelectorAll("[data-tab-panel]").forEach((panel) => {
        panel.classList.toggle("tab-hidden", panel.dataset.tabPanel !== tab && !(tab === "best" && panel.dataset.tabPanel === "damage"));
      });
    });
  });
}

function selectedCrit() {
  const move = document.querySelector('[name="move_name"]')?.value || "";
  return document.querySelector(`[data-crit-move="${cssEscape(move)}"]`)?.checked || false;
}

function syncAbilityEffects() {
  clearAutoField();
  const attacker = activeCardAbility("attacker");
  const defender = activeCardAbility("defender");
  [attacker, defender].forEach(applyGlobalAbilityEffect);
  applySideAbilityEffect("attacker", attacker, "defender");
  applySideAbilityEffect("defender", defender, "attacker");
  syncToggleLabels();
}

function activeCardAbility(cardKey) {
  const card = document.querySelector(`[data-set-card="${cardKey}"]`);
  const on = card?.querySelector("[data-ability-toggle]")?.checked;
  if (!on) return "";
  const editor = card.querySelector(".raw-editor");
  if (editor) return parseSet(editor.value).ability;
  return card.querySelector('[data-field="ability"]')?.textContent?.trim() || "";
}

function applyGlobalAbilityEffect(ability) {
  const key = normalizeName(ability);
  if (key === "fairyaura") autoCheck("fairy_aura", true);
  if (["drought", "orichalcumpulse", "megasol"].includes(key)) autoRadio("weather", "Sun");
  if (key === "drizzle") autoRadio("weather", "Rain");
  if (["sandstream", "sandspit"].includes(key)) autoRadio("weather", "Sand");
  if (key === "snowwarning") autoRadio("weather", "Snow");
  if (["electricsurge", "hadronengine"].includes(key)) autoRadio("terrain", "Electric");
  if (key === "grassysurge") autoRadio("terrain", "Grassy");
  if (key === "psychicsurge") autoRadio("terrain", "Psychic");
  if (key === "mistysurge") autoRadio("terrain", "Misty");
}

function applySideAbilityEffect(owner, ability, opponent) {
  const key = normalizeName(ability);
  if (key === "intimidate") applyIntimidate(owner, opponent);
  if (key === "friendguard" && owner === "defender") autoCheck("defender_friend_guard", true);
}

function applyIntimidate(source, target) {
  const targetAbility = normalizeName(activeCardAbility(target));
  if (targetAbility === "mirrorarmor") {
    applyStatDrop(target, source, "attack", -1);
    return;
  }
  if (targetAbility === "guarddog") {
    autoStageDelta(`${target}_attack`, 1);
    return;
  }
  if (preventsIntimidate(targetAbility)) return;
  applyStatDrop(source, target, "attack", -1);
}

function applyStatDrop(source, target, stat, stages) {
  const ability = normalizeName(activeCardAbility(target));
  if (ability === "contrary") {
    autoStageDelta(`${target}_${stat}`, -stages);
    return;
  }
  const multiplier = ability === "simple" ? 2 : 1;
  autoStageDelta(`${target}_${stat}`, stages * multiplier);
  if (stages < 0 && source !== target) {
    if (ability === "defiant") autoStageDelta(`${target}_attack`, 2 * multiplier);
    if (ability === "competitive") autoStageDelta(`${target}_special_attack`, 2 * multiplier);
    if (ability === "rattled") autoStageDelta(`${target}_speed`, 1 * multiplier);
  }
}

function preventsIntimidate(ability) {
  return ["clearbody", "fullmetalbody", "whitesmoke", "hypercutter", "innerfocus", "oblivious", "owntempo", "scrappy"].includes(normalizeName(ability));
}

function autoStageDelta(name, delta) {
  const input = document.querySelector(`[name="${name}"]`);
  if (!input) return;
  if (!input.dataset.auto) input.dataset.base = input.value || "0";
  const current = Number(input.value || 0);
  input.value = String(Math.max(-6, Math.min(6, current + delta)));
  input.dataset.auto = "ability";
}

function clearAutoField() {
  document.querySelectorAll("[data-auto]").forEach((input) => {
    if (input.type === "checkbox") input.checked = false;
    else if (input.type === "number") input.value = input.dataset.base || "0";
    delete input.dataset.auto;
    delete input.dataset.base;
  });
}

function autoCheck(name, checked) {
  const input = document.querySelector(`[name="${name}"]`);
  if (!input) return;
  input.checked = checked;
  input.dataset.auto = "ability";
}

function autoRadio(name, value) {
  const input = document.querySelector(`input[name="${name}"][value="${value}"]`);
  if (!input) return;
  input.checked = true;
  input.dataset.auto = "ability";
}

function autoBoost(name, value) {
  const input = document.querySelector(`[name="${name}"]`);
  if (!input || Number(input.value || 0) !== 0) return;
  input.value = String(value);
  input.dataset.auto = "ability";
}

function syncToggleLabels() {
  document.querySelectorAll("[data-toggle-group] label").forEach((label) => {
    const input = label.querySelector("input");
    if (input) label.classList.toggle("is-on", input.checked);
  });
}

let autoRunTimer = null;
function autoRun() {
  const panel = document.querySelector(".results-panel");
  if (!panel || panel.querySelector(".empty-state")) return;
  clearTimeout(autoRunTimer);
  autoRunTimer = setTimeout(() => document.querySelector("form.workspace")?.requestSubmit(), 250);
}

function escapeHtml(value) {
  return String(value).replace(/[&<>"']/g, (ch) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[ch]));
}

function escapeAttr(value) {
  return escapeHtml(value).replace(/`/g, "&#96;");
}

function parseBool(value) {
  return ["true", "on", "1", "yes"].includes(String(value).trim().toLowerCase());
}

function displayStatus(value) {
  const key = normalizeName(value);
  if (key === "brn" || key === "burn" || key === "burned") return "Burned";
  if (key === "par" || key === "paralyzed") return "Paralyzed";
  if (key === "psn" || key === "poison" || key === "poisoned") return "Poisoned";
  if (key === "tox" || key === "badlypoisoned") return "Badly Poisoned";
  if (key === "slp" || key === "sleep" || key === "asleep") return "Asleep";
  if (key === "drowsy") return "Drowsy";
  if (key === "frz" || key === "frozen") return "Frozen";
  return "Healthy";
}

function normalizeName(value) {
  return String(value || "").toLowerCase().replace(/[^a-z0-9]+/g, "");
}

function megaAlias(value) {
  const name = String(value || "");
  if (name.endsWith("-Mega")) return `Mega ${name.slice(0, -5)}`;
  return name.replace("-Mega-", " Mega ");
}

document.addEventListener("DOMContentLoaded", async () => {
  await loadMoveTypes();
  await loadSpeciesTypes();
  await loadSpeciesAbilities();
  restoreState();
  initShare();
  initRun();
  initToggles();
  initPersistentInputs();
  initSpBoxes();
  initBoostBoxes();
  initAbilityToggles();
  initNatures();
  initStatuses();
  initRawEditors();
  applyRestoredDynamicState();
  initMoves();
  initSwap();
  initResultTabs();
});
