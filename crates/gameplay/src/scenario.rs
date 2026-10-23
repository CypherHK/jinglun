use crate::{Error, Result, Rules};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub const GENERATOR_VERSION: &str = "0.6.0";
pub const SCENARIO_VERSION: &str = "jinglun-040-foundation-1";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    pub version: String,
    pub countries: BTreeMap<String, Vec<u32>>,
    pub capital: u32,
    pub candidates: Vec<u32>,
}
impl Scenario {
    pub fn fixed() -> Self {
        Self {
            version: SCENARIO_VERSION.into(),
            countries: BTreeMap::from([
                ("player".into(), vec![8, 9, 26, 29, 38, 39, 63, 74, 83, 91]),
                (
                    "resource".into(),
                    vec![
                        2, 11, 12, 16, 18, 30, 43, 58, 60, 62, 65, 66, 69, 71, 77, 79, 87, 97, 99,
                    ],
                ),
                (
                    "maritime".into(),
                    vec![
                        3, 7, 22, 34, 41, 44, 46, 49, 54, 64, 67, 73, 75, 76, 82, 84, 85,
                    ],
                ),
            ]),
            capital: 39,
            candidates: vec![8, 29],
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct Cell {
    pub id: u32,
    pub is_land: bool,
    pub is_coastal: bool,
    pub neighbors: Vec<u32>,
    pub water_source_ids: Vec<u32>,
    pub construction: Construction,
}
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct Construction {
    pub buildable_area_km2: f64,
    pub cost_multiplier: Option<f64>,
}
#[derive(Deserialize)]
struct Map {
    generator_version: String,
    config: jinglun_world::MapConfig,
    cells: Vec<Cell>,
    edges: Vec<Edge>,
    environment: Environment,
}
#[derive(Deserialize)]
struct Edge {
    id: u32,
    a: u32,
    b: u32,
}
#[derive(Deserialize)]
struct Environment {
    water_sources: Vec<Source>,
    scale: Scale,
}
#[derive(Deserialize)]
struct Scale {
    water_unit_m3: f64,
}
#[derive(Deserialize)]
struct Source {
    id: u32,
    months: Vec<SourceMonth>,
}
#[derive(Deserialize)]
struct SourceMonth {
    freshwater_ceiling: f64,
}

#[derive(Clone, Debug)]
pub(crate) struct Projection {
    pub cells: BTreeMap<u32, Cell>,
    pub owners: BTreeMap<u32, String>,
    pub land: BTreeMap<u32, u64>,
    pub water: BTreeMap<u32, u64>,
    pub source_m3: BTreeMap<u32, f64>,
    pub components: BTreeMap<String, usize>,
}

pub(crate) fn generated() -> Result<Value> {
    let map = jinglun_world::generate(jinglun_world::MapConfig {
        seed: "jinglun-040".into(),
        cell_count: 100,
        relaxations: 2,
        land_ratio: 0.45,
    })
    .map_err(|_| Error::Invalid("map generation"))?;
    Ok(serde_json::to_value(map)?)
}

pub(crate) fn project(value: &Value, scenario: &Scenario, rules: &Rules) -> Result<Projection> {
    if scenario != &Scenario::fixed() {
        return Err(Error::Invalid("scenario version or boundaries"));
    }
    let map: Map = serde_json::from_value(value.clone())?;
    if map.generator_version != GENERATOR_VERSION
        || map.config
            != (jinglun_world::MapConfig {
                seed: "jinglun-040".into(),
                cell_count: 100,
                relaxations: 2,
                land_ratio: 0.45,
            })
    {
        return Err(Error::Invalid("generator version or config"));
    }
    let mut cells = BTreeMap::new();
    for cell in map.cells {
        if cells.insert(cell.id, cell).is_some() {
            return Err(Error::Invalid("duplicate cell"));
        }
    }
    if cells.len() != 100 || cells.keys().copied().ne(0..100) {
        return Err(Error::Invalid("100 unique raw cells required"));
    }
    let mut water = BTreeMap::new();
    let mut source_m3 = BTreeMap::new();
    finite_nonnegative(map.environment.scale.water_unit_m3)?;
    if map.environment.scale.water_unit_m3 == 0.0 {
        return Err(Error::Invalid("water scale"));
    }
    for source in map.environment.water_sources {
        if source.months.len() != 12 {
            return Err(Error::Invalid("12 source months required"));
        }
        let mut min = f64::INFINITY;
        for month in source.months {
            finite_nonnegative(month.freshwater_ceiling)?;
            min = min.min(month.freshwater_ceiling);
        }
        let m3 = min * map.environment.scale.water_unit_m3;
        finite_nonnegative(m3)?;
        let slots = quota(
            m3,
            rules.water_m3_per_slot,
            rules.water_min,
            rules.water_max,
        )?;
        if water.insert(source.id, slots).is_some() {
            return Err(Error::Invalid("duplicate source"));
        }
        source_m3.insert(source.id, m3);
    }
    let mut owners = BTreeMap::new();
    for (country, ids) in &scenario.countries {
        for id in ids {
            if !cells.get(id).is_some_and(|c| c.is_land)
                || owners.insert(*id, country.clone()).is_some()
            {
                return Err(Error::Invalid("country land coverage"));
            }
        }
    }
    if owners.len() != 46 || cells.values().filter(|c| c.is_land).count() != 46 {
        return Err(Error::Invalid("46 land cells required"));
    }
    let mut land = BTreeMap::new();
    for cell in cells.values() {
        if cell
            .neighbors
            .iter()
            .any(|id| !cells.contains_key(id) || *id == cell.id)
            || cell
                .neighbors
                .iter()
                .copied()
                .collect::<BTreeSet<_>>()
                .len()
                != cell.neighbors.len()
            || cell
                .neighbors
                .iter()
                .any(|id| !cells[id].neighbors.contains(&cell.id))
            || cell
                .water_source_ids
                .iter()
                .any(|id| !water.contains_key(id))
            || cell
                .water_source_ids
                .iter()
                .copied()
                .collect::<BTreeSet<_>>()
                .len()
                != cell.water_source_ids.len()
        {
            return Err(Error::Invalid("cell references"));
        }
        finite_nonnegative(cell.construction.buildable_area_km2)?;
        if let Some(cost) = cell.construction.cost_multiplier {
            positive(cost)?;
        }
        if cell.is_land {
            if !owners.contains_key(&cell.id) {
                return Err(Error::Invalid("unassigned land"));
            }
            land.insert(
                cell.id,
                quota(
                    cell.construction.buildable_area_km2,
                    rules.land_km2_per_slot,
                    rules.land_min,
                    rules.land_max,
                )?,
            );
        }
    }
    let mut edges = BTreeSet::new();
    for e in map.edges {
        if !edges.insert(e.id)
            || !cells.contains_key(&e.a)
            || !cells.contains_key(&e.b)
            || !cells[&e.a].neighbors.contains(&e.b)
        {
            return Err(Error::Invalid("edge reference"));
        }
    }
    if !cells[&scenario.capital].is_coastal
        || scenario
            .candidates
            .iter()
            .any(|id| cells[id].is_coastal || owners[id] != "player")
    {
        return Err(Error::Invalid("capital or candidate geography"));
    }
    let components = scenario
        .countries
        .iter()
        .map(|(country, ids)| {
            let mut unseen: BTreeSet<_> = ids.iter().copied().collect();
            let mut count = 0;
            while let Some(id) = unseen.pop_first() {
                count += 1;
                let mut stack = vec![id];
                while let Some(id) = stack.pop() {
                    for neighbor in &cells[&id].neighbors {
                        if unseen.remove(neighbor) {
                            stack.push(*neighbor);
                        }
                    }
                }
            }
            (country.clone(), count)
        })
        .collect();
    Ok(Projection {
        cells,
        owners,
        land,
        water,
        source_m3,
        components,
    })
}

pub(crate) fn finite_nonnegative(n: f64) -> Result<()> {
    if !n.is_finite() || n < 0.0 {
        Err(Error::Invalid("finite nonnegative number required"))
    } else {
        Ok(())
    }
}
pub(crate) fn positive(n: f64) -> Result<()> {
    finite_nonnegative(n)?;
    if n == 0.0 {
        Err(Error::Invalid("positive coefficient required"))
    } else {
        Ok(())
    }
}
fn quota(n: f64, per: f64, min: u64, max: u64) -> Result<u64> {
    if n == 0.0 {
        return Ok(0);
    }
    let raw = (n / per).floor();
    if !raw.is_finite() || raw >= u64::MAX as f64 {
        return Err(Error::Overflow);
    }
    Ok((raw as u64).clamp(min, max))
}
