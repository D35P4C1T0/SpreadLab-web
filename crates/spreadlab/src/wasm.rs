use crate::api::{
    calculate_damage_request, find_min_combined_hp_def_survival, find_min_hp_def_survival,
    find_min_offensive_ko, load_metadata, run_defensive_optimization, run_offensive_optimization,
    CombinedHpDefSurvivalRequest, DamageRequest, HpDefSurvivalRequest, OffensiveKoRequest,
    OptimizeRequest,
};
use serde::de::DeserializeOwned;
use serde::Serialize;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = findMinSurvival)]
pub fn find_min_survival_json(request_json: &str) -> Result<String, JsValue> {
    to_json(crate::api::find_min_survival(from_json::<
        crate::api::SurvivalRequest,
    >(request_json)?))
}

#[wasm_bindgen(js_name = loadMetadata)]
pub fn load_metadata_json() -> Result<String, JsValue> {
    to_json(load_metadata())
}

#[wasm_bindgen(js_name = calculateDamage)]
pub fn calculate_damage_json(request_json: &str) -> Result<String, JsValue> {
    let request = from_json::<DamageRequest>(request_json)?;
    to_json(calculate_damage_request(request))
}

#[wasm_bindgen(js_name = calculateAllMoves)]
pub fn calculate_all_moves_json(request_json: &str) -> Result<String, JsValue> {
    let request = from_json::<crate::api::AllMovesRequest>(request_json)?;
    to_json(crate::api::calculate_all_moves_request(request))
}

#[wasm_bindgen(js_name = findMinHpDefSurvival)]
pub fn find_min_hp_def_survival_json(request_json: &str) -> Result<String, JsValue> {
    let request = from_json::<HpDefSurvivalRequest>(request_json)?;
    to_json(find_min_hp_def_survival(request))
}

#[wasm_bindgen(js_name = findMinCombinedHpDefSurvival)]
pub fn find_min_combined_hp_def_survival_json(request_json: &str) -> Result<String, JsValue> {
    let request = from_json::<CombinedHpDefSurvivalRequest>(request_json)?;
    to_json(find_min_combined_hp_def_survival(request))
}

#[wasm_bindgen(js_name = findMinOffensiveKo)]
pub fn find_min_offensive_ko_json(request_json: &str) -> Result<String, JsValue> {
    let request = from_json::<OffensiveKoRequest>(request_json)?;
    to_json(find_min_offensive_ko(request))
}

#[wasm_bindgen(js_name = runDefensiveOptimization)]
pub fn run_defensive_optimization_json(request_json: &str) -> Result<String, JsValue> {
    let request = from_json::<OptimizeRequest>(request_json)?;
    to_json(run_defensive_optimization(request))
}

#[wasm_bindgen(js_name = runOffensiveOptimization)]
pub fn run_offensive_optimization_json(request_json: &str) -> Result<String, JsValue> {
    let request = from_json::<OptimizeRequest>(request_json)?;
    to_json(run_offensive_optimization(request))
}

fn from_json<T>(json: &str) -> Result<T, JsValue>
where
    T: DeserializeOwned,
{
    serde_json::from_str(json).map_err(|error| JsValue::from_str(&format!("invalid JSON: {error}")))
}

fn to_json<T, E>(result: Result<T, E>) -> Result<String, JsValue>
where
    T: Serialize,
    E: std::fmt::Display,
{
    let value = result.map_err(|error| JsValue::from_str(&error.to_string()))?;
    serde_json::to_string(&value).map_err(|error| JsValue::from_str(&error.to_string()))
}
