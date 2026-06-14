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
        r#"{top}
<form method="post" action="{action}" class="workspace">
  {hidden_sets}
  <aside class="side-column"><section class="calc-panel">{calc}</section>{field}</aside>
  <main class="sets-panel">{sets}</main>
  <section class="results-panel">{results}</section>
</form>"#,
        top = topbar(mode),
        action = mode.action(),
        hidden_sets = hidden_sets(mode),
        calc = calculation_panel(mode),
        sets = pokemon_sets(mode),
        field = field_panel(),
        results = results_panel(mode, result.as_ref()),
    );
    render_shell(mode.title(), body)
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
                <link rel="stylesheet" href="/assets/app.css?v=20260613-6"/>
                <script defer src="/assets/app.js?v=20260613-6"></script>
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
    <span>[Gen 9 Champions] VGC 2026 Reg M-A</span>
  </div>
  <nav class="mode-tabs">
    <a class="{defensive}" href="/survive">Defensive Calculator</a>
    <a class="{offensive}" href="/ko">Offensive Calculator</a>
  </nav>
  <div class="top-actions"></div>
</header>"#
    )
}

fn calculation_panel(mode: Mode) -> String {
    let hit_goal = if mode == Mode::Ko {
        ""
    } else {
        r#"<label class="hit-goal">KO target<select name="hit_goal"><option value="1" selected>OHKO</option><option value="2">2HKO</option><option value="3">3HKO</option></select></label>"#
    };
    format!(
        r#"<div class="section-title"><span>1</span><b>Calculation</b></div>
<input name="move_name" type="hidden" value="Iron Head"/>
{hit_goal}
{chance_controls}
<button class="primary" type="submit">Run</button>
<button class="secondary swap-action" type="button">Swap</button>
<a class="secondary" href="/survive">Clear</a>"#,
        hit_goal = hit_goal,
        chance_controls = chance_controls(mode),
    )
}

fn pokemon_sets(mode: Mode) -> String {
    let defender_label = if mode == Mode::Ko {
        "Defender"
    } else {
        "Defender (Optimized)"
    };
    format!(
        r#"<section class="panel sets">
  <div class="section-title"><span>2</span><b>Pokemon Sets</b></div>
  <div class="card-grid">
    <article class="poke-card {attacker_optimized}" data-set-card="attacker">
      <div class="card-head"><b>Attacker</b><div><button class="raw-toggle" type="button">Raw set</button></div></div>
      <div class="slot">Attacker 1</div>
      <div class="poke-row">
        <div class="sprite"><img data-sprite-name="Kingambit" src="/api/sprite/Kingambit" alt="Kingambit sprite"/></div>
        <div>
          <h2 data-field="name">Kingambit <em>♂</em></h2>
          <div data-field="types"><span class="type type-dark">Dark</span><span class="type type-steel">Steel</span></div>
          <p>Ability: <span data-field="ability">Defiant</span></p><p class="item-line">Item: <span data-field="item">Black Glasses</span><img data-item-sprite="Black Glasses" src="/api/item-sprite/Black%20Glasses" alt="Black Glasses"/></p>
        </div>
      </div>
      <label class="ability-toggle"><input type="checkbox" data-ability-toggle="attacker" checked/> Ability active</label>
      <label class="status">Status {status_attacker}</label>
      <label class="nature">Nature {nature_attacker}</label>
      <textarea class="raw-editor" data-hidden-target="attacker_set">{attacker}</textarea>
      {attacker_sp_row}
      {attacker_boost_row}
      <div class="moves" data-field="moves">
        <button class="move selected" type="button" data-move="Iron Head">Iron Head <span>Steel</span><label class="crit-toggle"><input type="checkbox" data-crit-move="Iron Head"/>Crit</label></button>
        <button class="move" type="button" data-move="Knock Off">Knock Off <span>Dark</span><label class="crit-toggle"><input type="checkbox" data-crit-move="Knock Off"/>Crit</label></button>
        <button class="move" type="button" data-move="Sucker Punch">Sucker Punch <span>Dark</span><label class="crit-toggle"><input type="checkbox" data-crit-move="Sucker Punch"/>Crit</label></button>
        <button class="move" type="button" data-move="Swords Dance">Swords Dance <span>Normal</span><label class="crit-toggle"><input type="checkbox" data-crit-move="Swords Dance"/>Crit</label></button>
      </div>
    </article>
    <article class="poke-card {defender_optimized}" data-set-card="defender">
      <div class="card-head"><b>{defender_label}</b><div><button class="raw-toggle" type="button">Raw set</button></div></div>
      <div class="slot defender-slot"><span>Defender 1</span><label>HP %<input name="hp_percent" type="number" value="100"/></label></div>
      <div class="poke-row">
        <div class="sprite"><img data-sprite-name="Floette-Mega" src="/api/sprite/Floette-Mega" alt="Floette-Mega sprite"/></div>
        <div>
          <h2 data-field="name">Floette-Mega <em>♀</em></h2>
          <div data-field="types"><span class="type type-fairy">Fairy</span></div>
          <p>Ability: <span data-field="ability">Fairy Aura</span></p><p class="item-line">Item: <span data-field="item">Floettite</span><img data-item-sprite="Floettite" src="/api/item-sprite/Floettite" alt="Floettite"/></p>
        </div>
      </div>
      <label class="ability-toggle"><input type="checkbox" data-ability-toggle="defender" checked/> Ability active</label>
      <label class="status">Status {status_defender}</label>
      <label class="nature">Nature {nature_defender}</label>
      <textarea class="raw-editor" data-hidden-target="defender_set">{defender}</textarea>
      {defender_sp_row}
      {defender_boost_row}
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
            ""
        } else {
            attacker_sp_row()
        },
        attacker_boost_row = boost_stage_row("attacker"),
        defender_sp_row = if mode == Mode::Ko {
            defender_sp_row()
        } else {
            ""
        },
        defender_boost_row = boost_stage_row("defender"),
    )
}

fn field_panel() -> &'static str {
    r#"<section class="panel field">
  <div class="section-title"><span>3</span><b>Field & Battle Conditions</b></div>
  <div class="choice-row format" data-toggle-group>
    <label><input type="radio" name="format" value="Singles"/>Singles</label>
    <label><input type="radio" name="format" value="Doubles" checked/>Doubles</label>
  </div>
  <div class="field-divider"></div>
  <div class="choice-row five" data-toggle-group>
    <label><input type="radio" name="terrain" value="None" checked/>None</label>
    <label><input type="radio" name="terrain" value="Electric"/>Electric</label>
    <label><input type="radio" name="terrain" value="Grassy"/>Grassy</label>
    <label><input type="radio" name="terrain" value="Misty"/>Misty</label>
    <label><input type="radio" name="terrain" value="Psychic"/>Psychic</label>
  </div>
  <div class="field-divider"></div>
  <div class="choice-row five" data-toggle-group>
    <label><input type="radio" name="weather" value="None" checked/>None</label>
    <label><input type="radio" name="weather" value="Sun"/>Sun</label>
    <label><input type="radio" name="weather" value="Rain"/>Rain</label>
    <label><input type="radio" name="weather" value="Sand"/>Sand</label>
    <label><input type="radio" name="weather" value="Snow"/>Snow</label>
  </div>
  <div class="field-divider"></div>
  <div class="conditions" data-toggle-group>
    <label><input name="fairy_aura" type="checkbox" value="true"/>Fairy Aura</label>
    <label><input name="gravity" type="checkbox" value="true"/>Gravity</label>
    <label><input name="protect" type="checkbox" value="true"/>Protect</label>
    <label><input name="helping_hand" type="checkbox" value="true"/>Helping Hand</label>
    <label><input name="defender_aurora_veil" type="checkbox" value="true"/>Aurora Veil</label>
    <label><input name="defender_reflect" type="checkbox" value="true"/>Reflect</label>
    <label><input name="defender_light_screen" type="checkbox" value="true"/>Light Screen</label>
    <label><input name="defender_friend_guard" type="checkbox" value="true"/>Friend Guard</label>
  </div>
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
            r#"<article class="best-card empty-state"><h2>Run calculation</h2><p>Results from the SpreadLab API will render here as cards, tables, damage ranges, and rolls.</p></article>"#,
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
        let rolls = value
            .get("rolls")
            .and_then(Value::as_array)
            .map(|rolls| {
                rolls
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_default();
        return result_shell(
            "Damage",
            "16 rolls",
            "",
            &damage_card(summary, "Damage calculation", "PASS"),
            &format!(
                r#"<pre class="json-result" data-tab-panel="raw">{}</pre>"#,
                escape(&rolls)
            ),
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
    let mut html = String::new();
    html.push_str(&best_spread_card(&best, mode));
    if let Some(result) = best.get("result").or_else(|| best.get("combined")) {
        html.push_str(&damage_card(result, "Benchmark damage", "PASS"));
    }
    html.push_str(&matches_table(&matches));
    html.push_str(&format!(
        r#"<pre class="json-result" data-tab-panel="raw">{}</pre>"#,
        escape(json)
    ));
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
<nav class="result-tabs"><button type="button" class="active" data-result-tab="best">Best Spread</button><button type="button" data-result-tab="all">All Results</button><button type="button" data-result-tab="damage">Damage Breakdown</button><button type="button" data-result-tab="raw">Raw Rolls</button></nav>
{body}{extra}
<div class="result-actions"><button type="button">▣ Copy Set</button><button type="button">⇩ Download JSON</button><button class="share-action" type="button">↗ Share Link</button></div>"#
    )
}

fn best_spread_card(best: &Value, mode: Mode) -> String {
    let nature = best.get("nature").and_then(Value::as_str).unwrap_or("Any");
    let sp_line = best
        .get("sp_line")
        .and_then(Value::as_str)
        .unwrap_or("No match");
    let total = best
        .get("total_points")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let result = best.get("result").or_else(|| best.get("combined"));
    let ko = result
        .and_then(|r| r.get("ko_chance"))
        .and_then(Value::as_f64)
        .map(percent)
        .unwrap_or_else(|| "-".to_owned());
    let stats = best.get("final_stats").unwrap_or(&Value::Null);
    let metric = if mode == Mode::Ko {
        "KO Chance"
    } else {
        "KO Chance"
    };
    format!(
        r#"<article class="best-card" data-tab-panel="best">
  <h2>♛ Best Spread</h2>
  <div class="best-grid">
    <div><small>Nature</small><b>{nature}</b><em>API selected</em></div>
    <div class="spread-big">{sp_line}<small>Total SP: {total} / 66</small></div>
    <div><small>{metric}</small><b>{ko}</b><small>from optimizer</small></div>
  </div>
  {stats}
</article>"#,
        stats = final_stats(stats),
    )
}

fn damage_card(summary: &Value, title: &str, status: &str) -> String {
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
  <div class="damage-title">vs <b>{title}</b> <span>API</span><em>{status}</em></div>
  <div class="damage-grid">
    <div><small>Damage</small><b>{min} - {max}</b><span>{pmin:.1}% - {pmax:.1}%</span></div>
    <div><small>KO Chance</small><b>{ko}</b><span>calculated</span></div>
    <div><small>Max damage</small><b>{max} HP</b><span>raw HP damage</span></div>
  </div>
  <div class="meter"><span style="width: {meter}%"></span></div>
  <p>Goal evaluated by API <strong>Live result</strong></p>
</article>"#,
        meter = pmax.clamp(0.0, 100.0),
    )
}

fn matches_table(matches: &[Value]) -> String {
    let rows = matches
        .iter()
        .take(12)
        .map(|entry| {
            let rank = num(entry, "rank");
            let nature = entry.get("nature").and_then(Value::as_str).unwrap_or("-");
            let sp_line = entry.get("sp_line").and_then(Value::as_str).unwrap_or("-");
            let total = num(entry, "total_points");
            let result = entry.get("result").or_else(|| entry.get("combined")).unwrap_or(&Value::Null);
            let ko = result
                .get("ko_chance")
                .and_then(Value::as_f64)
                .map(percent)
                .unwrap_or_else(|| "-".to_owned());
            format!(
                "<tr><td>{rank}</td><td>{nature}</td><td>{sp_line}</td><td>{total}</td><td>{ko}</td><td>{} - {}</td></tr>",
                num(result, "min_damage"),
                num(result, "max_damage")
            )
        })
        .collect::<Vec<_>>()
        .join("");
    format!(
        r#"<article class="table-card" data-tab-panel="all"><h2>All Results</h2><table>
<tr><th>Rank</th><th>Nature</th><th>SPs</th><th>Total SP</th><th>KO Chance</th><th>Damage (min - max)</th></tr>{rows}
</table></article>"#
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
<input name="move_name_2" value="Giga Drain" hidden/>
<input name="move_times_affected_1" value="0" hidden/>
<input name="move_times_affected_2" value="0" hidden/>"#,
            sample_attacker_two()
        )
    } else {
        r#"<input name="move_times_affected" value="0" hidden/>
<input name="mode" value="defensive" hidden/>
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

fn attacker_sp_row() -> &'static str {
    r#"<div class="statline sp-statline"><span>HP</span><span>Atk</span><span>Def</span><span>SpA</span><span>SpD</span><span>Spe</span></div>
<div class="sp-row display-sps" data-sp-row="attacker"><input type="number" min="0" max="32" step="1" data-sp-key="hp" value="0"/><input type="number" min="0" max="32" step="1" data-sp-key="atk" value="32"/><input type="number" min="0" max="32" step="1" data-sp-key="def" value="0"/><input type="number" min="0" max="32" step="1" data-sp-key="spa" value="0"/><input type="number" min="0" max="32" step="1" data-sp-key="spd" value="0"/><input type="number" min="0" max="32" step="1" data-sp-key="spe" value="0"/></div>"#
}

fn defender_sp_row() -> &'static str {
    r#"<div class="statline sp-statline spread"><span>HP</span><span>Atk</span><span>Def</span><span>SpA</span><span>SpD</span><span>Spe</span></div>
<div class="sp-row display-sps" data-sp-row="defender"><input name="lock_hp" type="number" min="0" max="32" step="1" data-sp-key="hp" value="4"/><input name="lock_attack" type="number" min="0" max="32" step="1" data-sp-key="atk" value="0"/><input name="lock_defense" type="number" min="0" max="32" step="1" data-sp-key="def" value="0"/><input name="lock_special_attack" type="number" min="0" max="32" step="1" data-sp-key="spa" value="0"/><input name="lock_special_defense" type="number" min="0" max="32" step="1" data-sp-key="spd" value="28"/><input name="lock_speed" type="number" min="0" max="32" step="1" data-sp-key="spe" value="0"/></div>"#
}

fn nature_select(class: &str, selected: Option<&str>) -> String {
    format!(
        r#"<select class="{class}" data-card-nature>{}</select>"#,
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
