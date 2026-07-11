mod ui;

use axum::{
    extract::{Form, Path, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use clap::{Parser, Subcommand};
use serde::{de, de::DeserializeOwned, Deserialize, Deserializer, Serialize};
use serde_json::{json, Value};
use spreadlab_rs::{api, data::ChampionsData};
use std::{
    collections::{BTreeSet, HashMap},
    net::SocketAddr,
    path::PathBuf,
    sync::Arc,
};
use thiserror::Error;
use tokio::net::TcpListener;
use tower_http::{services::ServeDir, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Debug, Parser)]
#[command(name = "spreadlab-web")]
#[command(about = "Dedicated SpreadLab WebUI")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Serve {
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        #[arg(long, default_value_t = 3000)]
        port: u16,
    },
}

#[derive(Clone)]
struct AppState {
    data: Arc<ChampionsData>,
}

const POKEMON_CHAMPIONS_ITEMS: &[&str] = &[
    "Abomasite",
    "Absolite",
    "Aerodactylite",
    "Aggronite",
    "Alakazite",
    "Altarianite",
    "Ampharosite",
    "Aspear Berry",
    "Audinite",
    "Babiri Berry",
    "Banettite",
    "Barbaracleite",
    "Beedrillite",
    "Big Root",
    "Black Belt",
    "Black Glasses",
    "Blastoisinite",
    "Blazikenite",
    "BrightPowder",
    "Cameruptite",
    "Chandelurite",
    "Charcoal",
    "Charizardite X",
    "Charizardite Y",
    "Charti Berry",
    "Cheri Berry",
    "Chesnaughtite",
    "Chesto Berry",
    "Chilan Berry",
    "Chimechite",
    "Choice Scarf",
    "Chople Berry",
    "Clefablite",
    "Coba Berry",
    "Colbur Berry",
    "Crabominite",
    "Damp Rock",
    "Delphoxite",
    "Dragalgeite",
    "Dragon Fang",
    "Dragoninite",
    "Drampanite",
    "Eelektrossite",
    "Emboarite",
    "Excadrite",
    "Expert Belt",
    "Fairy Feather",
    "Falinksite",
    "Feraligite",
    "Floettite",
    "Focus Band",
    "Focus Sash",
    "Froslassite",
    "Galladite",
    "Garchompite",
    "Gardevoirite",
    "Gengarite",
    "Glalitite",
    "Glimmoranite",
    "Golurkite",
    "Greninjite",
    "Gyaradosite",
    "Haban Berry",
    "Hard Stone",
    "Hawluchanite",
    "Heat Rock",
    "Heracronite",
    "Houndoominite",
    "Icy Rock",
    "Iron Ball",
    "Kangaskhanite",
    "Kasib Berry",
    "Kebia Berry",
    "King's Rock",
    "Leftovers",
    "Leppa Berry",
    "Life Orb",
    "Light Ball",
    "Light Clay",
    "Lopunnite",
    "Lucarionite",
    "Lum Berry",
    "Magnet",
    "Malamarite",
    "Manectite",
    "Mawileite",
    "Medichamite",
    "Meganiumite",
    "Mental Herb",
    "Meowsticite",
    "Metagrossite",
    "Metal Coat",
    "Metronome",
    "Miracle Seed",
    "Muscle Band",
    "Mystic Water",
    "Never-Melt Ice",
    "Occa Berry",
    "Oran Berry",
    "Passho Berry",
    "Payapa Berry",
    "Pecha Berry",
    "Persim Berry",
    "Pidgeotite",
    "Pinsirite",
    "Poison Barb",
    "Pyroarite",
    "Quick Claw",
    "Raichunite X",
    "Raichunite Y",
    "Rawst Berry",
    "Rindo Berry",
    "Roseli Berry",
    "Sablenite",
    "Sceptileite",
    "Scizorite",
    "Scolipedeite",
    "Scope Lens",
    "Scovillainite",
    "Scraftyite",
    "Sharp Beak",
    "Sharpedonite",
    "Shed Shell",
    "Shell Bell",
    "Shuca Berry",
    "Silk Scarf",
    "SilverPowder",
    "Sitrus Berry",
    "Skarmorite",
    "Slowbronite",
    "Smooth Rock",
    "Soft Sand",
    "Spell Tag",
    "Staraptorite",
    "Starminite",
    "Steelixite",
    "Swampertite",
    "Tanga Berry",
    "TwistedSpoon",
    "Tyranitarite",
    "Venusaurite",
    "Victreebelite",
    "Wacan Berry",
    "White Herb",
    "Wide Lens",
    "Wise Glasses",
    "Yache Berry",
    "Zoom Lens",
];

const TRANSPARENT_PNG: &[u8] = &[
    137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6, 0,
    0, 0, 31, 21, 196, 137, 0, 0, 0, 13, 73, 68, 65, 84, 120, 156, 99, 0, 1, 0, 0, 5, 0, 1, 13, 10,
    45, 180, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
];

#[derive(Debug, Error)]
enum WebError {
    #[error(transparent)]
    Api(#[from] api::ApiError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("task failed: {0}")]
    Join(#[from] tokio::task::JoinError),
    #[error("asset error: {0}")]
    Asset(String),
}

impl IntoResponse for WebError {
    fn into_response(self) -> Response {
        let status = match self {
            WebError::Api(api::ApiError::Showdown(_)) => StatusCode::BAD_REQUEST,
            WebError::Api(api::ApiError::Optimize(_)) => StatusCode::BAD_REQUEST,
            WebError::Json(_) => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, Json(json!({ "error": self.to_string() }))).into_response()
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "spreadlab_web=info,tower_http=info,axum=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    match Cli::parse().command {
        Command::Serve { host, port } => serve(host, port).await,
    }
}

async fn serve(host: String, port: u16) -> anyhow::Result<()> {
    let data = Arc::new(ChampionsData::load()?);
    let state = AppState { data };
    let assets = ServeDir::new("crates/spreadlab-web/assets");
    let app = Router::new()
        .route("/", get(page_damage))
        .route("/damage", get(page_damage).post(form_damage))
        .route("/survive", get(page_survive).post(form_survive))
        .route("/sequence", get(page_sequence).post(form_sequence))
        .route("/ko", get(page_ko).post(form_ko))
        .route("/optimize", get(page_optimize).post(form_optimize))
        .route("/api/meta", get(api_meta))
        .route("/api/damage", post(api_damage))
        .route("/api/survive", post(api_survive))
        .route("/api/survive-sequence", post(api_sequence))
        .route("/api/ko", post(api_ko))
        .route("/api/optimize/defensive", post(api_optimize_defensive))
        .route("/api/optimize/offensive", post(api_optimize_offensive))
        .route("/api/sprite/:name", get(api_sprite))
        .route("/api/item-sprite/:name", get(api_item_sprite))
        .route("/api/move-types", get(api_move_types))
        .route("/api/pokemon-list", get(api_pokemon_list))
        .route("/api/item-list", get(api_item_list))
        .route("/api/species-types", get(api_species_types))
        .route("/api/species-abilities", get(api_species_abilities))
        .route("/api/unsupported-items", get(api_unsupported_items))
        .nest_service("/assets", assets)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr: SocketAddr = format!("{host}:{port}").parse()?;
    let listener = TcpListener::bind(addr).await?;
    tracing::info!("serving SpreadLab WebUI at http://{addr}");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn api_sprite(Path(name): Path<String>) -> Result<impl IntoResponse, WebError> {
    let slug = showdown_sprite_slug(&name);
    let path = PathBuf::from("crates/spreadlab-web/assets/sprites-static/showdown")
        .join(format!("{slug}.img"));
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|error| WebError::Asset(error.to_string()))?;
    }
    if tokio::fs::metadata(&path).await.is_err() {
        let bytes = match fetch_showdown_sprite(&slug).await {
            Some(bytes) => bytes,
            None => {
                let fallback_slug = pokeapi_slug(&sprite_slug(&name));
                fetch_pokeapi_sprite(&fallback_slug)
                    .await
                    .unwrap_or_else(|| MISSINGNO_GEN3.to_vec())
            }
        };
        tokio::fs::write(&path, bytes)
            .await
            .map_err(|error| WebError::Asset(error.to_string()))?;
    }
    let bytes = tokio::fs::read(path)
        .await
        .map_err(|error| WebError::Asset(error.to_string()))?;
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, pokemon_sprite_content_type(&bytes));
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=604800"),
    );
    Ok((headers, bytes))
}

const MISSINGNO_GEN3: &[u8] = include_bytes!("../assets/fallbacks/missingno-gen3.png");

async fn fetch_showdown_sprite(slug: &str) -> Option<Vec<u8>> {
    let response = reqwest::get(format!(
        "https://play.pokemonshowdown.com/sprites/ani/{slug}.gif"
    ))
    .await
    .ok()?;
    if !response.status().is_success() {
        return None;
    }
    response.bytes().await.ok().map(|bytes| bytes.to_vec())
}

async fn fetch_pokeapi_sprite(slug: &str) -> Option<Vec<u8>> {
    let response = reqwest::get(format!("https://pokeapi.co/api/v2/pokemon/{slug}"))
        .await
        .ok()?;
    if !response.status().is_success() {
        return None;
    }
    let details = response.json::<Value>().await.ok()?;
    let sprite_url = details
        .pointer("/sprites/front_default")
        .and_then(Value::as_str)
        .or_else(|| {
            details
                .pointer("/sprites/versions/generation-viii/icons/front_default")
                .and_then(Value::as_str)
        })
        .or_else(|| {
            details
                .pointer("/sprites/other/official-artwork/front_default")
                .and_then(Value::as_str)
        })?;
    reqwest::get(sprite_url)
        .await
        .ok()?
        .bytes()
        .await
        .ok()
        .map(|bytes| bytes.to_vec())
}

fn pokemon_sprite_content_type(bytes: &[u8]) -> HeaderValue {
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        HeaderValue::from_static("image/gif")
    } else {
        HeaderValue::from_static("image/png")
    }
}

async fn api_item_sprite(Path(name): Path<String>) -> Result<impl IntoResponse, WebError> {
    let slug = item_slug(&name);
    let path =
        PathBuf::from("crates/spreadlab-web/assets/item-sprites").join(format!("{slug}.png"));
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|error| WebError::Asset(error.to_string()))?;
    }
    let stale_empty_fallback = tokio::fs::metadata(&path)
        .await
        .map(|metadata| slug != "none" && metadata.len() <= TRANSPARENT_PNG.len() as u64)
        .unwrap_or(true);
    if stale_empty_fallback {
        let bytes = if slug == "none" {
            TRANSPARENT_PNG.to_vec()
        } else {
            fetch_item_sprite(&slug).await
        };
        tokio::fs::write(&path, bytes)
            .await
            .map_err(|error| WebError::Asset(error.to_string()))?;
    }
    let bytes = tokio::fs::read(path)
        .await
        .map_err(|error| WebError::Asset(error.to_string()))?;
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("image/png"));
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=604800"),
    );
    Ok((headers, bytes))
}

async fn fetch_item_sprite(slug: &str) -> Vec<u8> {
    let Ok(response) = reqwest::get(format!("https://pokeapi.co/api/v2/item/{slug}")).await else {
        return TRANSPARENT_PNG.to_vec();
    };
    if !response.status().is_success() {
        return MISSINGNO_GEN3.to_vec();
    }
    let Ok(details) = response.json::<Value>().await else {
        return MISSINGNO_GEN3.to_vec();
    };
    let Some(sprite_url) = details.pointer("/sprites/default").and_then(Value::as_str) else {
        return MISSINGNO_GEN3.to_vec();
    };
    let Ok(response) = reqwest::get(sprite_url).await else {
        return MISSINGNO_GEN3.to_vec();
    };
    match response.bytes().await {
        Ok(bytes) => bytes.to_vec(),
        Err(_) => MISSINGNO_GEN3.to_vec(),
    }
}

async fn page_damage() -> Html<String> {
    Html(ui::render(ui::Mode::Damage, None))
}

async fn page_survive() -> Html<String> {
    Html(ui::render(ui::Mode::Survive, None))
}

async fn page_sequence() -> Html<String> {
    Html(ui::render(ui::Mode::Sequence, None))
}

async fn page_ko() -> Html<String> {
    Html(ui::render(ui::Mode::Ko, None))
}

async fn page_optimize() -> Html<String> {
    Html(ui::render(ui::Mode::Optimize, None))
}

async fn api_meta(State(state): State<AppState>) -> Result<Json<api::MetadataResponse>, WebError> {
    let data = state.data.clone();
    let response = tokio::task::spawn_blocking(move || {
        let mut response = api::MetadataResponse {
            species: data.species_names().map(str::to_owned).collect(),
            regulation: data.regulation_m_b_names().map(str::to_owned).collect(),
            items: pokemon_champions_item_names().map(str::to_owned).collect(),
            abilities: data.ability_names().map(str::to_owned).collect(),
            moves: data.move_names().map(str::to_owned).collect(),
        };
        response.species.sort();
        response.species.dedup();
        response.regulation.sort();
        response.regulation.dedup();
        response.items.sort();
        response.items.dedup();
        response.abilities.sort();
        response.abilities.dedup();
        response.moves.sort();
        response.moves.dedup();
        response
    })
    .await?;
    Ok(Json(response))
}

async fn api_move_types(State(state): State<AppState>) -> Json<HashMap<String, String>> {
    let data = state.data.clone();
    let map = data
        .move_names()
        .filter_map(|name| {
            data.move_data(name)
                .ok()
                .map(|move_| (move_.name.clone(), move_.type_name.clone()))
        })
        .collect();
    Json(map)
}

async fn api_pokemon_list(State(state): State<AppState>) -> Json<Vec<String>> {
    let names = state
        .data
        .species_names()
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    Json(names.into_iter().collect())
}

async fn api_item_list(State(_state): State<AppState>) -> Json<Vec<String>> {
    let mut names = pokemon_champions_item_names()
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    names.insert("None".to_owned());
    Json(names.into_iter().collect())
}

async fn api_species_types(State(state): State<AppState>) -> Json<HashMap<String, Vec<String>>> {
    let data = state.data.clone();
    let map = data
        .species_names()
        .filter_map(|name| {
            data.species(name).ok().map(|species| {
                let mut types = species.types.clone();
                types.dedup();
                (name.to_owned(), types)
            })
        })
        .collect();
    Json(map)
}

async fn api_species_abilities() -> Json<HashMap<String, Vec<String>>> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct SpeciesAbility {
        name: String,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct SpeciesEntry {
        display_name: String,
        #[serde(default)]
        abilities: Vec<SpeciesAbility>,
    }
    #[derive(Deserialize)]
    struct ChampionsDataJson {
        species: Vec<SpeciesEntry>,
    }

    let parsed: ChampionsDataJson = serde_json::from_str(damage_calc::data::CHAMPIONS_DATA_JSON)
        .unwrap_or(ChampionsDataJson { species: vec![] });
    let map = parsed
        .species
        .into_iter()
        .map(|species| {
            let mut abilities = species
                .abilities
                .into_iter()
                .map(|ability| ability.name)
                .collect::<Vec<_>>();
            abilities.dedup();
            (species.display_name, abilities)
        })
        .collect();
    Json(map)
}

async fn api_unsupported_items(State(state): State<AppState>) -> Json<Vec<String>> {
    Json(unsupported_item_names(&state.data))
}

fn unsupported_item_names(data: &ChampionsData) -> Vec<String> {
    let _ = data;
    let mut items = pokemon_champions_item_names()
        .filter(|item| spreadlab_rs::data::parse_item(item).is_err())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    items.sort();
    items
}

fn pokemon_champions_item_names() -> impl Iterator<Item = &'static str> {
    POKEMON_CHAMPIONS_ITEMS.iter().copied()
}

async fn api_damage(
    State(state): State<AppState>,
    Json(mut request): Json<api::DamageRequest>,
) -> Result<Json<Value>, WebError> {
    let warnings = warning_messages_from_sets([&request.attacker_set, &request.defender_set]);
    normalize_damage_request(&mut request);
    call_with_data_value(state, warnings, move |data| {
        api::calculate_damage_request_with_data(&data, request)
    })
    .await
}

async fn api_survive(
    State(state): State<AppState>,
    Json(mut request): Json<api::HpDefSurvivalRequest>,
) -> Result<Json<Value>, WebError> {
    let warnings = warning_messages_from_sets([&request.attacker_set, &request.defender_set]);
    let optimize_nature = request.optimize_nature;
    let limit = request.limit;
    normalize_survive_request(&mut request);
    let mut response = call_with_data_value(state, warnings, move |data| {
        api::find_min_hp_def_survival_with_data(&data, request)
    })
    .await?;
    if optimize_nature {
        remove_neutral_nature_results(&mut response.0, limit);
    }
    Ok(response)
}

async fn api_sequence(
    State(state): State<AppState>,
    Json(mut request): Json<api::CombinedHpDefSurvivalRequest>,
) -> Result<Json<Value>, WebError> {
    let warnings = warning_messages_from_sets(
        std::iter::once(&request.defender_set)
            .chain(request.hits.iter().map(|hit| &hit.attacker_set)),
    );
    let optimize_nature = request.optimize_nature;
    let limit = request.limit;
    normalize_sequence_request(&mut request);
    let mut response = call_with_data_value(state, warnings, move |data| {
        api::find_min_combined_hp_def_survival_with_data(&data, request)
    })
    .await?;
    if optimize_nature {
        remove_neutral_nature_results(&mut response.0, limit);
    }
    Ok(response)
}

async fn api_ko(
    State(state): State<AppState>,
    Json(mut request): Json<api::OffensiveKoRequest>,
) -> Result<Json<Value>, WebError> {
    let warnings = warning_messages_from_sets([&request.attacker_set, &request.defender_set]);
    let optimize_nature = request.optimize_nature;
    let limit = request.limit;
    normalize_ko_request(&mut request);
    let mut response = call_with_data_value(state, warnings, move |data| {
        api::find_min_offensive_ko_with_data(&data, request)
    })
    .await?;
    if optimize_nature {
        remove_neutral_nature_results(&mut response.0, limit);
    }
    Ok(response)
}

async fn api_optimize_defensive(
    State(state): State<AppState>,
    Json(mut request): Json<api::OptimizeRequest>,
) -> Result<Json<Value>, WebError> {
    let warnings = warning_messages_from_sets(
        request
            .benchmarks
            .iter()
            .flat_map(|benchmark| [&benchmark.attacker_set, &benchmark.defender_set]),
    );
    normalize_optimize_request(&mut request);
    call_with_data_value(state, warnings, move |data| {
        api::run_defensive_optimization_with_data(&data, request)
    })
    .await
}

async fn api_optimize_offensive(
    State(state): State<AppState>,
    Json(mut request): Json<api::OptimizeRequest>,
) -> Result<Json<Value>, WebError> {
    let warnings = warning_messages_from_sets(
        request
            .benchmarks
            .iter()
            .flat_map(|benchmark| [&benchmark.attacker_set, &benchmark.defender_set]),
    );
    normalize_optimize_request(&mut request);
    call_with_data_value(state, warnings, move |data| {
        api::run_offensive_optimization_with_data(&data, request)
    })
    .await
}

async fn call_with_data_value<T, F>(
    state: AppState,
    warnings: Vec<String>,
    f: F,
) -> Result<Json<Value>, WebError>
where
    T: Serialize + Send + 'static,
    F: FnOnce(Arc<ChampionsData>) -> Result<T, api::ApiError> + Send + 'static,
{
    let data = state.data.clone();
    let response = tokio::task::spawn_blocking(move || f(data)).await??;
    let mut value = serde_json::to_value(response)?;
    attach_warnings(&mut value, warnings);
    Ok(Json(value))
}

fn normalize_damage_request(request: &mut api::DamageRequest) {
    request.attacker_set = normalize_showdown_set(&request.attacker_set);
    request.defender_set = normalize_showdown_set(&request.defender_set);
}

fn normalize_survive_request(request: &mut api::HpDefSurvivalRequest) {
    request.attacker_set = normalize_showdown_set(&request.attacker_set);
    request.defender_set = normalize_showdown_set(&request.defender_set);
}

fn normalize_sequence_request(request: &mut api::CombinedHpDefSurvivalRequest) {
    request.defender_set = normalize_showdown_set(&request.defender_set);
    for hit in &mut request.hits {
        hit.attacker_set = normalize_showdown_set(&hit.attacker_set);
    }
}

fn normalize_ko_request(request: &mut api::OffensiveKoRequest) {
    request.attacker_set = normalize_showdown_set(&request.attacker_set);
    request.defender_set = normalize_showdown_set(&request.defender_set);
}

fn normalize_optimize_request(request: &mut api::OptimizeRequest) {
    for benchmark in &mut request.benchmarks {
        benchmark.attacker_set = normalize_showdown_set(&benchmark.attacker_set);
        benchmark.defender_set = normalize_showdown_set(&benchmark.defender_set);
    }
}

fn attach_warnings(value: &mut Value, warnings: Vec<String>) {
    if warnings.is_empty() {
        return;
    }
    let warnings = warnings.into_iter().map(Value::String).collect::<Vec<_>>();
    match value {
        Value::Object(map) => {
            map.insert("warnings".to_owned(), Value::Array(warnings));
        }
        other => {
            *other = json!({
                "matches": other.take(),
                "warnings": warnings
            });
        }
    }
}

fn remove_neutral_nature_results(value: &mut Value, limit: usize) {
    let Some(map) = value.as_object_mut() else {
        return;
    };
    let best = map
        .get_mut("matches")
        .and_then(Value::as_array_mut)
        .map(|matches| {
            matches.retain(|entry| !entry_has_neutral_nature(entry));
            matches.truncate(default_api_limit(limit));
            for (index, entry) in matches.iter_mut().enumerate() {
                if let Some(entry) = entry.as_object_mut() {
                    entry.insert("rank".to_owned(), json!(index + 1));
                }
            }
            matches.first().cloned().unwrap_or(Value::Null)
        });
    if let Some(best) = best {
        map.insert("best".to_owned(), best);
    }
    if map
        .get("closest_miss")
        .is_some_and(entry_has_neutral_nature)
    {
        map.insert("closest_miss".to_owned(), Value::Null);
    }
}

fn entry_has_neutral_nature(entry: &Value) -> bool {
    entry
        .get("nature")
        .and_then(Value::as_str)
        .is_some_and(is_neutral_nature)
}

fn is_neutral_nature(nature: &str) -> bool {
    matches!(
        nature,
        "Hardy" | "Bashful" | "Docile" | "Quirky" | "Serious"
    )
}

fn default_api_limit(limit: usize) -> usize {
    if limit == 0 {
        10
    } else {
        limit
    }
}

fn warning_messages_from_sets<'a>(sets: impl IntoIterator<Item = &'a String>) -> Vec<String> {
    let mut warnings = sets
        .into_iter()
        .filter_map(|set| unsupported_ability(set))
        .map(|ability| {
            format!("Ability {ability} is not supported yet; it was ignored for this calculation.")
        })
        .collect::<Vec<_>>();
    warnings.sort();
    warnings.dedup();
    warnings
}

fn unsupported_ability(text: &str) -> Option<String> {
    let ability = text.lines().find_map(ability_line_value)?;
    spreadlab_rs::data::parse_ability(ability)
        .is_err()
        .then(|| ability.to_owned())
}

fn ability_line_value(line: &str) -> Option<&str> {
    let trimmed = line.trim();
    let (label, value) = trimmed.split_once(':')?;
    label
        .eq_ignore_ascii_case("Ability")
        .then(|| value.trim())
        .filter(|value| !value.is_empty())
}

async fn form_damage(State(state): State<AppState>, Form(form): Form<DamageForm>) -> Html<String> {
    render_form_result(ui::Mode::Damage, async {
        let request = form.damage_request()?;
        let response = api_damage(State(state), Json(request)).await?.0;
        serde_json::to_string_pretty(&response).map_err(Into::into)
    })
    .await
}

async fn form_survive(
    State(state): State<AppState>,
    Form(form): Form<SurviveForm>,
) -> Html<String> {
    render_form_result(ui::Mode::Survive, async {
        let request = form.survive_request()?;
        let response = api_survive(State(state), Json(request)).await?.0;
        serde_json::to_string_pretty(&response).map_err(Into::into)
    })
    .await
}

async fn form_sequence(
    State(state): State<AppState>,
    Form(form): Form<SequenceForm>,
) -> Html<String> {
    render_form_result(ui::Mode::Sequence, async {
        let request = form.sequence_request()?;
        let response = api_sequence(State(state), Json(request)).await?.0;
        serde_json::to_string_pretty(&response).map_err(Into::into)
    })
    .await
}

async fn form_ko(State(state): State<AppState>, Form(form): Form<KoForm>) -> Html<String> {
    render_form_result(ui::Mode::Ko, async {
        let request = form.ko_request()?;
        let response = api_ko(State(state), Json(request)).await?.0;
        serde_json::to_string_pretty(&response).map_err(Into::into)
    })
    .await
}

async fn form_optimize(
    State(state): State<AppState>,
    Form(form): Form<OptimizeForm>,
) -> Html<String> {
    render_form_result(ui::Mode::Optimize, async {
        let offensive = form.mode == "offensive";
        let request = form.optimize_request()?;
        let response = if offensive {
            api_optimize_offensive(State(state), Json(request)).await?.0
        } else {
            api_optimize_defensive(State(state), Json(request)).await?.0
        };
        serde_json::to_string_pretty(&response).map_err(Into::into)
    })
    .await
}

async fn render_form_result<F>(mode: ui::Mode, future: F) -> Html<String>
where
    F: std::future::Future<Output = Result<String, WebError>>,
{
    let result = match future.await {
        Ok(json) => ui::ResultBlock::Json(json),
        Err(error) => ui::ResultBlock::Error(error.to_string()),
    };
    Html(ui::render(mode, Some(result)))
}

#[derive(Debug, Deserialize)]
struct DamageForm {
    attacker_set: String,
    defender_set: String,
    move_name: String,
    #[serde(default)]
    #[serde(deserialize_with = "de_u8")]
    move_times_affected: u8,
    #[serde(flatten)]
    field: FieldForm,
}

#[derive(Debug, Deserialize)]
struct SurviveForm {
    #[serde(flatten)]
    damage: DamageForm,
    #[serde(default = "default_max_ko")]
    #[serde(deserialize_with = "de_f32")]
    max_ko_chance: f32,
    #[serde(default = "default_hp_percent")]
    #[serde(deserialize_with = "de_f32")]
    hp_percent: f32,
    #[serde(default)]
    nature: String,
    #[serde(default)]
    #[serde(deserialize_with = "de_bool")]
    optimize_nature: bool,
    #[serde(default = "default_limit")]
    #[serde(deserialize_with = "de_usize")]
    limit: usize,
}

#[derive(Debug, Deserialize)]
struct KoForm {
    #[serde(flatten)]
    damage: DamageForm,
    #[serde(default = "default_min_ko")]
    #[serde(deserialize_with = "de_f32")]
    min_ko_chance: f32,
    #[serde(default)]
    nature: String,
    #[serde(default)]
    #[serde(deserialize_with = "de_bool")]
    optimize_nature: bool,
    #[serde(default = "default_limit")]
    #[serde(deserialize_with = "de_usize")]
    limit: usize,
}

#[derive(Debug, Deserialize)]
struct SequenceForm {
    defender_set: String,
    attacker_set_1: String,
    move_name_1: String,
    #[serde(default)]
    #[serde(deserialize_with = "de_u8")]
    move_times_affected_1: u8,
    attacker_set_2: String,
    move_name_2: String,
    #[serde(default)]
    #[serde(deserialize_with = "de_u8")]
    move_times_affected_2: u8,
    #[serde(default = "default_max_ko")]
    #[serde(deserialize_with = "de_f32")]
    max_ko_chance: f32,
    #[serde(default = "default_hp_percent")]
    #[serde(deserialize_with = "de_f32")]
    hp_percent: f32,
    #[serde(default)]
    nature: String,
    #[serde(default)]
    #[serde(deserialize_with = "de_bool")]
    optimize_nature: bool,
    #[serde(default = "default_limit")]
    #[serde(deserialize_with = "de_usize")]
    limit: usize,
    #[serde(flatten)]
    field: FieldForm,
}

#[derive(Debug, Deserialize)]
struct OptimizeForm {
    mode: String,
    attacker_set: String,
    defender_set: String,
    move_name: String,
    #[serde(default)]
    #[serde(deserialize_with = "de_u8")]
    move_times_affected: u8,
    #[serde(default)]
    #[serde(deserialize_with = "de_bool")]
    full_spend: bool,
    #[serde(default = "default_limit")]
    #[serde(deserialize_with = "de_usize")]
    limit: usize,
    #[serde(default)]
    lock_hp: String,
    #[serde(default)]
    lock_attack: String,
    #[serde(default)]
    lock_defense: String,
    #[serde(default)]
    lock_special_attack: String,
    #[serde(default)]
    lock_special_defense: String,
    #[serde(default)]
    lock_speed: String,
    #[serde(flatten)]
    field: FieldForm,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct FieldForm {
    #[serde(default = "default_format")]
    format: String,
    #[serde(default = "default_none")]
    weather: String,
    #[serde(default = "default_none")]
    terrain: String,
    #[serde(default)]
    #[serde(deserialize_with = "de_bool")]
    gravity: bool,
    #[serde(default)]
    #[serde(deserialize_with = "de_bool")]
    fairy_aura: bool,
    #[serde(default)]
    #[serde(deserialize_with = "de_bool")]
    protect: bool,
    #[serde(default)]
    #[serde(deserialize_with = "de_bool")]
    helping_hand: bool,
    #[serde(default)]
    #[serde(deserialize_with = "de_bool")]
    attacker_tailwind: bool,
    #[serde(default)]
    #[serde(deserialize_with = "de_bool")]
    defender_tailwind: bool,
    #[serde(default)]
    #[serde(deserialize_with = "de_bool")]
    defender_reflect: bool,
    #[serde(default)]
    #[serde(deserialize_with = "de_bool")]
    defender_light_screen: bool,
    #[serde(default)]
    #[serde(deserialize_with = "de_bool")]
    defender_aurora_veil: bool,
    #[serde(default)]
    #[serde(deserialize_with = "de_bool")]
    defender_friend_guard: bool,
    #[serde(default)]
    #[serde(deserialize_with = "de_i8")]
    attacker_attack: i8,
    #[serde(default)]
    #[serde(deserialize_with = "de_i8")]
    attacker_defense: i8,
    #[serde(default)]
    #[serde(deserialize_with = "de_i8")]
    attacker_special_attack: i8,
    #[serde(default)]
    #[serde(deserialize_with = "de_i8")]
    attacker_special_defense: i8,
    #[serde(default)]
    #[serde(deserialize_with = "de_i8")]
    attacker_speed: i8,
    #[serde(default)]
    #[serde(deserialize_with = "de_i8")]
    defender_attack: i8,
    #[serde(default)]
    #[serde(deserialize_with = "de_i8")]
    defender_defense: i8,
    #[serde(default)]
    #[serde(deserialize_with = "de_i8")]
    defender_special_attack: i8,
    #[serde(default)]
    #[serde(deserialize_with = "de_i8")]
    defender_special_defense: i8,
    #[serde(default)]
    #[serde(deserialize_with = "de_i8")]
    defender_speed: i8,
}

impl DamageForm {
    fn damage_request(self) -> Result<api::DamageRequest, WebError> {
        from_value(json!({
            "attacker_set": normalize_showdown_set(&self.attacker_set),
            "defender_set": normalize_showdown_set(&self.defender_set),
            "move_name": self.move_name,
            "move_times_affected": self.move_times_affected,
            "field": self.field.value()
        }))
    }
}

impl SurviveForm {
    fn survive_request(self) -> Result<api::HpDefSurvivalRequest, WebError> {
        from_value(json!({
            "attacker_set": normalize_showdown_set(&self.damage.attacker_set),
            "defender_set": normalize_showdown_set(&self.damage.defender_set),
            "move_name": self.damage.move_name,
            "move_times_affected": self.damage.move_times_affected,
            "field": self.damage.field.value(),
            "max_ko_chance": ko_rolls_to_chance(self.max_ko_chance),
            "hp_percent": self.hp_percent,
            "nature": optional_string(self.nature),
            "optimize_nature": self.optimize_nature,
            "limit": self.limit
        }))
    }
}

impl KoForm {
    fn ko_request(self) -> Result<api::OffensiveKoRequest, WebError> {
        from_value(json!({
            "attacker_set": normalize_showdown_set(&self.damage.attacker_set),
            "defender_set": normalize_showdown_set(&self.damage.defender_set),
            "move_name": self.damage.move_name,
            "move_times_affected": self.damage.move_times_affected,
            "field": self.damage.field.value(),
            "min_ko_chance": ko_rolls_to_chance(self.min_ko_chance),
            "nature": optional_string(self.nature),
            "optimize_nature": self.optimize_nature,
            "limit": self.limit
        }))
    }
}

impl SequenceForm {
    fn sequence_request(self) -> Result<api::CombinedHpDefSurvivalRequest, WebError> {
        let field = self.field.value();
        from_value(json!({
            "defender_set": normalize_showdown_set(&self.defender_set),
            "hits": [
                {
                    "attacker_set": normalize_showdown_set(&self.attacker_set_1),
                    "move_name": self.move_name_1,
                    "move_times_affected": self.move_times_affected_1,
                    "field": field
                },
                {
                    "attacker_set": normalize_showdown_set(&self.attacker_set_2),
                    "move_name": self.move_name_2,
                    "move_times_affected": self.move_times_affected_2,
                    "field": field
                }
            ],
            "max_ko_chance": ko_rolls_to_chance(self.max_ko_chance),
            "hp_percent": self.hp_percent,
            "nature": optional_string(self.nature),
            "optimize_nature": self.optimize_nature,
            "limit": self.limit
        }))
    }
}

impl OptimizeForm {
    fn optimize_request(self) -> Result<api::OptimizeRequest, WebError> {
        from_value(json!({
            "benchmarks": [{
                "attacker_set": normalize_showdown_set(&self.attacker_set),
                "defender_set": normalize_showdown_set(&self.defender_set),
                "move_name": self.move_name,
                "move_times_affected": self.move_times_affected,
                "field": self.field.value()
            }],
            "full_spend": self.full_spend,
            "locked": {
                "hp": optional_u16(self.lock_hp),
                "attack": optional_u16(self.lock_attack),
                "defense": optional_u16(self.lock_defense),
                "special_attack": optional_u16(self.lock_special_attack),
                "special_defense": optional_u16(self.lock_special_defense),
                "speed": optional_u16(self.lock_speed)
            },
            "limit": self.limit
        }))
    }
}

impl FieldForm {
    fn value(self) -> Value {
        json!({
            "format": self.format,
            "weather": self.weather,
            "terrain": self.terrain,
            "gravity": self.gravity,
            "fairy_aura": self.fairy_aura,
            "protect": self.protect,
            "helping_hand": self.helping_hand,
            "attacker_tailwind": self.attacker_tailwind,
            "defender_tailwind": self.defender_tailwind,
            "defender_reflect": self.defender_reflect,
            "defender_light_screen": self.defender_light_screen,
            "defender_aurora_veil": self.defender_aurora_veil,
            "defender_friend_guard": self.defender_friend_guard,
            "attacker_boosts": {
                "attack": self.attacker_attack,
                "defense": self.attacker_defense,
                "special_attack": self.attacker_special_attack,
                "special_defense": self.attacker_special_defense,
                "speed": self.attacker_speed
            },
            "defender_boosts": {
                "attack": self.defender_attack,
                "defense": self.defender_defense,
                "special_attack": self.defender_special_attack,
                "special_defense": self.defender_special_defense,
                "speed": self.defender_speed
            }
        })
    }
}

fn from_value<T: DeserializeOwned>(value: Value) -> Result<T, WebError> {
    serde_json::from_value(value).map_err(Into::into)
}

fn optional_string(value: String) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_owned())
}

fn optional_u16(value: String) -> Option<u16> {
    value.trim().parse().ok()
}

fn ko_rolls_to_chance(value: f32) -> f32 {
    if value > 1.0 {
        (value / 16.0).clamp(0.0, 1.0)
    } else {
        value.clamp(0.0, 1.0)
    }
}

fn normalize_showdown_set(text: &str) -> String {
    let mut training_parts = Vec::new();
    let mut insert_at = None;
    let mut ability_insert_at = None;
    let mut has_ability_on = false;
    let mut is_mega_floette = false;
    let mut out = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.to_ascii_lowercase().starts_with("ability:") {
            ability_insert_at = Some(out.len() + 1);
            if let Some(ability) = ability_line_value(trimmed) {
                if spreadlab_rs::data::parse_ability(ability).is_err() {
                    continue;
                }
            }
        }
        if trimmed.to_ascii_lowercase().starts_with("ability on:") {
            has_ability_on = true;
        }
        if let Some(payload) = trimmed
            .strip_prefix("EVs:")
            .or_else(|| trimmed.strip_prefix("evs:"))
            .or_else(|| trimmed.strip_prefix("SPs:"))
            .or_else(|| trimmed.strip_prefix("sps:"))
        {
            if insert_at.is_none() {
                insert_at = Some(out.len());
            }
            training_parts.extend(
                payload
                    .split('/')
                    .map(str::trim)
                    .filter(|part| !part.is_empty())
                    .map(str::to_owned),
            );
            continue;
        }
        let mut line = line.to_owned();
        if trimmed.to_ascii_lowercase().starts_with("floette-mega") {
            is_mega_floette = true;
            line = line.replacen("Floette-Mega", "Mega Floette", 1).replacen(
                "floette-mega",
                "Mega Floette",
                1,
            );
        } else if trimmed.to_ascii_lowercase().starts_with("mega floette") {
            is_mega_floette = true;
        }
        if is_mega_floette && trimmed.eq_ignore_ascii_case("Ability: Flower Veil") {
            line = "Ability: Fairy Aura".to_owned();
        }
        if trimmed.to_ascii_lowercase().contains("@ floettite") {
            out.push(line.replace("@ Floettite", "").replace("@ floettite", ""));
        } else if let Some((name, item)) = line.split_once('@') {
            if spreadlab_rs::data::parse_item(item.trim()).is_err()
                && is_damage_neutral_unsupported_item(item.trim())
            {
                out.push(name.trim_end().to_owned());
            } else {
                out.push(line);
            }
        } else {
            out.push(line);
        }
    }
    if ability_insert_at.is_some() && !has_ability_on {
        out.insert(
            ability_insert_at.unwrap().min(out.len()),
            "Ability On: true".to_owned(),
        );
    }
    if !training_parts.is_empty() {
        let line = format!("SPs: {}", training_parts.join(" / "));
        out.insert(insert_at.unwrap_or(1).min(out.len()), line);
    }
    out.join("\n")
}

fn is_damage_neutral_unsupported_item(item: &str) -> bool {
    matches!(
        normalize_item_key(item).as_str(),
        "heavydutyboots"
            | "shedshell"
            | "terrainextender"
            | "damprock"
            | "heatrock"
            | "icyrock"
            | "smoothrock"
            | "lightclay"
    )
}

fn normalize_item_key(item: &str) -> String {
    item.chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn de_u8<'de, D>(deserializer: D) -> Result<u8, D::Error>
where
    D: Deserializer<'de>,
{
    de_parse(deserializer)
}

fn de_i8<'de, D>(deserializer: D) -> Result<i8, D::Error>
where
    D: Deserializer<'de>,
{
    de_parse(deserializer)
}

fn de_f32<'de, D>(deserializer: D) -> Result<f32, D::Error>
where
    D: Deserializer<'de>,
{
    de_parse(deserializer)
}

fn de_usize<'de, D>(deserializer: D) -> Result<usize, D::Error>
where
    D: Deserializer<'de>,
{
    de_parse(deserializer)
}

fn de_bool<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    Ok(matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "true" | "on" | "1" | "yes"
    ))
}

fn de_parse<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    let value = String::deserialize(deserializer)?;
    value.trim().parse().map_err(de::Error::custom)
}

fn default_format() -> String {
    "Doubles".to_owned()
}

fn default_none() -> String {
    "None".to_owned()
}

fn default_max_ko() -> f32 {
    0.125
}

fn default_min_ko() -> f32 {
    1.0
}

fn default_hp_percent() -> f32 {
    100.0
}

fn default_limit() -> usize {
    10
}

fn showdown_sprite_slug(name: &str) -> String {
    let slug = name
        .trim()
        .to_ascii_lowercase()
        .replace(" (hisuian)", "-hisui")
        .replace(" (alolan)", "-alola")
        .replace(" (galarian)", "-galar")
        .replace(" (paldean combat breed)", "-paldea-combat-breed")
        .replace(" (paldean blaze breed)", "-paldea-blaze-breed")
        .replace(" (paldean aqua breed)", "-paldea-aqua-breed")
        .replace(['(', ')', '.', '\'', '♀', '♂'], "")
        .replace(" forme", "")
        .replace(" form", "")
        .replace(' ', "-");

    if let Some(rest) = slug.strip_prefix("mega-") {
        return showdown_prefixed_mega_slug(rest);
    }
    if let Some((species, form)) = slug.split_once("-mega-") {
        return showdown_formed_mega_slug(species, form);
    }
    slug
}

fn showdown_prefixed_mega_slug(rest: &str) -> String {
    let Some((species, form)) = rest.rsplit_once('-') else {
        return format!("{rest}-mega");
    };
    if matches!(form, "x" | "y" | "z") {
        format!("{species}-mega{form}")
    } else if matches!(
        form,
        "m" | "f" | "curly" | "droopy" | "stretchy" | "original"
    ) {
        format!("{species}-{form}mega")
    } else {
        format!("{rest}-mega")
    }
}

fn showdown_formed_mega_slug(species: &str, form: &str) -> String {
    if matches!(form, "x" | "y" | "z") {
        format!("{species}-mega{form}")
    } else {
        format!("{species}-{form}mega")
    }
}

fn sprite_slug(name: &str) -> String {
    name.to_ascii_lowercase()
        .replace("mega ", "")
        .replace(" (hisuian)", "-hisui")
        .replace(" (alolan)", "-alola")
        .replace(" (galarian)", "-galar")
        .replace(" (paldean combat breed)", "-paldea-combat-breed")
        .replace(" (paldean blaze breed)", "-paldea-blaze-breed")
        .replace(" (paldean aqua breed)", "-paldea-aqua-breed")
        .replace(['(', ')', '.', '\'', '♀', '♂'], "")
        .replace(" forme", "")
        .replace(" form", "")
        .replace(' ', "-")
}

fn pokeapi_slug(slug: &str) -> String {
    match slug {
        "mega-floette" | "floette-mega" => "floette".to_owned(),
        other => other.to_owned(),
    }
}

fn item_slug(name: &str) -> String {
    let trimmed = name.trim();
    if trimmed.eq_ignore_ascii_case("none") || trimmed.is_empty() {
        return "none".to_owned();
    }
    trimmed
        .to_ascii_lowercase()
        .replace(['\'', '.'], "")
        .replace(' ', "-")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn showdown_sprite_slugs_cover_new_and_formed_megas() {
        assert_eq!(showdown_sprite_slug("Mega Eelektross"), "eelektross-mega");
        assert_eq!(showdown_sprite_slug("Floette-Mega"), "floette-mega");
        assert_eq!(showdown_sprite_slug("Mega Raichu X"), "raichu-megax");
        assert_eq!(showdown_sprite_slug("Garchomp-Mega-Z"), "garchomp-megaz");
        assert_eq!(
            showdown_sprite_slug("Tatsugiri-Mega-Curly"),
            "tatsugiri-curlymega"
        );
    }

    #[test]
    fn pokemon_champions_item_catalog_contains_recent_items() {
        let items = pokemon_champions_item_names().collect::<BTreeSet<_>>();

        for item in ["Choice Scarf", "Garchompite", "Barbaracleite"] {
            assert!(
                items.contains(item),
                "{item} missing from item selector catalog"
            );
        }
    }

    #[test]
    fn pokemon_champions_item_catalog_has_no_duplicates() {
        let item_count = pokemon_champions_item_names().count();
        let unique_count = pokemon_champions_item_names()
            .collect::<BTreeSet<_>>()
            .len();

        assert_eq!(item_count, unique_count);
    }

    #[test]
    fn pokeapi_resource_names_are_display_names() {
        assert_eq!(
            pokeapi_resource_display_name("charizard-mega-x"),
            "Mega Charizard X"
        );
        assert_eq!(pokeapi_resource_display_name("mr-mime"), "Mr-Mime");
        assert_eq!(
            pokeapi_resource_display_name("giratina-origin"),
            "Giratina-Origin"
        );
    }

    #[test]
    fn champions_item_catalog_is_supported_by_spreadlab_parser() {
        let data = ChampionsData::load().expect("Champions data loads");
        assert_eq!(unsupported_item_names(&data), Vec::<String>::new());
    }

    #[test]
    fn newly_supported_neutral_items_are_preserved_for_core_parse() {
        let normalized = normalize_showdown_set(
            "Kingambit @ Shed Shell\nAbility: Defiant\nAdamant Nature\n- Iron Head",
        );
        assert!(normalized.starts_with("Kingambit @ Shed Shell\n"));
    }

    #[test]
    fn battle_relevant_items_are_not_stripped() {
        for item in [
            "Focus Sash",
            "Leftovers",
            "Black Sludge",
            "Safety Goggles",
            "Covert Cloak",
            "Rocky Helmet",
            "Eject Button",
            "Eject Pack",
            "Red Card",
        ] {
            let normalized = normalize_showdown_set(&format!(
                "Kingambit @ {item}\nAbility: Defiant\nAdamant Nature\n- Iron Head"
            ));
            assert!(normalized.contains(item));
        }
    }

    #[test]
    fn unknown_items_still_reach_core_parse_and_fail_loudly() {
        let normalized = normalize_showdown_set(
            "Kingambit @ Definitely Damage Relevant\nAbility: Defiant\nAdamant Nature\n- Iron Head",
        );
        assert!(normalized.contains("@ Definitely Damage Relevant"));
    }

    #[test]
    fn unsupported_abilities_are_warned_and_removed_before_core_parse() {
        let raw = "Garchomp @ Focus Sash\nAbility: Rough Skin\nJolly Nature\n- Earthquake";
        let normalized = normalize_showdown_set(raw);
        let warnings = warning_messages_from_sets([&raw.to_owned()]);

        assert!(!normalized.contains("Ability: Rough Skin"));
        assert_eq!(
            warnings,
            vec![
                "Ability Rough Skin is not supported yet; it was ignored for this calculation."
                    .to_owned()
            ]
        );
    }

    #[test]
    fn optimized_nature_results_drop_neutral_natures() {
        let mut value = json!({
            "best": { "rank": 1, "nature": "Hardy" },
            "matches": [
                { "rank": 1, "nature": "Hardy" },
                { "rank": 2, "nature": "Bold" },
                { "rank": 3, "nature": "Serious" },
                { "rank": 4, "nature": "Calm" }
            ],
            "closest_miss": { "rank": 1, "nature": "Quirky" }
        });

        remove_neutral_nature_results(&mut value, 10);

        assert_eq!(value["best"]["nature"], "Bold");
        assert_eq!(value["matches"][0]["rank"], 1);
        assert_eq!(value["matches"][0]["nature"], "Bold");
        assert_eq!(value["matches"][1]["rank"], 2);
        assert_eq!(value["matches"][1]["nature"], "Calm");
        assert!(value["closest_miss"].is_null());
    }

    #[test]
    fn defensive_optimizer_allows_attacker_sps_over_total_cap() {
        let data = ChampionsData::load().expect("Champions data loads");
        let response = api::find_min_hp_def_survival_with_data(
            &data,
            api::HpDefSurvivalRequest {
                attacker_set: normalize_showdown_set(
                    "Kingambit @ Black Glasses\n\
                     Ability: Defiant\n\
                     SPs: 32 HP / 32 Atk / 32 Def\n\
                     Adamant Nature\n\
                     - Iron Head",
                ),
                defender_set: normalize_showdown_set(
                    "Mega Floette\n\
                     Ability: Fairy Aura\n\
                     Timid Nature\n\
                     - Protect",
                ),
                move_name: "Iron Head".to_owned(),
                max_ko_chance: 0.125,
                hp_percent: Some(100.0),
                nature: None,
                optimize_nature: true,
                limit: 1,
                move_times_affected: 0,
                critical: false,
                field: None,
            },
        )
        .expect("defensive optimization accepts over-cap attacker SP total");

        assert!(response.best.is_some());
    }

    #[test]
    fn offensive_optimizer_allows_defender_sps_over_total_cap() {
        let data = ChampionsData::load().expect("Champions data loads");
        let response = api::find_min_offensive_ko_with_data(
            &data,
            api::OffensiveKoRequest {
                attacker_set: normalize_showdown_set(
                    "Kingambit @ Black Glasses\n\
                     Ability: Defiant\n\
                     Adamant Nature\n\
                     - Iron Head",
                ),
                defender_set: normalize_showdown_set(
                    "Mega Floette\n\
                     Ability: Fairy Aura\n\
                     SPs: 32 HP / 32 Def / 32 SpD\n\
                     Timid Nature\n\
                     - Protect",
                ),
                move_name: "Iron Head".to_owned(),
                min_ko_chance: 0.0,
                nature: None,
                optimize_nature: true,
                limit: 1,
                move_times_affected: 0,
                critical: false,
                field: None,
            },
        )
        .expect("offensive optimization accepts over-cap defender SP total");

        assert!(response.best.is_some());
    }
}
