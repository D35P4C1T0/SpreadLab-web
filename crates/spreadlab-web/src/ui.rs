use leptos::prelude::*;
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Damage,
    Survive,
    Sequence,
    Ko,
    Optimize,
}

pub enum ResultBlock {
    Json(String),
    Error(String),
}

pub fn render(mode: Mode, result: Option<ResultBlock>) -> String {
    let body = format!(
        r##"{top}
{sidebar}
<form method="post" action="{action}" class="workspace">
  {hidden_sets}
  <div class="left-workspace"><section class="calc-panel" aria-label="Spread optimization">{calc}</section><main class="sets-panel">{sets}</main></div>
  <div class="right-workspace">{field}<section id="results" class="results-panel" aria-label="Calculation results" aria-live="polite" aria-busy="false" tabindex="-1">{results}</section></div>
  <a class="mobile-result-link" href="#results">View results <span data-mobile-result>Ready</span></a>
</form>
{early_restore}"##,
        top = topbar(mode),
        sidebar = sidebar(),
        action = mode.action(),
        hidden_sets = hidden_sets(mode),
        calc = calculation_panel(mode),
        sets = pokemon_sets(mode),
        field = field_panel(),
        results = results_panel(mode, result.as_ref()),
        early_restore = early_restore_script(),
    );
    render_shell(mode.title(), body)
}

fn early_restore_script() -> &'static str {
    r#"<script>
(() => {
  const key = "spreadlab.webui.state.v1";
  const escapeHtml = (value) => String(value ?? "").replace(/[&<>"']/g, (ch) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[ch]));
  const setValue = (selector, value) => {
    if (value == null || value === "") return;
    const input = document.querySelector(selector);
    if (input) input.value = value;
  };
  const setChecked = (selector, checked) => {
    const input = document.querySelector(selector);
    if (input && typeof checked === "boolean") input.checked = checked;
  };
  const setSelectedMove = (moveName) => {
    if (!moveName) return;
    setValue('[name="move_name"]', moveName);
    document.querySelectorAll(".move").forEach((node) => {
      node.classList.toggle("selected", node.dataset.move === moveName);
    });
  };
  const firstLine = (lines, matcher, fallback = "") => {
    const line = lines.find((entry) => matcher.test(entry));
    return line ? line.replace(matcher, "").trim() : fallback;
  };
  const parseSet = (text) => {
    const lines = String(text || "").split(/\r?\n/).map((line) => line.trim()).filter(Boolean);
    const [rawName = "Unknown", rawItem = "None"] = (lines[0] || "Unknown").split("@").map((part) => part.trim());
    return {
      name: rawName || "Unknown",
      item: rawItem || "None",
      ability: firstLine(lines, /^Ability:\s*/i, "None"),
    };
  };
  const typeClass = (type) => `type-badge type-${String(type).toLowerCase().replace(/[^a-z0-9]+/g, "-")}`;
  const renderCard = (key, text) => {
    const card = document.querySelector(`[data-set-card="${key}"]`);
    if (!card || !text) return;
    const parsed = parseSet(text);
    const name = card.querySelector('[data-field="name"]');
    if (name) name.textContent = parsed.name;
    card.querySelector('[data-field="ability"]')?.replaceChildren(document.createTextNode(parsed.ability));
    card.querySelector('[data-field="item"]')?.replaceChildren(document.createTextNode(parsed.item));
    const types = card.querySelector('[data-field="types"]');
    const savedTypes = key === "attacker" ? window.__spreadlabEarlyState?.attackerTypes : window.__spreadlabEarlyState?.defenderTypes;
    if (types && Array.isArray(savedTypes) && savedTypes.length) {
      types.innerHTML = savedTypes.map((type) => `<span class="pokemon-type-icon" data-type="${escapeHtml(type)}" title="${escapeHtml(type)}" aria-label="${escapeHtml(type)} type"><img src="/assets/type-icons/${String(type).toLowerCase()}.svg" alt="" aria-hidden="true"/><span class="sr-only">${escapeHtml(type)}</span></span>`).join("");
    } else if (types) {
      types.innerHTML = '<span class="type type-unknown">...</span>';
    }
    const sprite = card.querySelector("[data-sprite-name]");
    if (sprite) {
      sprite.dataset.spriteName = parsed.name;
      sprite.src = `/api/sprite/${encodeURIComponent(parsed.name)}?v=static-1`;
      sprite.alt = `${parsed.name} sprite`;
    }
    const item = card.querySelector("[data-item-sprite]");
    if (item) {
      item.dataset.itemSprite = parsed.item;
      item.src = `/api/item-sprite/${encodeURIComponent(parsed.item)}`;
      item.alt = parsed.item;
    }
  };
  try {
    const state = JSON.parse(localStorage.getItem(key) || "null");
    if (!state) return;
    window.__spreadlabEarlyState = state;
setValue('[data-set-card="attacker"] .raw-editor', state.attacker);
setValue('[data-set-card="defender"] .raw-editor', state.defender);
setSelectedMove(state.move);
setValue('[name="move_times_affected"]', state.moveTimesAffected);
setValue('[name="hp_percent"]', state.hpPercent);
    setValue('[name="max_ko_chance"]', state.maxKo);
    setValue('[name="min_ko_chance"]', state.minKo);
    setValue('[name="hit_goal"]', state.hitGoal);
    setValue('[name="limit"]', state.limit);
    setValue('[data-set-card="attacker"] [data-card-nature]', state.attackerNature);
    setValue('[data-set-card="defender"] [data-card-nature]', state.defenderNature);
    setChecked('[data-set-card="attacker"] [data-ability-toggle]', state.attackerAbilityOn);
    setChecked('[data-set-card="defender"] [data-ability-toggle]', state.defenderAbilityOn);
    setValue('[data-set-card="attacker"] [data-status-select]', state.attackerStatus);
    setValue('[data-set-card="defender"] [data-status-select]', state.defenderStatus);
    for (const [name, value] of Object.entries(state.boosts || {})) setValue(`[name="${CSS.escape(name)}"]`, value);
    for (const [move, checked] of Object.entries(state.crits || {})) setChecked(`[data-crit-move="${CSS.escape(move)}"]`, checked);
    renderCard("attacker", state.attacker);
    renderCard("defender", state.defender);
  } catch (_) {}
})();
</script>"#
}

fn render_shell(title: &str, body: String) -> String {
    let title = title.to_owned();
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <title>{format!("SpreadLab - {title}")}</title>
                <link rel="icon" href="/api/item-sprite/Energy%20Root"/>
        <link rel="stylesheet" href="/assets/app.css?v=20260913-2"/>
        <script src="/assets/setdex_ncp-g10.js?v=20260711-5"></script>
        <script defer src="/assets/app.js?v=20260913-2"></script>
            </head>
            <body inner_html=body></body>
        </html>
    }
    .to_html()
}

fn topbar(mode: Mode) -> String {
    let defensive = if mode != Mode::Ko { "active" } else { "" };
    let offensive = if mode == Mode::Ko { "active" } else { "" };
    format!(
        r#"<header class="app-top">
  <div class="brand">
    <div class="brand-mark"><img src="/api/item-sprite/Energy%20Root" alt="SpreadLab"/></div>
    <strong>SpreadLab</strong>
                <span>Champions · Reg M-C</span>
  </div>
  <nav class="mode-tabs" aria-label="Calculator mode">
    <a class="{defensive}" href="/survive">Defensive Calculator</a>
    <a class="{offensive}" href="/ko">Offensive Calculator</a>
  </nav>
  <button type="button" class="settings-action" data-open-dialog="settings" aria-label="Display settings">⚙ <span>Settings</span></button>
</header>"#
    )
}

fn sidebar() -> String {
    format!(
        r##"<aside class="app-sidebar" aria-label="Application navigation">
  <nav>
    <a class="active" href="#" aria-current="page" title="Calculator"><span class="nav-icon" aria-hidden="true">{calculator_icon}</span><span>Calculator</span></a>
    <button type="button" data-open-dialog="saved" title="Saved Sets"><span class="nav-icon" aria-hidden="true">{saved_icon}</span><span>Saved Sets</span></button>
    <button type="button" data-open-dialog="metagame" title="Metagame presets"><span class="nav-icon" aria-hidden="true">{metagame_icon}</span><span>Metagame</span></button>
    <button type="button" data-open-dialog="guides" title="Guides"><span class="nav-icon" aria-hidden="true">{guides_icon}</span><span>Guides</span></button>
  </nav>
  <small><span class="sidebar-context">SpreadLab </span>v{version}<span class="sidebar-context"><br/>Champions · Reg M-C</span></small>
</aside>
<dialog id="shell-dialog" aria-labelledby="shell-dialog-title">
  <div class="dialog-heading"><h2 id="shell-dialog-title"></h2><button type="button" data-close-dialog aria-label="Close dialog"><svg viewBox="0 0 24 24" aria-hidden="true" focusable="false"><path d="m6 6 12 12M18 6 6 18"/></svg></button></div>
  <div data-dialog-content></div>
</dialog>"##,
        version = env!("CARGO_PKG_VERSION"),
        calculator_icon = include_str!("../assets/sidebar-icons/calculator.svg"),
        saved_icon = include_str!("../assets/sidebar-icons/clipboard-list.svg"),
        metagame_icon = include_str!("../assets/sidebar-icons/chart-no-axes-combined.svg"),
        guides_icon = include_str!("../assets/sidebar-icons/book-open.svg"),
    )
}

fn calculation_panel(mode: Mode) -> String {
    let hit_goal = if mode == Mode::Ko {
        ""
    } else {
        r#"<label class="hit-goal">KO target<select name="hit_goal"><option value="1" selected>OHKO</option><option value="2">2HKO</option><option value="3">3HKO</option></select></label>"#
    };
    format!(
        r#"<div class="section-title"><span class="step-index">1</span><div><b>Optimization</b><small>Find the optimal spread · updates automatically</small></div></div>
<input name="move_name" type="hidden" value="Iron Head"/>
{hit_goal}
{effect_controls}
{chance_controls}
<button class="primary" type="submit">Recalculate</button>"#,
        hit_goal = hit_goal,
        effect_controls = effect_controls(mode),
        chance_controls = chance_controls(mode),
    )
}

fn pokemon_sets(mode: Mode) -> String {
    let defender_label = if mode == Mode::Ko {
        "Defender"
    } else {
        "Defender"
    };
    format!(
        r#"<section class="panel sets">
  <div class="section-title sets-title"><div><b>Matchup</b></div><button class="swap-action" type="button" aria-label="Swap attacker and defender"><span>⇄</span> Swap sides</button></div>
  <div class="card-grid">
    <article class="poke-card {attacker_optimized}" data-set-card="attacker">
      <div class="card-head"><div><b class="side-kicker"><span class="side-icon" aria-hidden="true">⚔</span> Attacker</b></div><button class="raw-toggle" type="button" aria-expanded="false">Paste / edit set</button></div>

      <div class="poke-row">
        <div class="sprite"><img data-sprite-name="Kingambit" src="/api/sprite/Kingambit?v=static-1" alt="Kingambit sprite"/></div>
        <div>
          <div class="slot pokemon-slot"><div class="pokemon-combobox select2-container" data-pokemon-combobox="attacker"><button class="select2-choice pokemon-choice" type="button" aria-expanded="false" aria-label="Attacker Pokemon"><span class="select2-chosen" data-pokemon-choice="attacker">Kingambit</span><span class="select2-arrow"><b></b></span></button><div class="pokemon-menu select2-drop" data-pokemon-menu="attacker"><div class="select2-search"><input class="pokemon-selector" data-pokemon-selector="attacker" value="Kingambit" autocomplete="off" role="combobox" aria-label="Search Attacker Pokemon"/></div><div class="pokemon-options" data-pokemon-options="attacker" role="listbox"></div></div></div><label class="form-selector" data-form-control hidden>Form<select data-form-selector="attacker" aria-label="Attacker form"></select></label></div>
          <h2 class="sr-only" data-field="name">Kingambit</h2>
          <div data-field="types"><span class="pokemon-type-icon" title="Dark" aria-label="Dark type"><img src="/assets/type-icons/dark.svg" alt="" aria-hidden="true"/><span class="sr-only">Dark</span></span><span class="pokemon-type-icon" title="Steel" aria-label="Steel type"><img src="/assets/type-icons/steel.svg" alt="" aria-hidden="true"/><span class="sr-only">Steel</span></span></div>

        </div>
      </div>
      <div class="loadout"><div class="ability-control"><label>Ability<span data-field="ability" class="sr-only">Defiant</span><select data-ability-select="attacker" aria-label="attacker ability"><option>Defiant</option></select></label><label class="ability-toggle"><input type="checkbox" data-ability-toggle="attacker" aria-label="attacker ability active" checked/>Active</label></div><div class="item-control"><small>Item</small><p class="item-line"><span class="item-label">Item:</span><img data-item-sprite="Black Glasses" src="/api/item-sprite/Black%20Glasses" alt="Black Glasses"/><span data-field="item" class="sr-only">Black Glasses</span><span class="pokemon-combobox item-combobox select2-container" data-item-combobox="attacker"><button class="select2-choice item-choice" type="button" aria-expanded="false" aria-label="Attacker Item"><span class="select2-chosen" data-item-choice="attacker">Black Glasses</span><span class="select2-arrow"><b></b></span></button><span class="pokemon-menu select2-drop" data-item-menu="attacker"><span class="select2-search"><input class="pokemon-selector item-selector" data-item-selector="attacker" value="Black Glasses" autocomplete="off" role="combobox" aria-label="Search Attacker Item"/></span><span class="pokemon-options" data-item-options="attacker" role="listbox"></span></span></span></p></div></div>
      <div class="condition-row"><label class="status">Status {status_attacker}</label><label class="nature">Nature {nature_attacker}</label></div>
      <textarea class="raw-editor" aria-label="Attacker Showdown set" data-hidden-target="attacker_set">{attacker}</textarea>
      {attacker_sp_row}
      {attacker_boost_row}
      <div class="move-picker"><label>Moves<select data-move-selector aria-label="Move selector"><option value="Iron Head">Iron Head</option></select></label></div>
      <div class="moves" data-field="moves">
        <div class="move selected" data-move="Iron Head"><button class="move-select" type="button">Iron Head <span>Steel</span></button><label class="crit-toggle"><input type="checkbox" data-crit-move="Iron Head"/>Crit</label><button class="move-delete" type="button" data-delete-move="Iron Head" aria-label="Delete Iron Head">&#128465;&#xfe0e;</button></div>
        <div class="move" data-move="Knock Off"><button class="move-select" type="button">Knock Off <span>Dark</span></button><label class="crit-toggle"><input type="checkbox" data-crit-move="Knock Off"/>Crit</label><button class="move-delete" type="button" data-delete-move="Knock Off" aria-label="Delete Knock Off">&#128465;&#xfe0e;</button></div>
        <div class="move" data-move="Sucker Punch"><button class="move-select" type="button">Sucker Punch <span>Dark</span></button><label class="crit-toggle"><input type="checkbox" data-crit-move="Sucker Punch"/>Crit</label><button class="move-delete" type="button" data-delete-move="Sucker Punch" aria-label="Delete Sucker Punch">&#128465;&#xfe0e;</button></div>
        <div class="move" data-move="Swords Dance"><button class="move-select" type="button">Swords Dance <span>Normal</span></button><label class="crit-toggle"><input type="checkbox" data-crit-move="Swords Dance"/>Crit</label><button class="move-delete" type="button" data-delete-move="Swords Dance" aria-label="Delete Swords Dance">&#128465;&#xfe0e;</button></div>
      </div>
      <footer class="card-footer">{set_library}</footer>
    </article>
    <article class="poke-card {defender_optimized}" data-set-card="defender">
      <div class="card-head"><div><b class="side-kicker"><svg class="side-icon" aria-hidden="true" viewBox="0 0 24 24"><path d="M12 3 4 6v6c0 5 8 9 8 9s8-4 8-9V6Z"/></svg> {defender_label}</b></div><button class="raw-toggle" type="button" aria-expanded="false">Paste / edit set</button></div>

      <div class="poke-row">
        <div class="sprite"><img data-sprite-name="Floette-Mega" src="/api/sprite/Floette-Mega?v=static-1" alt="Floette-Mega sprite"/></div>
        <div>
          <div class="slot defender-slot"><div class="pokemon-combobox select2-container" data-pokemon-combobox="defender"><button class="select2-choice pokemon-choice" type="button" aria-expanded="false" aria-label="Defender Pokemon"><span class="select2-chosen" data-pokemon-choice="defender">Floette</span><span class="select2-arrow"><b></b></span></button><div class="pokemon-menu select2-drop" data-pokemon-menu="defender"><div class="select2-search"><input class="pokemon-selector" data-pokemon-selector="defender" value="Floette" autocomplete="off" role="combobox" aria-label="Search Defender Pokemon"/></div><div class="pokemon-options" data-pokemon-options="defender" role="listbox"></div></div></div><label class="form-selector" data-form-control hidden>Form<select data-form-selector="defender" aria-label="Defender form"></select></label></div>
          <h2 class="sr-only" data-field="name">Floette-Mega</h2>
          <div data-field="types"><span class="pokemon-type-icon" title="Fairy" aria-label="Fairy type"><img src="/assets/type-icons/fairy.svg" alt="" aria-hidden="true"/><span class="sr-only">Fairy</span></span></div>

        </div>
      </div>
      <div class="loadout"><div class="ability-control"><label>Ability<span data-field="ability" class="sr-only">Fairy Aura</span><select data-ability-select="defender" aria-label="defender ability"><option>Fairy Aura</option></select></label><label class="ability-toggle"><input type="checkbox" data-ability-toggle="defender" aria-label="defender ability active" checked/>Active</label></div><div class="item-control"><small>Item</small><p class="item-line"><span class="item-label">Item:</span><img data-item-sprite="Floettite" src="/api/item-sprite/Floettite" alt="Floettite"/><span data-field="item" class="sr-only">Floettite</span><span class="pokemon-combobox item-combobox select2-container" data-item-combobox="defender"><button class="select2-choice item-choice" type="button" aria-expanded="false" aria-label="Defender Item"><span class="select2-chosen" data-item-choice="defender">Floettite</span><span class="select2-arrow"><b></b></span></button><span class="pokemon-menu select2-drop" data-item-menu="defender"><span class="select2-search"><input class="pokemon-selector item-selector" data-item-selector="defender" value="Floettite" autocomplete="off" role="combobox" aria-label="Search Defender Item"/></span><span class="pokemon-options" data-item-options="defender" role="listbox"></span></span></span></p></div></div>
      <div class="condition-row"><label class="status">Status {status_defender}</label><label class="nature">Nature {nature_defender}</label></div>
      <textarea class="raw-editor" aria-label="Defender Showdown set" data-hidden-target="defender_set">{defender}</textarea>
      {defender_sp_row}
      {defender_boost_row}

      <div class="defender-note"><span aria-hidden="true">♢</span><span>No moves needed for defender</span><label>Current HP %<input name="hp_percent" type="number" value="100"/></label></div>
      <footer class="card-footer">{set_library}</footer>
    </article>
  </div>
</section>"#,
        attacker = sample_attacker(),
        defender = sample_defender(),
        nature_attacker = nature_select("card-nature", Some("Adamant")),
        nature_defender = nature_select("card-nature defender-card-nature", Some("Timid")),
        status_attacker = status_select("attacker", Some("Healthy")),
        status_defender = status_select("defender", Some("Healthy")),
        attacker_optimized = if mode == Mode::Ko {
            "optimized-card"
        } else {
            ""
        },
        defender_optimized = if mode == Mode::Ko {
            ""
        } else {
            "optimized-card"
        },
        attacker_sp_row = if mode == Mode::Ko {
            optimized_sp_row()
        } else {
            attacker_sp_row().to_owned()
        },
        attacker_boost_row = boost_stage_row("attacker"),
        defender_sp_row = if mode == Mode::Ko {
            defender_sp_row().to_owned()
        } else {
            optimized_sp_row()
        },
        defender_boost_row = boost_stage_row("defender"),
        set_library = set_library_controls(),
    )
}

fn set_library_controls() -> &'static str {
    r#"<div class="set-library">
  <button type="button" data-save-set>Save set</button>
  <button type="button" data-delete-set disabled>Delete</button>
  <div class="save-set-row" data-save-set-row hidden>
    <input type="text" data-save-set-name maxlength="48" placeholder="Set name" aria-label="Saved set name"/>
    <button type="button" data-confirm-save>Save</button>
    <button type="button" data-cancel-save>Cancel</button>
  </div>
</div>"#
}

fn field_panel() -> &'static str {
    r#"<section class="panel field" aria-label="Battle conditions">
  <div class="section-title"><b><span aria-hidden="true">☷</span> Field &amp; Battle Conditions</b></div>
  <div class="field-group"><span class="field-label" id="format-label">Format</span><div class="choice-row format" role="group" aria-labelledby="format-label" data-toggle-group>
    <label><input type="radio" name="format" value="Singles"/>Singles</label>
    <label><input type="radio" name="format" value="Doubles" checked/>Doubles</label>
  </div></div>
  <div class="field-group"><span class="field-label" id="weather-label">Weather</span><div class="choice-row five" role="group" aria-labelledby="weather-label" data-toggle-group>
    <label><input type="radio" name="weather" value="None" checked/>None</label>
    <label><input type="radio" name="weather" value="Sun"/>Sun</label>
    <label><input type="radio" name="weather" value="Rain"/>Rain</label>
    <label><input type="radio" name="weather" value="Sand"/>Sand</label>
    <label><input type="radio" name="weather" value="Snow"/>Snow</label>
  </div></div>
  <div class="field-group"><span class="field-label" id="terrain-label">Terrain</span><div class="choice-row five" role="group" aria-labelledby="terrain-label" data-toggle-group>
    <label><input type="radio" name="terrain" value="None" checked/>None</label>
    <label><input type="radio" name="terrain" value="Electric"/>Electric</label>
    <label><input type="radio" name="terrain" value="Grassy"/>Grassy</label>
    <label><input type="radio" name="terrain" value="Misty"/>Misty</label>
    <label><input type="radio" name="terrain" value="Psychic"/>Psychic</label>
  </div></div>
  <div class="field-group effects-group"><span class="field-label" id="effects-label">Effects</span><div class="conditions" role="group" aria-labelledby="effects-label" data-toggle-group>
    <label><input name="fairy_aura" type="checkbox" value="true"/>Fairy Aura</label>
    <label><input name="gravity" type="checkbox" value="true"/>Gravity</label>
    <label><input name="protect" type="checkbox" value="true"/>Protect</label>
    <label><input name="helping_hand" type="checkbox" value="true"/>Helping Hand</label>
    <label><input name="defender_aurora_veil" type="checkbox" value="true"/>Aurora Veil</label>
    <label><input name="defender_reflect" type="checkbox" value="true"/>Reflect</label>
    <label><input name="defender_light_screen" type="checkbox" value="true"/>Light Screen</label>
    <label><input name="defender_friend_guard" type="checkbox" value="true"/>Friend Guard</label>
  </div></div>
</section>"#
}

fn results_panel(mode: Mode, result: Option<&ResultBlock>) -> String {
    match result {
        Some(ResultBlock::Error(error)) => result_shell(
            "Error",
            "0 results",
            "",
            &format!(
                r#"<article class="best-card error-card"><h2>Run failed</h2><p>{}</p></article>"#,
                escape(error)
            ),
            "",
        ),
        Some(ResultBlock::Json(json)) => render_json_result(mode, json),
        None => result_shell(
            "Results",
            "No run yet",
            "Ready",
            r#"<article class="best-card empty-state"><h2>Live results</h2><p>Select a move to start. Every change updates this panel automatically.</p></article>"#,
            "",
        ),
    }
}

fn render_json_result(mode: Mode, json: &str) -> String {
    let value = match serde_json::from_str::<Value>(json) {
        Ok(value) => value,
        Err(_) => {
            return result_shell(
                "Results",
                "Parse error",
                "",
                &format!(r#"<pre class="json-result visible">{}</pre>"#, escape(json)),
                "",
            )
        }
    };
    if let Some(summary) = value.get("summary") {
        let roll_count = value
            .get("rolls")
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(0);
        return result_shell(
            "Damage",
            &format!("{roll_count} rolls"),
            "",
            &damage_card(summary, "Damage calculation", "PASS", value.get("rolls")),
            "",
        );
    }

    let matches = if value.is_array() {
        value.as_array().cloned().unwrap_or_default()
    } else {
        value
            .get("matches")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
    };
    let best = value
        .get("best")
        .cloned()
        .or_else(|| matches.first().cloned())
        .unwrap_or(Value::Null);
    let count = matches.len();
    let best_label = best
        .get("sp_line")
        .and_then(Value::as_str)
        .unwrap_or("No spread");
    if best.is_null() {
        return result_shell(
            "Results",
            "0 matching spreads",
            "",
            r#"<article class="best-card empty-state"><h2>No matching spread</h2><p>No spread meets this target. Adjust the KO chance, nature, or battle conditions.</p></article>"#,
            "",
        );
    }
    let mut html = String::new();
    if let Some(result) = best.get("result").or_else(|| best.get("combined")) {
        html.push_str(&damage_card(
            result,
            "Benchmark damage",
            "PASS",
            best.get("rolls").or_else(|| result.get("rolls")),
        ));
    }
    html.push_str(&best_spread_card(&best, mode));
    html.push_str(&matches_table(&matches));
    result_shell(
        "Results",
        &format!("{count} results"),
        best_label,
        &html,
        "",
    )
}

fn result_shell(title: &str, count: &str, best: &str, body: &str, extra: &str) -> String {
    format!(
        r#"<div class="results-head"><b>{title}</b><span>{count}</span><p>{best}</p></div>
{body}{extra}
<div class="result-actions"><button type="button" disabled aria-disabled="true">▣ Copy Set</button><button type="button" disabled aria-disabled="true">⇩ Download JSON</button><button class="share-action" type="button" disabled aria-disabled="true">↗ Share Link</button></div>"#
    )
}

fn best_spread_card(best: &Value, _mode: Mode) -> String {
    let nature = best.get("nature").and_then(Value::as_str).unwrap_or("Any");
    let sp_line = best
        .get("sp_line")
        .and_then(Value::as_str)
        .unwrap_or("No match")
        .trim_start_matches("SPs:")
        .trim();
    let total = best
        .get("total_points")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let stats = best.get("final_stats").unwrap_or(&Value::Null);
    format!(
        r#"<article class="best-card" data-tab-panel="best" aria-label="Optimized spread">
  <div class="spread-summary"><b>{nature}</b><span>{sp_line}</span><small>{total} / 66 SP used</small></div>
  {target_hp}
  {stats}
</article>"#,
        target_hp = if _mode == Mode::Ko {
            String::new()
        } else {
            format!(
                r#"<p class="target-hp">Target HP: <b>{}</b></p>"#,
                num(stats, "hp")
            )
        },
        stats = final_stats(stats),
    )
}

fn damage_card(summary: &Value, title: &str, _status: &str, rolls: Option<&Value>) -> String {
    let min = num(summary, "min_damage");
    let max = num(summary, "max_damage");
    let pmin = summary
        .get("percent_min")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let pmax = summary
        .get("percent_max")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let ko = summary
        .get("ko_chance")
        .and_then(Value::as_f64)
        .map(percent)
        .unwrap_or_else(|| "-".to_owned());
    format!(
        r#"<article class="damage-card" data-tab-panel="damage">
  <div class="damage-title"><b>{title}</b></div>
  <div class="damage-grid">
    <div><small>Damage</small><b>{min}–{max} <span class="unit">HP</span></b><span>{pmin:.1}–{pmax:.1}%</span></div>
    <div><small>KO Chance</small><b>{ko}</b></div>
    <div><small>Max damage</small><b>{max} HP</b></div>
  </div>
  <div class="meter"><span style="width: {meter}%"></span></div>
  {rolls}
</article>"#,
        meter = pmax.clamp(0.0, 100.0),
        rolls = damage_rolls(rolls),
    )
}

fn damage_rolls(rolls: Option<&Value>) -> String {
    let Some(rolls) = rolls.and_then(Value::as_array) else {
        return String::new();
    };
    let values = rolls
        .iter()
        .map(|roll| escape(&roll.to_string()))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        r#"<details class="damage-rolls"><summary>{} damage rolls</summary><code>{}</code></details>"#,
        rolls.len(),
        values
    )
}

fn matches_table(matches: &[Value]) -> String {
    let rows = matches
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let row_class = if index == 0 { "best-row" } else { "" };
            let rank = num(entry, "rank");
            let nature = entry.get("nature").and_then(Value::as_str).unwrap_or("-");
            let sp_line = entry.get("sp_line").and_then(Value::as_str).unwrap_or("-");
            let result = entry.get("result").or_else(|| entry.get("combined")).unwrap_or(&Value::Null);
            let ko = result
                .get("ko_chance")
                .and_then(Value::as_f64)
                .map(percent)
                .unwrap_or_else(|| "-".to_owned());
            format!(
                "<tr class=\"{row_class}\"><td>{rank}</td><td>{nature}</td><td>{sp_line}</td><td>{ko}</td><td>{}–{}</td></tr>",
                num(result, "min_damage"),
                num(result, "max_damage")
            )
        })
        .collect::<Vec<_>>()
        .join("");
    format!(
        r#"<article class="table-card" data-tab-panel="all"><h2>All results</h2><div class="table-scroll" tabindex="0" role="region" aria-label="Ranked optimizer results"><table>
<thead><tr><th scope="col">Rank</th><th scope="col">Nature</th><th scope="col">SPs</th><th scope="col">KO chance</th><th scope="col">Damage</th></tr></thead><tbody>{rows}</tbody>
</table></div></article>"#
    )
}

fn final_stats(stats: &Value) -> String {
    let keys = [
        ("HP", "hp"),
        ("Atk", "attack"),
        ("Def", "defense"),
        ("SpA", "special_attack"),
        ("SpD", "special_defense"),
        ("Spe", "speed"),
    ];
    let body = keys
        .into_iter()
        .map(|(label, key)| format!("<span>{label}<b>{}</b></span>", num(stats, key)))
        .collect::<Vec<_>>()
        .join("");
    format!(r#"<div class="final-stats">{body}</div>"#)
}

fn num(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_i64)
        .map(|n| n.to_string())
        .or_else(|| {
            value
                .get(key)
                .and_then(Value::as_u64)
                .map(|n| n.to_string())
        })
        .unwrap_or_else(|| "-".to_owned())
}

fn percent(value: f64) -> String {
    format!("{:.1}%", value * 100.0)
}

fn hidden_sets(mode: Mode) -> String {
    let (attacker_name, defender_name) = match mode {
        Mode::Sequence => ("attacker_set_1", "defender_set"),
        _ => ("attacker_set", "defender_set"),
    };
    let extra = if mode == Mode::Sequence {
        format!(
            r#"<textarea name="attacker_set_2" hidden>{}</textarea>
<input name="move_name_1" value="Iron Head" hidden/>
<input name="move_name_2" value="Giga Drain" hidden/>"#,
            sample_attacker_two()
        )
    } else {
        r#"<input name="mode" value="defensive" hidden/>
<input name="full_spend" value="false" hidden/>"#
            .to_owned()
    };
    format!(
        r#"<textarea name="{attacker_name}" hidden>{attacker}</textarea>
<textarea name="{defender_name}" hidden>{defender}</textarea>
{extra}"#,
        attacker = sample_attacker(),
        defender = sample_defender(),
    )
}

fn chance_controls(mode: Mode) -> String {
    match mode {
        Mode::Ko => r#"<div class="inline-fields">
  <label>Min KO chance{min_ko_select}</label>
  <label>Limit<input name="limit" type="number" min="1" value="10"/></label>
</div>"#
            .replace("{min_ko_select}", &ko_chance_select("min_ko_chance", 16))
            .to_owned(),
        _ => r#"<div class="inline-fields">
  <label>Max KO chance{max_ko_select}</label>
  <label>Limit<input name="limit" type="number" min="1" value="10"/></label>
</div>"#
            .replace("{max_ko_select}", &ko_chance_select("max_ko_chance", 2))
            .to_owned(),
    }
}

fn effect_controls(mode: Mode) -> &'static str {
    match mode {
        Mode::Sequence => {
            r#"<div class="effect-fields">
  <label>Move 1 effect count<input name="move_times_affected_1" type="number" min="0" max="6" step="1" value="0" data-move-effect-count/></label>
  <label>Move 2 effect count<input name="move_times_affected_2" type="number" min="0" max="6" step="1" value="0" data-move-effect-count/></label>
</div>"#
        }
        _ => {
            r#"<label class="effect-field" hidden>Last Respects fainted allies<input name="move_times_affected" type="number" min="0" max="6" step="1" value="0" data-move-effect-count/></label>"#
        }
    }
}

fn ko_chance_select(name: &str, selected_rolls: u8) -> String {
    let options = (0..=16)
        .map(|rolls| {
            let attr = if rolls == selected_rolls {
                " selected"
            } else {
                ""
            };
            let percent = rolls as f32 * 100.0 / 16.0;
            format!(r#"<option value="{rolls}"{attr}>{rolls}/16 ({percent:.2}%)</option>"#)
        })
        .collect::<Vec<_>>()
        .join("");
    format!(r#"<select name="{name}" data-ko-rolls>{options}</select>"#)
}

fn boost_stage_row(prefix: &str) -> String {
    let input = |stat: &str, label: &str| {
        format!(
            r#"<input aria-label="{prefix} {label} boost" name="{prefix}_{stat}" type="number" min="-6" max="6" step="1" value="0" data-boost-key="{stat}"/>"#
        )
    };
    let row = [
        r#"<input aria-label="HP boost unavailable" type="number" value="0" disabled/>"#.to_owned(),
        input("attack", "Atk"),
        input("defense", "Def"),
        input("special_attack", "SpA"),
        input("special_defense", "SpD"),
        input("speed", "Spe"),
    ]
    .join("");
    format!(
        r#"<div class="boost-stage" data-boost-row="{prefix}">
  <div class="boost-hint">Boosts</div>
  <div class="statline boost-statline"><span>HP</span><span>Atk</span><span>Def</span><span>SpA</span><span>SpD</span><span>Spe</span></div>
  <div class="sp-row boost-row">{row}</div>
</div>"#
    )
}

fn _boost_inputs(prefix: &str) -> String {
    ["attack", "defense", "special_attack", "special_defense", "speed"]
        .into_iter()
        .map(|stat| {
            let label = match stat {
                "attack" => "Atk",
                "defense" => "Def",
                "special_attack" => "SpA",
                "special_defense" => "SpD",
                _ => "Spe",
            };
            format!(
                r#"<label>{label}<input name="{prefix}_{stat}" type="number" min="-6" max="6" value="0"/></label>"#
            )
        })
        .collect::<Vec<_>>()
        .join("")
}

fn optimized_sp_row() -> String {
    let cells = [
        ("hp", "HP"),
        ("atk", "Atk"),
        ("def", "Def"),
        ("spa", "SpA"),
        ("spd", "SpD"),
        ("spe", "Spe"),
    ]
    .into_iter()
    .map(|(key, label)| {
        format!(r#"<span><small>{label}</small><b data-optimized-sp="{key}">–</b></span>"#)
    })
    .collect::<Vec<_>>()
    .join("");
    format!(
        r#"<div class="optimized-sp-preview"><div class="boost-hint">Optimized SPs <span>Result preview</span></div><div class="preview-stats">{cells}</div></div>"#
    )
}

fn attacker_sp_row() -> &'static str {
    r#"<div class="boost-hint sp-hint">SPs</div>
<div class="statline sp-statline"><span>HP</span><span>Atk</span><span>Def</span><span>SpA</span><span>SpD</span><span>Spe</span></div>
<div class="sp-row display-sps" data-sp-row="attacker"><input type="number" min="0" max="32" step="1" aria-label="HP stat points" data-sp-key="hp" value="0"/><input type="number" min="0" max="32" step="1" aria-label="Atk stat points" data-sp-key="atk" value="32"/><input type="number" min="0" max="32" step="1" aria-label="Def stat points" data-sp-key="def" value="0"/><input type="number" min="0" max="32" step="1" aria-label="SpA stat points" data-sp-key="spa" value="0"/><input type="number" min="0" max="32" step="1" aria-label="SpD stat points" data-sp-key="spd" value="0"/><input type="number" min="0" max="32" step="1" aria-label="Spe stat points" data-sp-key="spe" value="0"/></div>"#
}

fn defender_sp_row() -> &'static str {
    r#"<div class="boost-hint sp-hint">SPs</div>
<div class="statline sp-statline spread"><span>HP</span><span>Atk</span><span>Def</span><span>SpA</span><span>SpD</span><span>Spe</span></div>
<div class="sp-row display-sps" data-sp-row="defender"><input name="lock_hp" type="number" min="0" max="32" step="1" aria-label="HP stat points" data-sp-key="hp" value="4"/><input name="lock_attack" type="number" min="0" max="32" step="1" aria-label="Atk stat points" data-sp-key="atk" value="0"/><input name="lock_defense" type="number" min="0" max="32" step="1" aria-label="Def stat points" data-sp-key="def" value="0"/><input name="lock_special_attack" type="number" min="0" max="32" step="1" aria-label="SpA stat points" data-sp-key="spa" value="0"/><input name="lock_special_defense" type="number" min="0" max="32" step="1" aria-label="SpD stat points" data-sp-key="spd" value="28"/><input name="lock_speed" type="number" min="0" max="32" step="1" aria-label="Spe stat points" data-sp-key="spe" value="0"/></div>"#
}

fn nature_select(class: &str, selected: Option<&str>) -> String {
    format!(
        r#"<select class="{class}" data-card-nature aria-label="Nature">{}</select>"#,
        nature_options(selected)
    )
}

fn status_select(side: &str, selected: Option<&str>) -> String {
    let statuses = [
        "Healthy",
        "Burned",
        "Paralyzed",
        "Poisoned",
        "Badly Poisoned",
        "Asleep",
        "Drowsy",
        "Frozen",
    ];
    let options = statuses
        .into_iter()
        .map(|status| {
            let attr = if selected == Some(status) {
                " selected"
            } else {
                ""
            };
            format!(r#"<option value="{status}"{attr}>{status}</option>"#)
        })
        .collect::<Vec<_>>()
        .join("");
    format!(r#"<select data-status-select="{side}">{options}</select>"#)
}

fn nature_options(selected: Option<&str>) -> String {
    let natures = [
        ("Any", "Optimize"),
        ("Hardy", "neutral"),
        ("Adamant", "+Atk, -SpA"),
        ("Bold", "+Def, -Atk"),
        ("Brave", "+Atk, -Spe"),
        ("Calm", "+SpD, -Atk"),
        ("Careful", "+SpD, -SpA"),
        ("Gentle", "+SpD, -Def"),
        ("Hasty", "+Spe, -Def"),
        ("Impish", "+Def, -SpA"),
        ("Jolly", "+Spe, -SpA"),
        ("Lax", "+Def, -SpD"),
        ("Lonely", "+Atk, -Def"),
        ("Mild", "+SpA, -Def"),
        ("Modest", "+SpA, -Atk"),
        ("Naive", "+Spe, -SpD"),
        ("Naughty", "+Atk, -SpD"),
        ("Quiet", "+SpA, -Spe"),
        ("Rash", "+SpA, -SpD"),
        ("Relaxed", "+Def, -Spe"),
        ("Sassy", "+SpD, -Spe"),
        ("Timid", "+Spe, -Atk"),
    ];
    natures
        .into_iter()
        .map(|(name, effect)| {
            let attr = if selected == Some(name) {
                " selected"
            } else {
                ""
            };
            format!(r#"<option value="{name}"{attr}>{name} ({effect})</option>"#)
        })
        .collect::<Vec<_>>()
        .join("")
}

fn sample_attacker() -> &'static str {
    "Kingambit @ Black Glasses\nAbility: Defiant\nTera Type: Steel\nAdamant Nature\nSPs: 32 Atk\n- Iron Head\n- Knock Off\n- Sucker Punch\n- Swords Dance"
}

fn sample_attacker_two() -> &'static str {
    "Venusaur @ Miracle Seed\nAbility: Chlorophyll\nTera Type: Grass\nModest Nature\nSPs: 32 HP / 32 SpA / 2 Spe\n- Giga Drain"
}

fn sample_defender() -> &'static str {
    "Floette-Mega @ Floettite\nAbility: Fairy Aura\nLevel: 50\nEVs: 26 HP / 13 Def / 5 SpA / 22 Spe\nTimid Nature\n- Dazzling Gleam\n- Draining Kiss\n- Calm Mind\n- Protect"
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

impl Mode {
    fn title(self) -> &'static str {
        match self {
            Mode::Damage => "Defensive Calculator",
            Mode::Survive => "Defensive Calculator",
            Mode::Sequence => "Defensive Calculator",
            Mode::Ko => "Offensive Calculator",
            Mode::Optimize => "Defensive Calculator",
        }
    }

    fn action(self) -> &'static str {
        match self {
            Mode::Damage => "/damage",
            Mode::Survive => "/survive",
            Mode::Sequence => "/sequence",
            Mode::Ko => "/ko",
            Mode::Optimize => "/optimize",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_optimizer_response_does_not_claim_target_met() {
        let html = render_json_result(Mode::Ko, r#"{"best":null,"matches":[]}"#);
        assert!(html.contains("No matching spread"));
        assert!(!html.contains("Target met"));
    }

    #[test]
    fn server_rendered_rankings_include_every_result() {
        let matches = (1..=20).map(|rank| serde_json::json!({
            "rank": rank, "nature": "Bold", "sp_line": "SPs: 4 HP / 32 Def",
            "total_points": 36, "result": {"ko_chance": 0.0, "min_damage": 25, "max_damage": 30}
        })).collect::<Vec<_>>();
        let html = matches_table(&matches);
        assert_eq!(html.matches("<tr class=").count(), 20);
        assert!(html.contains("best-row"));
    }
}
