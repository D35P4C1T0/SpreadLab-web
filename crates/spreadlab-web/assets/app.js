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
let pokemonList = [];
let itemList = [];
let moveList = [];
let pokemonSearchList = [];
let itemSearchList = [];
const megaStonePokemon = Object.freeze({
  Abomasite: "Mega Abomasnow",
  Absolite: "Mega Absol",
  Aerodactylite: "Mega Aerodactyl",
  Aggronite: "Mega Aggron",
  Alakazite: "Mega Alakazam",
  Altarianite: "Mega Altaria",
  Ampharosite: "Mega Ampharos",
  Audinite: "Mega Audino",
  Banettite: "Mega Banette",
  Barbaracleite: "Mega Barbaracle",
  Beedrillite: "Mega Beedrill",
  Blastoisinite: "Mega Blastoise",
  Blazikenite: "Mega Blaziken",
  Cameruptite: "Mega Camerupt",
  Chandelurite: "Mega Chandelure",
  "Charizardite X": "Mega Charizard X",
  "Charizardite Y": "Mega Charizard Y",
  Chesnaughtite: "Mega Chesnaught",
  Chimechite: "Mega Chimecho",
  Clefablite: "Mega Clefable",
  Crabominite: "Mega Crabominable",
  Delphoxite: "Mega Delphox",
  Dragalgeite: "Mega Dragalge",
  Dragoninite: "Mega Dragonite",
  Drampanite: "Mega Drampa",
  Eelektrossite: "Mega Eelektross",
  Emboarite: "Mega Emboar",
  Excadrite: "Mega Excadrill",
  Falinksite: "Mega Falinks",
  Feraligite: "Mega Feraligatr",
  Floettite: "Mega Floette",
  Froslassite: "Mega Froslass",
  Galladite: "Mega Gallade",
  Garchompite: "Mega Garchomp",
  Gardevoirite: "Mega Gardevoir",
  Gengarite: "Mega Gengar",
  Glalitite: "Mega Glalie",
  Glimmoranite: "Mega Glimmora",
  Golurkite: "Mega Golurk",
  Greninjite: "Mega Greninja",
  Gyaradosite: "Mega Gyarados",
  Hawluchanite: "Mega Hawlucha",
  Heracronite: "Mega Heracross",
  Houndoominite: "Mega Houndoom",
  Kangaskhanite: "Mega Kangaskhan",
  Lopunnite: "Mega Lopunny",
  Lucarionite: "Mega Lucario",
  Malamarite: "Mega Malamar",
  Manectite: "Mega Manectric",
  Mawileite: "Mega Mawile",
  Medichamite: "Mega Medicham",
  Meganiumite: "Mega Meganium",
  Meowsticite: "Mega Meowstic",
  Metagrossite: "Mega Metagross",
  Pidgeotite: "Mega Pidgeot",
  Pinsirite: "Mega Pinsir",
  Pyroarite: "Mega Pyroar",
  "Raichunite X": "Mega Raichu X",
  "Raichunite Y": "Mega Raichu Y",
  Sablenite: "Mega Sableye",
  Sceptileite: "Mega Sceptile",
  Scizorite: "Mega Scizor",
  Scolipedeite: "Mega Scolipede",
  Scovillainite: "Mega Scovillain",
  Scraftyite: "Mega Scrafty",
  Sharpedonite: "Mega Sharpedo",
  Skarmorite: "Mega Skarmory",
  Slowbronite: "Mega Slowbro",
  Staraptorite: "Mega Staraptor",
  Starminite: "Mega Starmie",
  Steelixite: "Mega Steelix",
  Swampertite: "Mega Swampert",
  Tyranitarite: "Mega Tyranitar",
  Venusaurite: "Mega Venusaur",
  Victreebelite: "Mega Victreebel",
});
const storageKey = "spreadlab.webui.state.v1";
const savedSetsKey = "spreadlab.webui.savedSets.v1";
let restoredState = null;
let builtInSets = [];
let savedSets = [];

const setdexStatLabels = Object.freeze({ hp: "HP", at: "Atk", df: "Def", sa: "SpA", sd: "SpD", sp: "Spe" });

function setdexPresetText(pokemon, set) {
  const item = set.item && normalizeName(set.item) !== "none" ? ` @ ${set.item}` : "";
  const lines = [`${pokemon}${item}`];
  if (set.ability) lines.push(`Ability: ${set.ability}`);
  const points = Object.entries(set.sps || {})
    .filter(([key, value]) => setdexStatLabels[key] && Number(value) > 0)
    .map(([key, value]) => `${Number(value)} ${setdexStatLabels[key]}`);
  lines.push(`SPs: ${points.length ? points.join(" / ") : "0 HP"}`);
  if (set.nature) lines.push(`${set.nature} Nature`);
  for (const move of set.moves || []) if (move) lines.push(`- ${move}`);
  return lines.join("\n");
}

function loadSetdexPresets() {
  if (typeof SETDEX_GEN10 !== "object" || !SETDEX_GEN10) return [];
  return Object.entries(SETDEX_GEN10).flatMap(([pokemon, sets]) =>
    Object.entries(sets).map(([setName, set]) => ({
      id: `gen10:${normalizeName(pokemon)}:${normalizeName(setName)}`,
      name: setName,
      pokemon,
      text: setdexPresetText(pokemon, set),
    })),
  );
}

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
  delete card.dataset.activeSavedSet;
  syncRawEditor(editor);
  refreshSetLibrary(card);
}

function renderCard(card, parsed) {
  const name = card.querySelector('[data-field="name"]');
  if (name) name.innerHTML = `${parsed.name} <em>${card.dataset.setCard === "attacker" ? "♂" : "♀"}</em>`;
  const selector = card.querySelector("[data-pokemon-selector]");
  if (selector && document.activeElement !== selector) selector.value = parsed.name;
  const choice = card.querySelector("[data-pokemon-choice]");
  if (choice) choice.replaceChildren(document.createTextNode(parsed.name));
  const itemSelector = card.querySelector("[data-item-selector]");
  if (itemSelector && document.activeElement !== itemSelector) itemSelector.value = parsed.item;
  const itemChoice = card.querySelector("[data-item-choice]");
  if (itemChoice) itemChoice.replaceChildren(document.createTextNode(parsed.item));
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
  if (moves) {
    if (!parsed.moves.length) {
      moves.innerHTML = `<div class="empty-moves">No moves selected</div>`;
    } else {
    const moveInput = document.querySelector('[name="move_name"]');
    const selectedMove = card.dataset.setCard === "attacker" && moveInput?.value
      ? moveInput.value
      : parsed.moves[0];
    moves.innerHTML = parsed.moves.map((move) =>
      `<div class="move ${move === selectedMove ? "selected" : ""}" data-move="${escapeAttr(move)}"><button class="move-select" type="button">${escapeHtml(move)} <span class="${typeClass(moveType(move))}">${escapeHtml(moveType(move))}</span></button><label class="crit-toggle"><input type="checkbox" data-crit-move="${escapeAttr(move)}"/>Crit</label><button class="move-delete" type="button" data-delete-move="${escapeAttr(move)}" aria-label="Delete ${escapeAttr(move)}"><svg aria-hidden="true" viewBox="0 0 16 16"><path d="M3 4h10M6 2h4l1 2H5l1-2Zm-1 4v7h6V6M7 7v4m2-4v4"/></svg></button></div>`
    ).join("");
    if (card.dataset.setCard === "attacker") setSelectedMove(selectedMove);
    }
  }
  if (card.dataset.setCard === "attacker") renderMoveSelector();

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

async function loadPokemonList() {
  try {
    const response = await fetch("/api/pokemon-list");
    if (!response.ok) return;
    const data = await response.json();
    setPokemonList(data);
  } catch (_) {}
}

async function loadItemList() {
  try {
    const response = await fetch("/api/item-list");
    if (!response.ok) return;
    const data = await response.json();
    setItemList(data);
  } catch (_) {}
}

async function loadMoveList() {
  try {
    const response = await fetch("/api/meta");
    if (!response.ok) return;
    const data = await response.json();
    moveList = sortedUniqueNames(data.moves || []);
    renderMoveSelector();
  } catch (_) {}
}

function renderMoveSelector() {
  const select = document.querySelector("[data-move-selector]");
  if (!select || !moveList.length) return;
  const currentMoves = parseSet(document.querySelector('[data-set-card="attacker"] .raw-editor')?.value || "").moves;
  const available = moveList.filter((move) => !currentMoves.includes(move));
  const full = currentMoves.length >= 4;
  select.innerHTML = `<option value="">${full ? "Four moves selected" : "Add a move…"}</option>` + available
    .map((move) => `<option value="${escapeAttr(move)}">${escapeHtml(move)}</option>`)
    .join("");
  select.value = "";
  select.disabled = full;
}

function setPokemonList(data) {
  pokemonList = sortedUniqueNames(data);
  pokemonSearchList = pokemonList.map((name) => ({ name, key: normalizeName(name) }));
}

function setItemList(data) {
  itemList = sortedUniqueNames(data);
  itemSearchList = itemList.map((name) => ({ name, key: normalizeName(name) }));
}

function sortedUniqueNames(data) {
  return [...new Set(data.filter(Boolean))].sort((a, b) => a.localeCompare(b));
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

function setSelectedMove(moveName) {
  if (!moveName) return;
  const moveInput = document.querySelector('[name="move_name"]');
  if (moveInput) moveInput.value = moveName;
  document.querySelectorAll(".move").forEach((node) => {
    node.classList.toggle("selected", node.dataset.move === moveName);
  });
  syncMoveEffectField(moveName);
}

function syncMoveEffectField(moveName) {
  const field = document.querySelector(".effect-field");
  const input = document.querySelector('[name="move_times_affected"]');
  const show = normalizeName(moveName) === "lastrespects";
  if (field) field.hidden = !show;
  if (input && !show) input.value = "0";
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
      autoRun();
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
      delete card.dataset.activeSavedSet;
      syncRawEditor(editor);
      refreshSetLibrary(card);
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
      delete card.dataset.activeSavedSet;
      syncRawEditor(editor);
      refreshSetLibrary(card);
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
      const card = editor.closest("[data-set-card]");
      if (card) {
        delete card.dataset.activeSavedSet;
        refreshSetLibrary(card);
      }
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
    button.addEventListener("click", () => {
      const card = button.closest("[data-set-card]");
      const expanded = card?.classList.toggle("show-raw") || false;
      button.setAttribute("aria-expanded", String(expanded));
      if (expanded) card?.querySelector(".raw-editor")?.focus();
    });
  });
}

function initSetLibraries() {
  const initialSets = [...document.querySelectorAll("[data-set-card] .raw-editor")]
    .map((editor) => {
      const text = editor.defaultValue.trim();
      const pokemon = parseSet(text).name;
      return { id: `builtin:${normalizeName(pokemon)}`, name: `${pokemon} · Common`, pokemon, text };
    })
    .filter((entry, index, all) => entry.pokemon && all.findIndex((candidate) => candidate.id === entry.id) === index);
  builtInSets = [...loadSetdexPresets(), ...initialSets].filter(
    (entry, index, all) => all.findIndex((candidate) => candidate.id === entry.id) === index,
  );
  savedSets = loadSavedSets();

  document.querySelectorAll("[data-set-card]").forEach((card) => {
    refreshSetLibrary(card);
    card.querySelector("[data-set-library]")?.addEventListener("change", (event) => applySetLibrarySelection(card, event.target.value));
    card.querySelector("[data-save-set]")?.addEventListener("click", () => openSaveSetRow(card));
    card.querySelector("[data-cancel-save]")?.addEventListener("click", () => closeSaveSetRow(card));
    card.querySelector("[data-confirm-save]")?.addEventListener("click", () => saveCurrentSet(card));
    card.querySelector("[data-save-set-name]")?.addEventListener("keydown", (event) => {
      if (event.key === "Enter") {
        event.preventDefault();
        saveCurrentSet(card);
      }
      if (event.key === "Escape") closeSaveSetRow(card);
    });
    card.querySelector("[data-delete-set]")?.addEventListener("click", () => deleteCurrentSavedSet(card));
  });
}

function loadSavedSets() {
  try {
    const parsed = JSON.parse(localStorage.getItem(savedSetsKey) || "[]");
    return Array.isArray(parsed) ? parsed.filter((entry) => entry?.id && entry?.name && entry?.pokemon && entry?.text) : [];
  } catch (_) {
    return [];
  }
}

function persistSavedSets() {
  localStorage.setItem(savedSetsKey, JSON.stringify(savedSets));
}

function setsForPokemon(pokemon) {
  const key = normalizeName(pokemon);
  return {
    builtIn: builtInSets.filter((entry) => normalizeName(entry.pokemon) === key),
    saved: savedSets.filter((entry) => normalizeName(entry.pokemon) === key),
  };
}

function refreshSetLibrary(card) {
  const select = card.querySelector("[data-set-library]");
  const editor = card.querySelector(".raw-editor");
  if (!select || !editor) return;
  const pokemon = parseSet(editor.value).name;
  const available = setsForPokemon(pokemon);
  const activeId = card.dataset.activeSavedSet || "";
  const matchingBuiltIn = available.builtIn.find((entry) => normalizedSetText(entry.text) === normalizedSetText(editor.value));
  const matchingSaved = available.saved.find((entry) => entry.id === activeId && normalizedSetText(entry.text) === normalizedSetText(editor.value));
  const selected = matchingSaved?.id || matchingBuiltIn?.id || "current";
  select.innerHTML = [
    `<option value="current">Current set</option>`,
    ...available.builtIn.map((entry) => `<option value="${escapeAttr(entry.id)}">${escapeHtml(entry.name)}</option>`),
    ...available.saved.map((entry) => `<option value="${escapeAttr(entry.id)}">★ ${escapeHtml(entry.name)}</option>`),
    `<option value="blank">Blank set</option>`,
  ].join("");
  select.value = selected;
  const deleteButton = card.querySelector("[data-delete-set]");
  if (deleteButton) deleteButton.disabled = !matchingSaved;
}

function applySetLibrarySelection(card, id) {
  if (id === "current") return;
  const editor = card.querySelector(".raw-editor");
  if (!editor) return;
  const pokemon = parseSet(editor.value).name;
  const entry = [...builtInSets, ...savedSets].find((candidate) => candidate.id === id);
  editor.value = id === "blank" ? buildBlankPokemonSet(card, pokemon) : entry?.text || editor.value;
  if (entry && id.startsWith("saved:")) card.dataset.activeSavedSet = id;
  else delete card.dataset.activeSavedSet;
  syncRawEditor(editor);
  refreshSetLibrary(card);
  saveState();
  autoRun();
}

function openSaveSetRow(card) {
  const row = card.querySelector("[data-save-set-row]");
  const input = card.querySelector("[data-save-set-name]");
  if (!row || !input) return;
  row.hidden = false;
  input.value = "";
  input.focus();
}

function closeSaveSetRow(card) {
  const row = card.querySelector("[data-save-set-row]");
  if (row) row.hidden = true;
}

function saveCurrentSet(card) {
  const input = card.querySelector("[data-save-set-name]");
  const editor = card.querySelector(".raw-editor");
  const name = input?.value.trim();
  if (!name || !editor) {
    input?.focus();
    return;
  }
  const pokemon = parseSet(editor.value).name;
  const id = `saved:${Date.now()}:${Math.random().toString(36).slice(2, 8)}`;
  savedSets.push({ id, name, pokemon, text: editor.value });
  persistSavedSets();
  card.dataset.activeSavedSet = id;
  closeSaveSetRow(card);
  document.querySelectorAll("[data-set-card]").forEach(refreshSetLibrary);
}

function deleteCurrentSavedSet(card) {
  const id = card.dataset.activeSavedSet;
  if (!id) return;
  savedSets = savedSets.filter((entry) => entry.id !== id);
  persistSavedSets();
  delete card.dataset.activeSavedSet;
  document.querySelectorAll("[data-set-card]").forEach(refreshSetLibrary);
}

function initMoves() {
  document.querySelector("[data-move-selector]")?.addEventListener("change", (event) => {
    addMoveToAttackerSet(event.target.value);
  });
  document.addEventListener("click", (event) => {
    const deleteButton = event.target.closest("[data-delete-move]");
    if (deleteButton) {
      deleteMoveFromAttackerSet(deleteButton.dataset.deleteMove);
      return;
    }
    const move = event.target.closest(".move");
    if (!move) return;
    if (event.target.closest(".crit-toggle")) {
      saveState();
      autoRun();
      return;
    }
    if (event.target.closest(".move-select") && move.dataset.move) setSelectedMove(move.dataset.move);
    saveState();
    autoRun();
  });
}

function addMoveToAttackerSet(moveName) {
  if (!moveName) return;
  const card = document.querySelector('[data-set-card="attacker"]');
  const editor = card?.querySelector(".raw-editor");
  if (!card || !editor) return;
  const parsed = parseSet(editor.value);
  if (parsed.moves.includes(moveName)) {
    setSelectedMove(moveName);
    renderMoveSelector();
    return;
  }
  if (parsed.moves.length >= 4) return;
  editor.value = `${editor.value.trimEnd()}\n- ${moveName}`;
  delete card.dataset.activeSavedSet;
  const moveInput = document.querySelector('[name="move_name"]');
  if (moveInput) moveInput.value = moveName;
  syncRawEditor(editor);
  refreshSetLibrary(card);
  saveState();
  autoRun();
}

function deleteMoveFromAttackerSet(moveName) {
  const card = document.querySelector('[data-set-card="attacker"]');
  const editor = card?.querySelector(".raw-editor");
  if (!card || !editor || !moveName) return;
  editor.value = editor.value
    .split(/\r?\n/)
    .filter((line) => !(line.trim().startsWith("-") && normalizeName(line.replace(/^-+\s*/, "")) === normalizeName(moveName)))
    .join("\n");
  delete card.dataset.activeSavedSet;
  const remainingMoves = parseSet(editor.value).moves;
  const moveInput = document.querySelector('[name="move_name"]');
  if (moveInput?.value === moveName || !remainingMoves.includes(moveInput?.value)) {
    if (moveInput) moveInput.value = remainingMoves[0] || "";
  }
  syncRawEditor(editor);
  refreshSetLibrary(card);
  saveState();
  autoRun();
}

function initPersistentInputs() {
  document.querySelectorAll('[name="hp_percent"], [name="max_ko_chance"], [name="min_ko_chance"], [name="limit"], [name="hit_goal"], [data-move-effect-count]').forEach((input) => {
    input.addEventListener("input", () => {
      saveState();
      autoRun();
    });
    input.addEventListener("change", () => {
      saveState();
      autoRun();
    });
  });
}

function initPokemonSelectors() {
  document.querySelectorAll("[data-pokemon-combobox]").forEach((combo) => {
    const cardKey = combo.dataset.pokemonCombobox;
    const input = combo.querySelector("[data-pokemon-selector]");
    const choice = combo.querySelector(".pokemon-choice");
    const menu = combo.querySelector("[data-pokemon-menu]");
    const optionsNode = combo.querySelector("[data-pokemon-options]");
    const renderOptions = (showAll = false) => renderPokemonOptions(input, optionsNode, showAll);
    renderOptions(true);
    choice?.addEventListener("click", () => {
      input.value = choice.querySelector("[data-pokemon-choice]")?.textContent?.trim() || input.value;
      renderOptions(true);
      openPokemonMenu(choice, menu, input);
    });
    input?.addEventListener("input", () => {
      renderOptions();
      openPokemonMenu(choice, menu, input, false);
    });
    input?.addEventListener("keydown", (event) => {
      const options = [...(optionsNode?.querySelectorAll("[data-pokemon-option]") || [])];
      const active = optionsNode?.querySelector(".is-active");
      const activeIndex = options.indexOf(active);
      if (event.key === "ArrowDown") {
        event.preventDefault();
        setActivePokemonOption(options, Math.min(activeIndex + 1, options.length - 1));
      } else if (event.key === "ArrowUp") {
        event.preventDefault();
        setActivePokemonOption(options, Math.max(activeIndex - 1, 0));
      } else if (event.key === "Enter") {
        event.preventDefault();
        const best = active || options[0];
        if (best) {
          applyPokemonSelection(cardKey, best.dataset.pokemonName, best.dataset.setId);
          closePokemonMenu(choice, menu);
        }
      } else if (event.key === "Escape") {
        closePokemonMenu(choice, menu);
      }
    });
    menu?.addEventListener("mousedown", (event) => {
      const option = event.target.closest("[data-pokemon-option]");
      if (!option) return;
      event.preventDefault();
      applyPokemonSelection(cardKey, option.dataset.pokemonName, option.dataset.setId);
      closePokemonMenu(choice, menu);
    });
    document.addEventListener("mousedown", (event) => {
      if (!combo.contains(event.target)) closePokemonMenu(choice, menu);
    });
  });
}

function initItemSelectors() {
  document.querySelectorAll("[data-item-combobox]").forEach((combo) => {
    const cardKey = combo.dataset.itemCombobox;
    const input = combo.querySelector("[data-item-selector]");
    const choice = combo.querySelector(".item-choice");
    const menu = combo.querySelector("[data-item-menu]");
    const optionsNode = combo.querySelector("[data-item-options]");
    const renderOptions = (showAll = false) => renderItemOptions(input, optionsNode, showAll);
    renderOptions(true);
    choice?.addEventListener("click", () => {
      input.value = choice.querySelector("[data-item-choice]")?.textContent?.trim() || input.value;
      renderOptions(true);
      openPokemonMenu(choice, menu, input);
    });
    input?.addEventListener("input", () => {
      renderOptions();
      openPokemonMenu(choice, menu, input, false);
    });
    input?.addEventListener("keydown", (event) => {
      const options = [...(optionsNode?.querySelectorAll("[data-item-option]") || [])];
      const active = optionsNode?.querySelector(".is-active");
      const activeIndex = options.indexOf(active);
      if (event.key === "ArrowDown") {
        event.preventDefault();
        setActivePokemonOption(options, Math.min(activeIndex + 1, options.length - 1));
      } else if (event.key === "ArrowUp") {
        event.preventDefault();
        setActivePokemonOption(options, Math.max(activeIndex - 1, 0));
      } else if (event.key === "Enter") {
        event.preventDefault();
        const best = active?.dataset.itemName || options[0]?.dataset.itemName;
        if (best) {
          applyItemSelection(cardKey, best);
          closePokemonMenu(choice, menu);
        }
      } else if (event.key === "Escape") {
        closePokemonMenu(choice, menu);
      }
    });
    menu?.addEventListener("mousedown", (event) => {
      const option = event.target.closest("[data-item-option]");
      if (!option) return;
      event.preventDefault();
      applyItemSelection(cardKey, option.dataset.itemName);
      closePokemonMenu(choice, menu);
    });
    document.addEventListener("mousedown", (event) => {
      if (!combo.contains(event.target)) closePokemonMenu(choice, menu);
    });
  });
}

function renderPokemonOptions(input, optionsNode, showAll = false) {
  if (!input || !optionsNode) return;
  const matches = fuzzyPokemonMatches(showAll ? "" : input.value, 24);
  const renderKey = `pokemon:${matches.map(({ name, setId }) => `${name}:${setId || ""}`).join("\u0000")}`;
  if (optionsNode.dataset.renderKey === renderKey) return;
  optionsNode.dataset.renderKey = renderKey;
  optionsNode.innerHTML = matches.length
    ? matches.map(({ name, pokemon, setName, setId }, index) => `<button class="pokemon-option ${setName ? "set-option" : "species-option"} ${index === 0 ? "is-active" : ""}" type="button" role="option" data-pokemon-option data-pokemon-name="${escapeAttr(pokemon || name)}"${setId ? ` data-set-id="${escapeAttr(setId)}"` : ""}>${setName ? `<span>${escapeHtml(setName)}</span>` : `<b>${escapeHtml(name)}</b>`}</button>`).join("")
    : `<div class="pokemon-option empty" role="option" aria-disabled="true">No match</div>`;
}

function renderItemOptions(input, optionsNode, showAll = false) {
  if (!input || !optionsNode) return;
  const matches = fuzzyItemMatches(showAll ? "" : input.value, 24);
  const renderKey = `item:${matches.map(({ name }) => name).join("\u0000")}`;
  if (optionsNode.dataset.renderKey === renderKey) return;
  optionsNode.dataset.renderKey = renderKey;
  optionsNode.innerHTML = matches.length
    ? matches.map(({ name }, index) => `<button class="pokemon-option ${index === 0 ? "is-active" : ""}" type="button" role="option" data-item-option data-item-name="${escapeAttr(name)}">${escapeHtml(name)}</button>`).join("")
    : `<div class="pokemon-option empty" role="option" aria-disabled="true">No match</div>`;
}

function openPokemonMenu(choice, menu, input, selectText = true) {
  if (!menu) return;
  choice?.setAttribute("aria-expanded", "true");
  menu.classList.add("open");
  if (input) {
    requestAnimationFrame(() => {
      input.focus();
      if (selectText) input.select();
    });
  }
}

function closePokemonMenu(choice, menu) {
  if (!menu) return;
  choice?.setAttribute("aria-expanded", "false");
  menu.classList.remove("open");
}

function setActivePokemonOption(options, index) {
  options.forEach((option, optionIndex) => {
    option.classList.toggle("is-active", optionIndex === index);
  });
  options[index]?.scrollIntoView({ block: "nearest" });
}

function fuzzyPokemonMatches(query, limit) {
  const entries = pokemonSearchList.flatMap(({ name, key }) => {
    const sets = setsForPokemon(name).builtIn.map((set) => ({
      name: `${name} (${set.name})`,
      pokemon: name,
      setName: set.name,
      setId: set.id,
      key: normalizeName(`${name} ${set.name}`),
    }));
    return [{ name, pokemon: name, key }, ...sets, {
      name: `${name} (Blank Set)`, pokemon: name, setName: "Blank Set", setId: "blank", key: `${key}blankset`,
    }];
  });
  return fuzzyMatches(entries, query, limit);
}

function fuzzyItemMatches(query, limit) {
  return fuzzyMatches(itemSearchList, query, limit);
}

function fuzzyMatches(entries, query, limit) {
  const normalizedQuery = normalizeName(query);
  if (!normalizedQuery) return entries.slice(0, limit).map((entry) => ({ ...entry, score: 0 }));
  const scored = entries.map((entry) => ({ ...entry, score: fuzzyScore(normalizedQuery, entry.key) }))
    .filter((entry) => entry.score !== null)
    .sort((a, b) => a.score - b.score || a.name.localeCompare(b.name));
  return scored.slice(0, limit);
}

function fuzzyScore(query, candidate) {
  if (!query) return 0;
  if (candidate === query) return -1000;
  if (candidate.startsWith(query)) return -500 + candidate.length - query.length;
  if (candidate.includes(query)) return -250 + candidate.indexOf(query);
  let last = -1;
  let score = 0;
  for (const char of query) {
    const index = candidate.indexOf(char, last + 1);
    if (index < 0) return null;
    score += index - last;
    last = index;
  }
  return score + candidate.length;
}

function applyPokemonSelection(cardKey, rawName, setId = "") {
  const card = document.querySelector(`[data-set-card="${cardKey}"]`);
  const editor = card?.querySelector(".raw-editor");
  if (!card || !editor) return;

  const selected = pokemonList.find((name) => normalizeName(name) === normalizeName(rawName));
  if (!selected) return;
  const previous = parseSet(editor.value).name;
  const selector = card.querySelector("[data-pokemon-selector]");
  const choice = card.querySelector("[data-pokemon-choice]");
  if (selector) selector.value = selected;
  if (choice) choice.replaceChildren(document.createTextNode(selected));
  if (setId && setId !== "blank") {
    const preset = builtInSets.find((entry) => entry.id === setId);
    if (preset) {
      editor.value = preset.text;
      delete card.dataset.activeSavedSet;
      syncRawEditor(editor);
      const label = `${selected} (${preset.name})`;
      if (selector) selector.value = label;
      if (choice) choice.replaceChildren(document.createTextNode(label));
      refreshSetLibrary(card);
      saveState();
      autoRun();
      return;
    }
  }
  if (setId === "blank") {
    editor.value = buildBlankPokemonSet(card, selected);
    delete card.dataset.activeSavedSet;
    syncRawEditor(editor);
    refreshSetLibrary(card);
    saveState();
    autoRun();
    return;
  }
  if (normalizeName(previous) === normalizeName(selected)) return;

  resetCardTraining(card);
  resetCardBoosts(card);
  clearCardMoves(card);
  const commonSet = setsForPokemon(selected).builtIn[0];
  let nextSet = commonSet?.text || buildBlankPokemonSet(card, selected);
  const megaStone = !commonSet && megaStoneForPokemon(selected);
  if (megaStone) nextSet = setFirstLineItem(nextSet, megaStone);
  editor.value = nextSet;
  delete card.dataset.activeSavedSet;
  syncRawEditor(editor);
  refreshSetLibrary(card);
  saveState();
  autoRun();
}

function applyItemSelection(cardKey, rawName) {
  const card = document.querySelector(`[data-set-card="${cardKey}"]`);
  const editor = card?.querySelector(".raw-editor");
  if (!card || !editor) return;

  const selected = itemList.find((name) => normalizeName(name) === normalizeName(rawName));
  if (!selected) return;
  const selector = card.querySelector("[data-item-selector]");
  const choice = card.querySelector("[data-item-choice]");
  if (selector) selector.value = selected;
  if (choice) choice.replaceChildren(document.createTextNode(selected));

  editor.value = setFirstLineItem(editor.value, selected);
  delete card.dataset.activeSavedSet;
  syncRawEditor(editor);
  refreshSetLibrary(card);
  saveState();
  autoRun();
}

function setFirstLineItem(text, item) {
  const lines = text.split(/\r?\n/);
  const [rawName = "Unknown"] = (lines[0] || "Unknown").split("@").map((part) => part.trim());
  lines[0] = normalizeName(item) === "none" ? rawName : `${rawName} @ ${item}`;
  return lines.join("\n");
}

function megaStoneForPokemon(name) {
  const selectedKey = normalizeMegaPokemonKey(name);
  for (const [stone, pokemon] of Object.entries(megaStonePokemon)) {
    const matches = megaPokemonNameVariants(pokemon).some(
      (variant) => normalizeMegaPokemonKey(variant) === selectedKey,
    );
    if (matches) {
      return stone;
    }
  }
  return null;
}

function megaPokemonNameVariants(pokemon) {
  const variants = [pokemon];
  const splitMega = pokemon.match(/^Mega (.+) ([XY])$/);
  if (splitMega) variants.push(`${splitMega[1]}-Mega-${splitMega[2]}`);
  else if (pokemon.startsWith("Mega ")) variants.push(`${pokemon.slice(5)}-Mega`);
  return variants;
}

function normalizeMegaPokemonKey(name) {
  return normalizeName(name).replace(/z$/, "");
}

function buildBlankPokemonSet(card, name) {
  const ability = primaryAbility(name) || "None";
  const nature = card.querySelector("[data-card-nature]")?.value || "Hardy";
  const natureLine = nature && nature !== "Any" ? nature : "Hardy";
  return `${name}\nAbility: ${ability}\n${natureLine} Nature\nSPs: 0 HP`;
}

function resetCardTraining(card) {
  card.querySelectorAll("[data-sp-key]").forEach((input) => {
    input.value = "0";
  });
}

function resetCardBoosts(card) {
  card.querySelectorAll("[data-boost-key]").forEach((input) => {
    if (input.disabled) return;
    input.value = "0";
    delete input.dataset.auto;
    delete input.dataset.base;
  });
}

function clearCardMoves(card) {
  const moves = card.querySelector('[data-field="moves"]');
  if (moves) moves.innerHTML = `<div class="empty-moves">No moves selected</div>`;
  if (card.dataset.setCard === "attacker") {
    const moveInput = document.querySelector('[name="move_name"]');
    if (moveInput) moveInput.value = "";
  }
}

function initSwap() {
  document.querySelector(".swap-action")?.addEventListener("click", () => {
    const attacker = document.querySelector('[data-set-card="attacker"] .raw-editor');
    const defender = document.querySelector('[data-set-card="defender"] .raw-editor');
    if (!attacker || !defender) return;
    [attacker.value, defender.value] = [defender.value, attacker.value];
    delete attacker.closest("[data-set-card]")?.dataset.activeSavedSet;
    delete defender.closest("[data-set-card]")?.dataset.activeSavedSet;
    syncRawEditor(attacker);
    syncRawEditor(defender);
    document.querySelectorAll("[data-set-card]").forEach(refreshSetLibrary);
    saveState();
    autoRun();
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
    move_times_affected: moveEffectCount("move_times_affected"),
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

function moveEffectCount(name) {
  const value = Number(document.querySelector(`[name="${name}"]`)?.value || 0);
  if (!Number.isFinite(value)) return 0;
  return Math.max(0, Math.min(6, Math.round(value)));
}

function koChance(name, fallbackRolls) {
  const rolls = Number(document.querySelector(`[name="${name}"]`)?.value || fallbackRolls);
  return Math.max(0, Math.min(16, rolls)) / 16;
}

function cardTypes(cardKey) {
  return [...document.querySelectorAll(`[data-set-card="${cardKey}"] [data-field="types"] span`)]
    .map((span) => span.textContent.trim())
    .filter((type) => type && type !== "..." && type !== "Unknown");
}

function collectState() {
  const fieldInput = (selector) => document.querySelector(selector);
  return {
    attacker: document.querySelector('[data-set-card="attacker"] .raw-editor')?.value || "",
    defender: document.querySelector('[data-set-card="defender"] .raw-editor')?.value || "",
    attackerTypes: cardTypes("attacker"),
    defenderTypes: cardTypes("defender"),
    move: document.querySelector('[name="move_name"]')?.value || "",
    moveTimesAffected: document.querySelector('[name="move_times_affected"]')?.value || "0",
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

let saveStateTimer = null;

function saveStateNow() {
  try {
    localStorage.setItem(storageKey, JSON.stringify(collectState()));
  } catch (_) {}
}

function saveState() {
  clearTimeout(saveStateTimer);
  saveStateTimer = setTimeout(saveStateNow, 120);
}

function restoreState() {
  try {
    const state = JSON.parse(localStorage.getItem(storageKey) || "null");
    if (!state) return false;
    restoredState = state;
    setValue('[data-set-card="attacker"] .raw-editor', state.attacker);
    setValue('[data-set-card="defender"] .raw-editor', state.defender);
    setValue('[name="move_name"]', state.move);
    setValue('[name="move_times_affected"]', state.moveTimesAffected);
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
    return true;
  } catch (_) {
    return false;
  }
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

let runController = null;

function initRun() {
  document.querySelector("form.workspace")?.addEventListener("submit", async (event) => {
    event.preventDefault();
    if (!canRunCalculation()) return;
    const panel = document.querySelector(".results-panel");
    runController?.abort();
    const controller = new AbortController();
    runController = controller;
    panel.classList.add("loading");
    panel.setAttribute("aria-busy", "true");
    try {
      const { path, body } = currentPayload();
      const response = await fetch(path, { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify(body), signal: controller.signal });
      const data = await response.json();
      if (!response.ok) throw new Error(data.error || response.statusText);
      if (controller.signal.aborted) return;
      await attachBestDamageRolls(data, path, body, controller.signal);
      panel.innerHTML = renderResults(data);
      initShare();
    } catch (error) {
      if (error.name === "AbortError") return;
      panel.innerHTML = `<div class="results-head"><b>Error</b><span>0 results</span></div><article class="best-card error-card"><h2>Run failed</h2><p>${escapeHtml(error.message)}</p></article>`;
    } finally {
      if (runController === controller) {
        panel.classList.remove("loading");
        panel.setAttribute("aria-busy", "false");
      }
    }
  });
}

async function attachBestDamageRolls(data, path, body, signal) {
  if (data.summary || signal.aborted) return;
  const matches = Array.isArray(data) ? data : (data.matches || []);
  const best = data.best || matches[0];
  if (!best) return;
  const damageBody = bestDamagePayload(path, body, best);
  if (!damageBody) return;
  try {
    const response = await fetch("/api/damage", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(damageBody),
      signal,
    });
    if (!response.ok) return;
    const damage = await response.json();
    if (Array.isArray(damage.rolls)) best.rolls = damage.rolls;
  } catch (error) {
    if (error.name === "AbortError") throw error;
  }
}

function bestDamagePayload(path, body, best) {
  let benchmark;
  let optimizedSide;
  if (path === "/api/survive" || path === "/api/ko") {
    benchmark = { ...body };
    optimizedSide = path === "/api/ko" ? "attacker_set" : "defender_set";
  } else if (path === "/api/survive-sequence") {
    const hit = body.hits?.[0];
    if (!hit) return null;
    benchmark = { ...hit, defender_set: body.defender_set };
    optimizedSide = "defender_set";
  } else if (path === "/api/optimize/defensive") {
    benchmark = { ...(body.benchmarks?.[0] || {}) };
    optimizedSide = "defender_set";
  } else {
    return null;
  }
  if (!benchmark[optimizedSide]) return null;
  benchmark[optimizedSide] = setWithOptimizedSpread(benchmark[optimizedSide], best);
  return {
    attacker_set: benchmark.attacker_set,
    defender_set: benchmark.defender_set,
    move_name: benchmark.move_name,
    move_times_affected: benchmark.move_times_affected || 0,
    critical: benchmark.critical || false,
    field: benchmark.field,
  };
}

function setWithOptimizedSpread(text, best) {
  let updated = text;
  if (best.sp_line) updated = replaceOrInsertLine(updated, /^(SPs|EVs):/i, best.sp_line);
  if (best.nature) updated = replaceOrInsertLine(updated, / Nature$/i, `${best.nature} Nature`);
  return updated;
}

function canRunCalculation() {
  return Boolean(
    document.querySelector('[name="move_name"]')?.value &&
    document.querySelector('[name="attacker_set"]')?.value &&
    document.querySelector('[name="defender_set"]')?.value
  );
}

function renderResults(data) {
  if (data.summary) {
    return resultShell("Damage", `${data.rolls?.length || 0} rolls`, "", `${warningsCard(data.warnings)}${damageCard(data.summary, data.rolls)}`);
  }
  const matches = Array.isArray(data) ? data : (data.matches || []);
  const best = data.best || matches[0] || null;
  const count = matches.length;
  const bestLabel = best?.sp_line || "No match";
  const body = best ? `${warningsCard(data.warnings)}${bestCard(best)}${damageCard(best.result || best.combined || {}, best.rolls)}${matchesTable(matches)}` : `${warningsCard(data.warnings)}<article class="best-card empty-state"><h2>No spread</h2><p>No matching result.</p></article>`;
  return resultShell("Results", `${count} results`, bestLabel, body);
}

function warningsCard(warnings) {
  if (!Array.isArray(warnings) || !warnings.length) return "";
  return `<article class="warning-card">${warnings.map((warning) => `<p>${escapeHtml(warning)}</p>`).join("")}</article>`;
}

function resultShell(title, count, best, body) {
  return `<div class="results-head"><b>${title}</b><span>${count}</span><p>${escapeHtml(best)}</p></div>
${body}<div class="result-actions"><button type="button" disabled aria-disabled="true">▣ Copy Set</button><button type="button" disabled aria-disabled="true">⇩ Download JSON</button><button class="share-action" type="button" disabled aria-disabled="true">↗ Share Link</button></div>`;
}

function bestCard(best) {
  const stats = best.final_stats || {};
  return `<article class="best-card" data-tab-panel="best"><h2>Best Spread</h2><div class="best-grid"><div><small>Nature</small><b>${escapeHtml(best.nature || "-")}</b><em>API selected</em></div><div class="spread-big">${escapeHtml(best.sp_line || "-")}<small>Total SP: ${best.total_points ?? "-"} / 66</small></div><div><small>KO Chance</small><b>${percent(best.result?.ko_chance ?? best.combined?.ko_chance)}</b><small>from optimizer</small></div></div>${finalStats(stats)}</article>`;
}

function damageCard(summary, rolls = summary.rolls || []) {
  const pmax = Number(summary.percent_max || 0);
  const move = document.querySelector('[name="move_name"]')?.value || "Selected move";
  const damageRolls = Array.isArray(rolls) && rolls.length
    ? `<div class="damage-rolls"><small>Damage rolls (${rolls.length})</small><code>[${rolls.map((roll) => escapeHtml(roll)).join(", ")}]</code></div>`
    : "";
  return `<article class="damage-card" data-tab-panel="damage"><div class="damage-title">vs <b>${escapeHtml(move)}</b> <span>API</span><em>PASS</em></div><div class="damage-grid"><div><small>Damage</small><b>${summary.min_damage ?? "-"} - ${summary.max_damage ?? "-"}</b><span>${fmt(summary.percent_min)}% - ${fmt(summary.percent_max)}%</span></div><div><small>KO Chance</small><b>${percent(summary.ko_chance)}</b><span>calculated</span></div><div><small>Max damage</small><b>${summary.max_damage ?? "-"} HP</b><span>raw HP damage</span></div></div><div class="meter"><span style="width: ${Math.max(0, Math.min(100, pmax))}%"></span></div>${damageRolls}<p>Goal evaluated by API <strong>Live result</strong></p></article>`;
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
  clearTimeout(autoRunTimer);
  if (!canRunCalculation()) {
    runController?.abort();
    const panel = document.querySelector(".results-panel");
    if (panel) panel.innerHTML = `<div class="results-head"><b>Results</b><span>Waiting</span><p>Select a move</p></div><article class="best-card empty-state"><h2>Add a move</h2><p>Choose a move from the selector to calculate.</p></article>`;
    return;
  }
  autoRunTimer = setTimeout(() => document.querySelector("form.workspace")?.requestSubmit(), 320);
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
  restoreState();
  await Promise.all([
    loadMoveTypes(),
    loadSpeciesTypes(),
    loadSpeciesAbilities(),
    loadPokemonList(),
    loadItemList(),
    loadMoveList(),
  ]);
  initShare();
  initRun();
  initSetLibraries();
  initToggles();
  initPersistentInputs();
  initPokemonSelectors();
  initItemSelectors();
  initSpBoxes();
  initBoostBoxes();
  initAbilityToggles();
  initNatures();
  initStatuses();
  initRawEditors();
  applyRestoredDynamicState();
  saveState();
  initMoves();
  initSwap();
  autoRun();
});

window.addEventListener("pagehide", saveStateNow);
