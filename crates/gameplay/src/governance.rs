use crate::*;
use std::collections::{BTreeMap, BTreeSet};

/// One currency unit is 100,000,000 ledger units. Each conversion rounds to the
/// nearest unit, with an absolute error of at most 5e-9 currency units.
pub const GOVERNANCE_RULES_VERSION: &str = "governance-rules-5";
pub const MONEY_SCALE: u64 = 100_000_000;
pub fn money(value: f64) -> Result<u64> {
    scenario::finite_nonnegative(value)?;
    if value == 0.0 {
        return Ok(0);
    }
    let bits = value.to_bits();
    let exponent = ((bits >> 52) & 0x7ff) as i32;
    let significand = (bits & ((1u64 << 52) - 1)) | if exponent == 0 { 0 } else { 1u64 << 52 };
    let binary_shift = if exponent == 0 {
        -1074
    } else {
        exponent - 1023 - 52
    };
    let numerator = significand as u128 * MONEY_SCALE as u128;
    let rounded = if binary_shift >= 0 {
        let shift = binary_shift as u32;
        if shift >= 64 || numerator > (u64::MAX >> shift) as u128 {
            return Err(Error::Overflow);
        }
        numerator << shift
    } else {
        let shift = (-binary_shift) as u32;
        if shift >= 128 {
            0
        } else {
            let denominator = 1u128 << shift;
            (numerator + denominator / 2) / denominator
        }
    };
    u64::try_from(rounded).map_err(|_| Error::Overflow)
}

// Proportional allocation floors to the common minor unit; stable keys receive the remainder.
fn apportion<K: Ord + Clone>(
    demand: &BTreeMap<K, u64>,
    available: u64,
) -> Result<BTreeMap<K, u64>> {
    let total = demand.values().try_fold(0u64, |sum, value| {
        sum.checked_add(*value).ok_or(Error::Overflow)
    })?;
    let cap = available.min(total);
    let mut result = BTreeMap::new();
    if total == 0 {
        return Ok(result);
    }
    for (key, need) in demand {
        result.insert(
            key.clone(),
            (*need as u128 * cap as u128 / total as u128) as u64,
        );
    }
    let mut remainder = cap - result.values().sum::<u64>();
    for (key, quantity) in &mut result {
        if remainder > 0 && *quantity < demand[key] {
            *quantity += 1;
            remainder -= 1;
        }
    }
    Ok(result)
}
pub fn currency(value: u64) -> f64 {
    value as f64 / MONEY_SCALE as f64
}
fn document() -> Result<&'static Value> {
    static DOCUMENT: std::sync::OnceLock<Value> = std::sync::OnceLock::new();
    if let Some(document) = DOCUMENT.get() {
        return Ok(document);
    }
    let confirmed = crate::initial_values::confirmed_document()?;
    let _ = DOCUMENT.set(confirmed);
    Ok(DOCUMENT.get().unwrap())
}
fn number(doc: &Value, pointer: &str) -> Result<f64> {
    crate::initial_values::number(doc, pointer)
}
fn param(doc: &Value, name: &str) -> Result<f64> {
    number(doc, &format!("/parameters/{name}/value"))
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Group {
    A,
    B,
}
impl Group {
    pub fn account(self) -> &'static str {
        match self {
            Self::A => "group_a",
            Self::B => "group_b",
        }
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum PublicWork {
    Housing,
    Clinic,
    Vocational,
    Road,
    Power,
    Water,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Service {
    Health,
    Safety,
    Education,
    Maintenance,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Institution {
    ClinicalCoverage,
    LaborProtection,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum RenewalChoice {
    WaterPriority,
    EnergyLongTerm,
    Decline,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub enum GovernanceCommand {
    SelectCandidate {
        site: u32,
    },
    OpenRegion {
        site: u32,
    },
    SetSubsidy {
        site: u32,
        cap: u64,
    },
    BuildPublic {
        id: String,
        site: u32,
        kind: PublicWork,
        road_edge: Option<u32>,
    },
    AcceptIntent {
        id: String,
        deadline: u32,
    },
    RejectIntent {
        id: String,
    },
    SignCooperation,
    SignEnergy,
    RenewAid {
        choice: RenewalChoice,
    },
    SetSecurity {
        monthly: u64,
    },
    SetServiceBudget {
        kind: Service,
        monthly: u64,
    },
    SetInstitution {
        kind: Institution,
        enabled: bool,
    },
    SetSpendingPriority {
        order: Vec<Service>,
    },
    PauseProject {
        id: String,
        paused: bool,
    },
    Retrofit {
        factory: String,
    },
    ExpediteProject {
        id: String,
        additional_months: u32,
    },
}

/// These rule coefficients are a calibration proposal, not changes to the
/// confirmed initial document. They remain visible in every formal save.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GovernanceCalibration {
    pub status: String,
    pub owner: String,
    pub review_date: String,
    pub initial_skilled_idle_share: f64,
    pub administrative_capacity: usize,
    pub housing_workers: u32,
    pub clinic_people: u32,
    pub trained_workers_per_quarter: u32,
    pub cooperation_training_per_quarter: u32,
    pub cooperation_export_orders_per_month: u32,
    pub cooperation_delay_months: u32,
    pub road_output_capacity: u32,
    pub power_output_capacity: u32,
    pub factory_skill_workers: u32,
    pub maximum_payback_months: f64,
    pub domestic_migration_per_month: u32,
    pub migration_source_population_share: f64,
    pub cooperation_monthly_fee: u64,
    pub foreign_buyer_monthly_budget: u64,
    pub public_work_monthly_maintenance: u64,
    pub security_recovery_budget_ratio: f64,
    pub security_decline_budget_ratio: f64,
    pub ordinary_energy_import_limit: f64,
    pub water_priority_fraction: f64,
    pub water_m3_per_slot: f64,
    pub water_slot_min: u64,
    pub water_slot_max: u64,
    pub capital_output_ratio: f64,
    pub protected_treasury: u64,
    pub clinic_base_cost: u64,
    pub power_base_cost: u64,
    pub water_base_cost: u64,
    pub retrofit_cost: u64,
    pub retrofit_output_increase: f64,
    pub labor_protection_wage_multiplier: f64,
}
impl Default for GovernanceCalibration {
    fn default() -> Self {
        Self {
            status: "pending_user_confirmation".into(),
            owner: "user".into(),
            review_date: "2026-10-25".into(),
            initial_skilled_idle_share: 0.25,
            administrative_capacity: 4,
            housing_workers: 200,
            clinic_people: 200,
            trained_workers_per_quarter: 20,
            cooperation_training_per_quarter: 10,
            cooperation_export_orders_per_month: 100,
            cooperation_delay_months: 3,
            road_output_capacity: 200,
            power_output_capacity: 200,
            factory_skill_workers: 20,
            maximum_payback_months: 48.0,
            domestic_migration_per_month: 100,
            migration_source_population_share: 0.01,
            cooperation_monthly_fee: MONEY_SCALE,
            foreign_buyer_monthly_budget: 10 * MONEY_SCALE,
            public_work_monthly_maintenance: MONEY_SCALE / 20,
            security_recovery_budget_ratio: 1.5,
            security_decline_budget_ratio: 0.5,
            ordinary_energy_import_limit: 6.0,
            water_priority_fraction: 0.92,
            water_m3_per_slot: 50_000_000.0,
            water_slot_min: 0,
            water_slot_max: 64,
            capital_output_ratio: 0.5,
            protected_treasury: 5 * MONEY_SCALE,
            clinic_base_cost: 2 * MONEY_SCALE,
            power_base_cost: 2 * MONEY_SCALE,
            water_base_cost: MONEY_SCALE,
            retrofit_cost: 3 * MONEY_SCALE,
            retrofit_output_increase: 0.2,
            labor_protection_wage_multiplier: 1.1,
        }
    }
}
impl GovernanceCalibration {
    fn validate(&self) -> Result<()> {
        if !(0.0..=1.0).contains(&self.initial_skilled_idle_share)
            || !self.security_recovery_budget_ratio.is_finite()
            || self.security_recovery_budget_ratio <= 1.0
            || !self.security_decline_budget_ratio.is_finite()
            || !(0.0..1.0).contains(&self.security_decline_budget_ratio)
            || !self.ordinary_energy_import_limit.is_finite()
            || self.ordinary_energy_import_limit < 0.0
            || !self.water_m3_per_slot.is_finite()
            || self.water_m3_per_slot <= 0.0
            || self.water_slot_min > self.water_slot_max
            || self.status != "pending_user_confirmation"
            || self.owner.is_empty()
            || self.review_date.is_empty()
            || !(0.0..=1.0).contains(&self.migration_source_population_share)
            || !self.maximum_payback_months.is_finite()
            || self.maximum_payback_months <= 0.0
            || self.administrative_capacity == 0
            || self.housing_workers == 0
            || self.clinic_people == 0
            || self.road_output_capacity == 0
            || self.power_output_capacity == 0
            || !(0.0..=1.0).contains(&self.water_priority_fraction)
            || !(0.0..=1.0).contains(&self.capital_output_ratio)
            || !(0.0..1.0).contains(&self.retrofit_output_increase)
            || !self.labor_protection_wage_multiplier.is_finite()
            || self.labor_protection_wage_multiplier < 1.0
            || self.labor_protection_wage_multiplier > 2.0
        {
            return Err(Error::Invalid("governance calibration"));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SiteState {
    pub population: u32,
    pub initial_population: u32,
    pub idle_workers: u32,
    pub skilled_workers: u32,
    pub housing_capacity: u32,
    pub clinic_capacity: u32,
    pub road_capacity: u32,
    pub power_capacity: u32,
    pub vocational_schools: u32,
    pub teachers: u32,
    pub enrolled_students: u32,
    pub training_months: u32,
    pub water_access: bool,
    pub admitted: bool,
    pub subsidy_cap: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PublicProject {
    pub id: String,
    pub site: u32,
    pub kind: PublicWork,
    pub road_edge: Option<u32>,
    pub cost: u64,
    pub construction_months: u32,
    pub worked_months: u32,
    pub started_month: u32,
    pub activate_month: Option<u32>,
    pub active: bool,
    pub water_suspended: bool,
    pub first_operated_month: Option<u32>,
    pub paused: bool,
    pub escrow: String,
    pub reserved_materials: u64,
    pub expedite_months: u32,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum FactoryPhase {
    Construction,
    Operating,
    Reduced,
    ClosureWarned,
    Closed,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Factory {
    pub id: String,
    pub group: Group,
    pub site: u32,
    pub phase: FactoryPhase,
    pub water_suspended: bool,
    pub first_operated_month: Option<u32>,
    pub started_month: u32,
    pub activate_month: u32,
    pub construction_months: u32,
    pub worked_months: u32,
    pub capital: u64,
    pub subsidy: u64,
    pub subsidy_escrow: Option<String>,
    pub capital_escrow: String,
    pub reserved_materials: u64,
    pub loss_months: u32,
    pub close_month: Option<u32>,
    pub workers: u32,
    pub skilled_workers: u32,
    pub output: f64,
    pub domestic_sales: f64,
    pub export_sales: f64,
    pub ordinary_energy_used: f64,
    pub maritime_energy_used: f64,
    pub after_tax_profit: i128,
    pub operating_subsidy: u64,
    pub retrofitted: bool,
    pub retrofit_activate_month: Option<u32>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct InvestmentIntent {
    pub id: String,
    pub group: Group,
    pub site: u32,
    pub created_month: u32,
    pub due_month: u32,
    pub subsidy: u64,
    pub required_projects: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Commitment {
    pub id: String,
    pub group: Group,
    pub site: u32,
    pub accepted_month: u32,
    pub deadline: u32,
    pub subsidy: u64,
    pub escrow: Option<String>,
    pub required_projects: Vec<String>,
    pub status: CommitmentStatus,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum CommitmentStatus {
    Pending,
    Fulfilled,
    Breached,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum TodoKind {
    PromiseDelay,
    Closure,
    Funding,
    Renewal,
    Intent,
    ContractProposal,
    ContractRisk,
}
impl TodoKind {
    fn rank(self) -> u8 {
        match self {
            Self::PromiseDelay | Self::Closure | Self::Funding | Self::ContractRisk => 0,
            Self::Renewal => 1,
            Self::Intent => 2,
            Self::ContractProposal => 3,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GovernanceTodo {
    pub id: String,
    pub kind: TodoKind,
    pub created_month: u32,
    pub due_month: u32,
    pub default_result: String,
    pub options: Vec<String>,
    pub causes: Vec<String>,
    pub resolved: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Fact {
    pub month: u32,
    pub effect: String,
    pub causes: Vec<String>,
    pub deadline: Option<u32>,
    pub default_result: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MonthRecord {
    pub month: u32,
    pub output_by_site: BTreeMap<u32, f64>,
    pub scale_met: bool,
    pub sustainable: bool,
    pub housing_ok: bool,
    pub clinic_ok: bool,
    pub security_ok: bool,
    pub treasury: u64,
    pub recurring_balance: i128,
    pub aid: u64,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum GoalGrade {
    Achieved,
    Partial,
    NotAchieved,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GovernanceReport {
    pub month: u32,
    pub grade: GoalGrade,
    pub first_sustained_month: Option<u32>,
    pub fiscal_recovered: bool,
    pub failures: Vec<String>,
    pub causes: Vec<Fact>,
    pub housing_failed_months: Vec<u32>,
    pub clinic_failed_months: Vec<u32>,
    pub security_failed_months: Vec<u32>,
    pub treasury_net_energy_liability: i128,
    pub unpaid_energy_arrears: u64,
    pub future_energy_liability: u64,
    pub final_three_recurring: Vec<i128>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Scorecard {
    pub group: Group,
    pub site: u32,
    pub factory_capital: u64,
    pub cash_gap: u64,
    pub gaps: BTreeMap<String, f64>,
    pub promised_projects: Vec<String>,
    pub autonomous_ready: bool,
    pub conditional_ready: bool,
    pub recognizes_promises_after_month: u32,
    pub six_month_sales_forecast: Vec<f64>,
    pub forecast_monthly_profit: i128,
    pub static_payback_months: Option<f64>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TreasuryForecast {
    pub month: u32,
    pub treasury: u64,
    pub recurring_balance: i128,
    pub aid: u64,
    pub committed_escrow: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct CompletedPlan {
    month: u32,
    commands: Vec<(String, GovernanceCommand)>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GovernanceState {
    pub rules_version: String,
    pub institution_changes: BTreeMap<Institution, (u32, bool)>,
    pub institution_last_change: BTreeMap<Institution, u32>,
    pub last_diplomatic_spend: u64,
    pending_taxes: Vec<(String, Group, u64, u64)>,
    pub ordinary_energy_deliveries: Vec<(u32, Group, u64)>,
    monthly_energy_used: f64,
    current_energy_contract_paid: u64,
    monthly_existing_payroll: u64,
    pub pending_export_orders: Option<(u32, f64)>,
    pub calibration: GovernanceCalibration,
    pub selected_candidate: Option<u32>,
    pub sites: BTreeMap<u32, SiteState>,
    pub projects: BTreeMap<String, PublicProject>,
    pub factories: BTreeMap<String, Factory>,
    pub intents: BTreeMap<String, InvestmentIntent>,
    pub commitments: BTreeMap<String, Commitment>,
    pub distrust_until: BTreeMap<Group, u32>,
    pub todos: BTreeMap<String, GovernanceTodo>,
    pub facts: Vec<Fact>,
    pub history: Vec<MonthRecord>,
    pub report: Option<GovernanceReport>,
    pub planned: Vec<(String, GovernanceCommand)>,
    completed_plans: Vec<CompletedPlan>,
    pub decision_log: BTreeMap<String, (u32, GovernanceCommand)>,
    pub renewal: Option<(u32, RenewalChoice)>,
    pub aid_breached: bool,
    pub cooperation_start: Option<u32>,
    pub cooperation_months: u32,
    pub cooperation_risk_since: Option<u32>,
    pub energy_contract_start: Option<u32>,
    pub energy_long_start: Option<u32>,
    pub budgets: BTreeMap<Service, u64>,
    pub spending_priority: Vec<Service>,
    pub security: f64,
    pub health_funding_ratio: f64,
    pub clinical_coverage_institution: bool,
    pub labor_protection: bool,
    pub export_orders: f64,
    pub dismissed_intents: BTreeSet<String>,
    pub last_energy_unused_bill: u64,
    pub last_recurring_balance: i128,
    pub last_aid: u64,
    pub next_factory_id: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SystemReceipt {
    pub id: u64,
    pub month: u32,
    pub account: String,
    pub asset: Asset,
    pub delta: i128,
    pub external_counterbalance: i128,
    pub reason: String,
    pub external_party: String,
}
impl SystemReceipt {
    pub(crate) fn validate(&self, game: &Game) -> Result<()> {
        if self.id as usize >= game.system_receipts.len()
            || game.system_receipts[self.id as usize] != *self
            || self.month == 0
            || self.month >= game.month
            || self.delta == 0
            || self.delta.checked_neg() != Some(self.external_counterbalance)
            || !game.ledger.accounts.contains_key(&self.account)
            || self.reason.is_empty()
            || self.external_party != external_party(&self.reason)
            || game.governance.is_none()
        {
            return Err(Error::Invalid("system receipt"));
        }
        validate_asset(&self.asset)
    }
}

impl Game {
    pub fn confirmed_game() -> Result<Self> {
        Self::confirmed_game_with_calibration(GovernanceCalibration::default())
    }
    pub fn confirmed_game_with_calibration(calibration: GovernanceCalibration) -> Result<Self> {
        let doc = document()?;
        if doc["activation"] != "enabled" {
            return Err(Error::Invalid("confirmed initial document not activated"));
        }
        Self::governance_from_map(scenario::generated()?, calibration, doc)
    }
    fn governance_from_map(
        map: Value,
        calibration: GovernanceCalibration,
        doc: &Value,
    ) -> Result<Self> {
        calibration.validate()?;
        let mut accounts = Vec::new();
        for (id, amount) in [
            ("treasury", param(doc, "opening_treasury")?),
            ("group_a", param(doc, "group_a_starting_cash")?),
            ("group_b", param(doc, "group_b_starting_cash")?),
            ("construction", 0.0),
            ("households", 0.0),
            ("capital_inventory", 0.0),
            ("resource_supplier", 0.0),
            ("foreign_buyer", 0.0),
        ] {
            accounts.push(InitialAccount {
                id: id.into(),
                balances: BTreeMap::from([(Asset::Money, money(amount)?)]),
            });
        }
        accounts[5].balances.insert(
            Asset::Material("construction".into()),
            number(doc, "/materials/initial_inventory/value")? as u64,
        );
        accounts[4].balances.insert(
            Asset::Material("food".into()),
            number(doc, "/food/initial_inventory/value")? as u64,
        );
        let mut quota_rules = Rules::pending_sample();
        quota_rules.water_m3_per_slot = calibration.water_m3_per_slot;
        quota_rules.water_min = calibration.water_slot_min;
        quota_rules.water_max = calibration.water_slot_max;
        let mut game = Self::from_map(
            map,
            Scenario::fixed(),
            quota_rules,
            InitialState {
                status: Confirmation::Confirmed,
                provenance: "D022:3f8bfb423d6d686da03c25b7fcab706e3ddd6af9".into(),
                accounts,
            },
        )?;
        for (site, land) in [(39, 3), (74, 2)] {
            let source = game.projection.cells[&site]
                .water_source_ids
                .iter()
                .copied()
                .max_by_key(|id| game.projection.water[id])
                .ok_or(Error::Missing("existing industry source"))?;
            let reservation = format!("initial-industry/{site}");
            game.apply_internal(
                format!("initial/reserve/{site}"),
                Command::Reserve(Reservation {
                    id: reservation.clone(),
                    country: "player".into(),
                    cell: site,
                    source,
                    land,
                    water: land,
                    purpose: LandUse::Factory,
                    phase: AllocationPhase::Reserved,
                }),
            )?;
            game.apply_internal(
                format!("initial/operate/{site}"),
                Command::MarkOperational { reservation },
            )?;
        }
        let mut sites = BTreeMap::new();
        for site in &game.scenario.countries["player"] {
            let population = number(doc, &format!("/regions/{site}/population/value"))? as u32;
            let idle = number(doc, &format!("/regions/{site}/idle/value"))? as u32;
            sites.insert(
                *site,
                SiteState {
                    population,
                    initial_population: population,
                    idle_workers: idle,
                    skilled_workers: if [39, 74].contains(site) {
                        (idle as f64 * calibration.initial_skilled_idle_share).floor() as u32
                    } else {
                        0
                    },
                    housing_capacity: population,
                    clinic_capacity: population,
                    road_capacity: if [39, 74].contains(site) { 1000 } else { 0 },
                    power_capacity: if [39, 74].contains(site) { 1000 } else { 0 },
                    vocational_schools: 0,
                    teachers: 0,
                    enrolled_students: 0,
                    training_months: 0,
                    water_access: [39, 74].contains(site),
                    admitted: [39, 74].contains(site),
                    subsidy_cap: 0,
                },
            );
        }
        let mut budgets = BTreeMap::new();
        for (kind, name) in [
            (Service::Health, "health"),
            (Service::Safety, "safety"),
            (Service::Education, "education"),
            (Service::Maintenance, "maintenance"),
        ] {
            budgets.insert(
                kind,
                money(number(
                    doc,
                    &format!("/fiscal/spending_breakdown/{name}/value"),
                )?)?,
            );
        }
        game.governance = Some(GovernanceState {
            rules_version: GOVERNANCE_RULES_VERSION.into(),
            institution_changes: BTreeMap::new(),
            institution_last_change: BTreeMap::new(),
            last_diplomatic_spend: 0,
            pending_taxes: vec![],
            ordinary_energy_deliveries: vec![],
            monthly_energy_used: 0.0,
            current_energy_contract_paid: 0,
            monthly_existing_payroll: 0,
            pending_export_orders: None,
            calibration,
            selected_candidate: None,
            sites,
            projects: BTreeMap::new(),
            factories: BTreeMap::new(),
            intents: BTreeMap::new(),
            commitments: BTreeMap::new(),
            distrust_until: BTreeMap::new(),
            todos: BTreeMap::new(),
            facts: vec![Fact {
                month: 1,
                effect: "transition_aid_expires_month_13".into(),
                causes: vec!["D011".into()],
                deadline: Some(12),
                default_result: Some("aid_expires".into()),
            }],
            history: vec![],
            report: None,
            planned: vec![],
            completed_plans: vec![],
            decision_log: BTreeMap::new(),
            renewal: None,
            aid_breached: false,
            cooperation_start: None,
            cooperation_months: 0,
            cooperation_risk_since: None,
            energy_contract_start: None,
            energy_long_start: None,
            budgets,
            spending_priority: vec![
                Service::Health,
                Service::Safety,
                Service::Maintenance,
                Service::Education,
            ],
            security: 60.0,
            health_funding_ratio: 1.0,
            clinical_coverage_institution: false,
            labor_protection: false,
            export_orders: 0.0,
            dismissed_intents: BTreeSet::new(),
            last_energy_unused_bill: 0,
            last_recurring_balance: 0,
            last_aid: 0,
            next_factory_id: 1,
        });
        game.validate()?;
        Ok(game)
    }
    pub fn governance(&self) -> Option<&GovernanceState> {
        self.governance.as_ref()
    }
    pub fn system_receipts(&self) -> &[SystemReceipt] {
        &self.system_receipts
    }
    pub fn external_counterbalances(&self) -> Result<BTreeMap<Asset, i128>> {
        let mut totals = BTreeMap::new();
        for r in &self.system_receipts {
            let entry = totals.entry(r.asset.clone()).or_insert(0i128);
            *entry = entry
                .checked_add(r.external_counterbalance)
                .ok_or(Error::Overflow)?;
        }
        Ok(totals)
    }
    pub fn queue_governance(
        &mut self,
        id: impl Into<String>,
        command: GovernanceCommand,
    ) -> Result<()> {
        let id = id.into();
        identifier(&id)?;
        let state = self
            .governance
            .as_ref()
            .ok_or(Error::Invalid("formal game required"))?;
        if let Some((_, existing)) = state.planned.iter().find(|(key, _)| key == &id) {
            return if existing == &command {
                Ok(())
            } else {
                Err(Error::Conflict)
            };
        }
        if let Some((_, existing)) = state.decision_log.get(&id) {
            return if existing == &command {
                Ok(())
            } else {
                Err(Error::Conflict)
            };
        }
        let mut candidate = self.clone();
        candidate
            .governance
            .as_mut()
            .unwrap()
            .planned
            .push((id.clone(), command));
        let mut projected = candidate.clone();
        let mut state = projected.governance.take().unwrap();
        state.apply_plans(&mut projected)?;
        *self = candidate;
        Ok(())
    }
    pub fn withdraw_governance(&mut self, id: &str) -> Result<()> {
        let state = self
            .governance
            .as_mut()
            .ok_or(Error::Invalid("formal game required"))?;
        let pos = state
            .planned
            .iter()
            .position(|(key, _)| key == id)
            .ok_or(Error::Missing("planned command"))?;
        let mut candidate = self.clone();
        candidate.governance.as_mut().unwrap().planned.remove(pos);
        let mut projected = candidate.clone();
        let mut state = projected.governance.take().unwrap();
        state.apply_plans(&mut projected)?;
        *self = candidate;
        Ok(())
    }
    pub fn scorecard(&self, group: Group, site: u32) -> Result<Scorecard> {
        let mut projected = self.clone();
        let mut state = projected
            .governance
            .take()
            .ok_or(Error::Invalid("formal game required"))?;
        state.apply_plans(&mut projected)?;
        state.assess(&projected, group, site)
    }
    pub fn treasury_forecast(&self) -> Result<Vec<TreasuryForecast>> {
        let mut game = self.clone();
        if game.governance.is_none() {
            return Err(Error::Invalid("formal game required"));
        }
        let mut result = vec![];
        while game.month <= 24 {
            let month = game.month;
            game.advance_month_unchecked()?;
            let s = game.governance.as_ref().unwrap();
            result.push(TreasuryForecast {
                month,
                treasury: game.balance("treasury", &Asset::Money).unwrap(),
                recurring_balance: s.last_recurring_balance,
                aid: s.last_aid,
                committed_escrow: game
                    .ledger
                    .escrows
                    .values()
                    .filter(|e| e.payer == "treasury" && !e.cancelled)
                    .map(|e| e.total - e.paid)
                    .sum(),
            });
        }
        Ok(result)
    }
    fn settle_external(
        &mut self,
        account: &str,
        asset: Asset,
        delta: i128,
        reason: &str,
    ) -> Result<()> {
        if delta == 0 {
            return Ok(());
        }
        if external_party(reason) == "unspecified_settlement" {
            return Err(Error::Invalid("unknown external settlement counterparty"));
        }
        let balances = self
            .ledger
            .accounts
            .get_mut(account)
            .ok_or(Error::Missing("system settlement account"))?;
        let value = balances.get(&asset).copied().unwrap_or(0) as i128;
        let value = value.checked_add(delta).ok_or(Error::Overflow)?;
        let value = u64::try_from(value).map_err(|_| Error::Insufficient("system settlement"))?;
        balances.insert(asset.clone(), value);
        self.system_receipts.push(SystemReceipt {
            id: self.system_receipts.len() as u64,
            month: self.month,
            account: account.into(),
            asset,
            delta,
            external_counterbalance: delta.checked_neg().ok_or(Error::Overflow)?,
            reason: reason.into(),
            external_party: external_party(reason).into(),
        });
        Ok(())
    }
    pub(crate) fn conserved_with_external(&self, initial: &Ledger) -> Result<bool> {
        let actual = self.ledger.totals()?;
        let expected = initial.totals()?;
        let external = self.external_counterbalances()?;
        let assets: BTreeSet<_> = actual
            .keys()
            .chain(expected.keys())
            .chain(external.keys())
            .cloned()
            .collect();
        for asset in assets {
            let actual = i128::try_from(actual.get(&asset).copied().unwrap_or(0))
                .map_err(|_| Error::Overflow)?;
            let expected = i128::try_from(expected.get(&asset).copied().unwrap_or(0))
                .map_err(|_| Error::Overflow)?;
            if actual.checked_add(external.get(&asset).copied().unwrap_or(0)) != Some(expected) {
                return Ok(false);
            }
        }
        Ok(true)
    }
    pub(crate) fn validate_governance(&self) -> Result<()> {
        match &self.governance {
            None if !self.system_receipts.is_empty() => {
                Err(Error::Invalid("system receipts without governance"))
            }
            None => Ok(()),
            Some(s) => s.validate(self),
        }
    }
    pub(crate) fn validate_governance_replay(&self) -> Result<()> {
        let Some(state) = &self.governance else {
            return Ok(());
        };
        let doc = document()?;
        let mut replay =
            Self::governance_from_map(self.map.clone(), state.calibration.clone(), doc)?;
        for completed in &state.completed_plans {
            if completed.month != replay.month {
                return Err(Error::Invalid("governance plan history"));
            }
            replay.governance.as_mut().unwrap().planned = completed.commands.clone();
            replay.advance_month_unchecked()?;
        }
        replay.governance.as_mut().unwrap().planned = state.planned.clone();
        if replay.governance.as_ref() != Some(state)
            || replay.ledger != self.ledger
            || replay.system_receipts != self.system_receipts
            || serde_json::to_value(&replay.applied)? != serde_json::to_value(&self.applied)?
            || replay.reservations != self.reservations
            || replay.month != self.month
            || replay.admissions != self.admissions
            || replay.initial != self.initial
            || replay.rules != self.rules
        {
            return Err(Error::Invalid("governance business replay mismatch"));
        }
        let mut projected = replay.clone();
        let mut projected_state = projected.governance.take().unwrap();
        projected_state.apply_plans(&mut projected)?;
        Ok(())
    }
}

impl GovernanceState {
    fn fact(&mut self, month: u32, effect: impl Into<String>, causes: Vec<String>) {
        self.facts.push(Fact {
            month,
            effect: effect.into(),
            causes,
            deadline: None,
            default_result: None,
        });
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "one internal constructor keeps all required deadline and default fields together"
    )]
    fn todo(
        &mut self,
        id: String,
        kind: TodoKind,
        month: u32,
        due: u32,
        default: &str,
        options: &[&str],
        causes: Vec<String>,
    ) {
        self.todos.entry(id.clone()).or_insert(GovernanceTodo {
            id,
            kind,
            created_month: month,
            due_month: due,
            default_result: default.into(),
            options: options.iter().map(|s| s.to_string()).collect(),
            causes,
            resolved: false,
        });
    }
    pub fn visible_todos(&self, month: u32) -> Vec<&GovernanceTodo> {
        let mut todos: Vec<_> = self
            .todos
            .values()
            .filter(|t| !t.resolved && t.created_month <= month && t.due_month >= month)
            .collect();
        todos.sort_by_key(|t| (t.kind.rank(), t.created_month, t.id.clone()));
        todos.truncate(3);
        todos
    }
    fn apply(&mut self, game: &mut Game, id: &str, command: &GovernanceCommand) -> Result<()> {
        let month = game.month;
        match command {
            GovernanceCommand::SelectCandidate { site } => {
                if !game.scenario.candidates.contains(site) {
                    return Err(Error::Invalid("inland candidate"));
                }
                if self.selected_candidate.is_some_and(|old| old != *site) {
                    return Err(Error::Invalid("candidate already selected"));
                }
                self.selected_candidate = Some(*site);
            }
            GovernanceCommand::OpenRegion { site } => {
                self.sites
                    .get_mut(site)
                    .ok_or(Error::Invalid("domestic region"))?
                    .admitted = true;
            }
            GovernanceCommand::SetSubsidy { site, cap } => {
                self.sites
                    .get_mut(site)
                    .ok_or(Error::Invalid("domestic region"))?
                    .subsidy_cap = *cap;
                self.dismissed_intents
                    .retain(|key| !key.ends_with(&format!("/{site}")));
            }
            GovernanceCommand::BuildPublic {
                id: project_id,
                site,
                kind,
                road_edge,
            } => {
                self.build_public(game, project_id, *site, *kind, *road_edge, id)?;
            }
            GovernanceCommand::AcceptIntent {
                id: intent_id,
                deadline,
            } => {
                let intent = self
                    .intents
                    .get(intent_id)
                    .ok_or(Error::Missing("investment intent"))?
                    .clone();
                if month > intent.due_month || *deadline < month {
                    return Err(Error::Invalid("commitment deadline"));
                }
                if self.active_commitments() >= self.calibration.administrative_capacity {
                    return Err(Error::Insufficient("administrative capacity"));
                }
                let card = self.assess(game, intent.group, intent.site)?;
                if !card.conditional_ready {
                    return Err(Error::Invalid("intent conditions changed"));
                }
                let subsidy = card.cash_gap;
                if subsidy > self.sites[&intent.site].subsidy_cap {
                    return Err(Error::Insufficient("subsidy cap"));
                }
                let escrow = if subsidy > 0 {
                    let escrow = format!("subsidy/{intent_id}");
                    game.apply_internal(
                        format!("{id}/fund"),
                        Command::FundEscrow {
                            escrow: escrow.clone(),
                            payer: "treasury".into(),
                            payee: "construction".into(),
                            amount: subsidy,
                        },
                    )?;
                    Some(escrow)
                } else {
                    None
                };
                self.commitments.insert(
                    intent_id.clone(),
                    Commitment {
                        id: intent_id.clone(),
                        group: intent.group,
                        site: intent.site,
                        accepted_month: month,
                        deadline: *deadline,
                        subsidy,
                        escrow,
                        required_projects: card.promised_projects,
                        status: CommitmentStatus::Pending,
                    },
                );
                self.intents.remove(intent_id);
                if let Some(todo) = self.todos.get_mut(intent_id) {
                    todo.resolved = true;
                }
                self.fact(
                    month,
                    "conditional_investment_accepted",
                    vec![id.into(), intent_id.clone()],
                );
            }
            GovernanceCommand::RejectIntent { id: intent_id } => {
                let intent = self
                    .intents
                    .remove(intent_id)
                    .ok_or(Error::Missing("investment intent"))?;
                self.dismissed_intents
                    .insert(format!("{:?}/{}", intent.group, intent.site));
                if let Some(todo) = self.todos.get_mut(intent_id) {
                    todo.resolved = true;
                }
                self.fact(
                    month,
                    "investment_intent_rejected",
                    vec![id.into(), intent_id.clone()],
                );
            }
            GovernanceCommand::SignCooperation => {
                if self.cooperation_start.is_some() {
                    return Err(Error::Conflict);
                }
                if self.security < 50.0 {
                    return Err(Error::Invalid("cooperation security condition"));
                }
                self.cooperation_start = Some(month);
                self.cooperation_months = 0;
                self.cooperation_risk_since = None;
                if let Some(t) = self.todos.get_mut("maritime/proposal") {
                    t.resolved = true;
                }
            }
            GovernanceCommand::SignEnergy => {
                if self.energy_contract_start.is_some() {
                    return Err(Error::Conflict);
                }
                self.energy_contract_start = Some(month);
                if let Some(t) = self.todos.get_mut("resource/proposal") {
                    t.resolved = true;
                }
            }
            GovernanceCommand::RenewAid { choice } => {
                if !(9..=12).contains(&month) || self.renewal.is_some() {
                    return Err(Error::Invalid("aid renewal window or already decided"));
                }
                self.renewal = Some((month, *choice));
                if let Some(t) = self.todos.get_mut("aid/renewal") {
                    t.resolved = true;
                }
                if *choice == RenewalChoice::EnergyLongTerm {
                    self.energy_long_start = Some(month);
                    let doc = document()?;
                    game.apply_internal(
                        format!("{id}/obligation"),
                        Command::RecordObligation(PaymentObligation {
                            id: "resource_energy_long_term".into(),
                            payer: "treasury".into(),
                            payee: "resource_supplier".into(),
                            start_month: month,
                            months: 24,
                            quantity_per_month: number(doc, "/energy/renewal_quantity")? as u64,
                            unit_price: money(param(doc, "long_energy_price")?)?,
                        }),
                    )?;
                }
                if *choice == RenewalChoice::WaterPriority {
                    for source in self.shared_resource_sources(game) {
                        let in_construction: Vec<_> = game
                            .reservations
                            .values()
                            .filter(|r| {
                                r.country == "player"
                                    && r.source == source
                                    && r.phase == AllocationPhase::Reserved
                            })
                            .map(|r| r.id.clone())
                            .collect();
                        for reservation in in_construction {
                            game.apply_internal(
                                format!("{id}/wait/{reservation}"),
                                Command::SetWaterWaiting {
                                    reservation,
                                    waiting: true,
                                },
                            )?;
                        }
                        let total = game.projection.water[&source];
                        let occupied =
                            (total as f64 * self.calibration.water_priority_fraction).ceil() as u64;
                        self.limit_operating_water(
                            game,
                            source,
                            total.saturating_sub(occupied),
                            occupied,
                        )?;
                        let used: u64 = game
                            .reservations
                            .values()
                            .filter(|r| {
                                r.source == source
                                    && !matches!(
                                        r.phase,
                                        AllocationPhase::WaitingForWater
                                            | AllocationPhase::OperationalWaitingForWater
                                            | AllocationPhase::Retired
                                    )
                            })
                            .map(|r| r.water)
                            .sum();
                        let additional = occupied.min(total.saturating_sub(used));
                        if additional > 0 {
                            let cell = game.scenario.countries["resource"]
                                .iter()
                                .copied()
                                .find(|c| {
                                    game.projection.cells[c].water_source_ids.contains(&source)
                                })
                                .unwrap();
                            let r = Reservation {
                                id: format!("resource-priority/{source}"),
                                country: "resource".into(),
                                cell,
                                source,
                                land: 0,
                                water: additional,
                                purpose: LandUse::Housing,
                                phase: AllocationPhase::Reserved,
                            };
                            game.apply_internal(
                                format!("{id}/resource/{source}"),
                                Command::Reserve(r),
                            )?;
                        }
                        game.apply_internal(
                            format!("{id}/limit/{source}"),
                            Command::AddAdmissionLimitProtected {
                                protected: game
                                    .reservations
                                    .values()
                                    .filter(|r| {
                                        r.source == source
                                            && r.country == "player"
                                            && r.id.starts_with("initial-industry/")
                                            && r.phase == AllocationPhase::Operational
                                    })
                                    .map(|r| r.id.clone())
                                    .collect(),
                                limit: AdmissionLimit {
                                    id: format!("W/{source}"),
                                    country: "player".into(),
                                    source,
                                    new_water_slots: total.saturating_sub(occupied),
                                    start_month: month,
                                    end_month: 24,
                                },
                            },
                        )?;
                    }
                }
                self.fact(month, format!("aid_renewal_{choice:?}"), vec![id.into()]);
            }
            GovernanceCommand::SetSecurity { monthly } => {
                self.budgets.insert(Service::Safety, *monthly);
            }
            GovernanceCommand::SetServiceBudget { kind, monthly } => {
                self.budgets.insert(*kind, *monthly);
            }
            GovernanceCommand::SetInstitution { kind, enabled } => {
                if *kind == Institution::ClinicalCoverage {
                    return Err(Error::Invalid(
                        "clinical coverage switching is outside the first governance slice",
                    ));
                }
                if self.institution_changes.contains_key(kind)
                    || self
                        .institution_last_change
                        .get(kind)
                        .is_some_and(|last| month < last + 3)
                {
                    return Err(Error::Invalid("institution three-month cooldown"));
                }
                let current = match kind {
                    Institution::ClinicalCoverage => self.clinical_coverage_institution,
                    Institution::LaborProtection => self.labor_protection,
                };
                if current == *enabled {
                    return Err(Error::Invalid("institution unchanged"));
                }
                if *enabled && self.active_commitments() >= self.calibration.administrative_capacity
                {
                    return Err(Error::Insufficient("administrative capacity"));
                }
                self.institution_changes
                    .insert(*kind, (month + 1, *enabled));
                self.institution_last_change.insert(*kind, month);
                self.fact(month, "institution_announced_next_month", vec![id.into()]);
            }
            GovernanceCommand::SetSpendingPriority { order } => {
                if order.len() != 4 || order.iter().copied().collect::<BTreeSet<_>>().len() != 4 {
                    return Err(Error::Invalid("all four spending services exactly once"));
                }
                self.spending_priority = order.clone();
            }
            GovernanceCommand::PauseProject {
                id: project_id,
                paused,
            } => {
                let p = self
                    .projects
                    .get_mut(project_id)
                    .ok_or(Error::Missing("public project"))?;
                if p.active {
                    return Err(Error::Invalid("completed project"));
                }
                p.paused = *paused;
            }
            GovernanceCommand::ExpediteProject {
                id: project_id,
                additional_months,
            } => {
                let p = self
                    .projects
                    .get_mut(project_id)
                    .ok_or(Error::Missing("public project"))?;
                let remaining = p
                    .construction_months
                    .saturating_sub(p.worked_months)
                    .saturating_sub(1)
                    .saturating_sub(p.expedite_months);
                if *additional_months == 0
                    || *additional_months > remaining
                    || p.active
                    || p.activate_month.is_some()
                {
                    return Err(Error::Invalid("extra construction progress"));
                }
                let fee = ((p.cost as u128 * *additional_months as u128)
                    .div_ceil(p.construction_months as u128)) as u64;
                game.apply_internal(
                    format!("{id}/fee"),
                    Command::Transfer {
                        from: "treasury".into(),
                        to: "construction".into(),
                        asset: Asset::Money,
                        amount: fee,
                    },
                )?;
                p.expedite_months += *additional_months;
                p.paused = false;
                self.fact(
                    month,
                    "additional_paid_construction_progress",
                    vec![id.into(), project_id.clone()],
                );
            }
            GovernanceCommand::Retrofit { factory } => {
                let f = self
                    .factories
                    .get_mut(factory)
                    .ok_or(Error::Missing("factory"))?;
                if f.retrofitted
                    || f.retrofit_activate_month.is_some()
                    || matches!(f.phase, FactoryPhase::Construction | FactoryPhase::Closed)
                {
                    return Err(Error::Invalid("operating unmodified factory"));
                }
                game.settle_external(
                    f.group.account(),
                    Asset::Money,
                    -(self.calibration.retrofit_cost as i128),
                    "equipment_retrofit",
                )?;
                game.apply_internal(
                    format!("{id}/materials"),
                    Command::Transfer {
                        from: "capital_inventory".into(),
                        to: "construction".into(),
                        asset: Asset::Material("construction".into()),
                        amount: 2,
                    },
                )?;
                game.settle_external(
                    "construction",
                    Asset::Material("construction".into()),
                    -2,
                    "equipment_retrofit_materials",
                )?;
                f.retrofit_activate_month = Some(month + 2);
            }
        }
        self.decision_log
            .insert(id.into(), (month, command.clone()));
        Ok(())
    }
    fn active_commitments(&self) -> usize {
        self.commitments
            .values()
            .filter(|c| c.status == CommitmentStatus::Pending)
            .count()
            + self
                .projects
                .values()
                .filter(|p| !p.active || p.kind == PublicWork::Vocational)
                .count()
            + usize::from(self.clinical_coverage_institution)
            + usize::from(self.labor_protection)
            + self
                .institution_changes
                .values()
                .filter(|(_, enabled)| *enabled)
                .count()
    }
    fn apply_plans(&mut self, game: &mut Game) -> Result<()> {
        for (id, command) in self.planned.clone() {
            self.apply(game, &id, &command)?;
        }
        Ok(())
    }
    fn shared_resource_sources(&self, game: &Game) -> BTreeSet<u32> {
        let player: BTreeSet<_> = game.scenario.countries["player"]
            .iter()
            .flat_map(|c| game.projection.cells[c].water_source_ids.iter().copied())
            .collect();
        game.scenario.countries["resource"]
            .iter()
            .flat_map(|c| game.projection.cells[c].water_source_ids.iter().copied())
            .filter(|s| player.contains(s))
            .collect()
    }
    fn source(&self, game: &Game, site: u32) -> Result<u32> {
        game.projection
            .cells
            .get(&site)
            .ok_or(Error::Missing("site"))?
            .water_source_ids
            .iter()
            .copied()
            .max_by_key(|s| game.projection.water[s])
            .ok_or(Error::Missing("water source"))
    }
    fn reserve(&mut self, game: &mut Game, id: &str, site: u32, purpose: LandUse) -> Result<()> {
        let source = self.source(game, site)?;
        game.apply_internal(
            format!("reserve/{id}"),
            Command::Reserve(Reservation {
                id: id.into(),
                country: "player".into(),
                cell: site,
                source,
                land: 1,
                water: 1,
                purpose,
                phase: AllocationPhase::Reserved,
            }),
        )?;
        Ok(())
    }
    pub fn public_work_quote(
        &self,
        game: &Game,
        site: u32,
        kind: PublicWork,
        road_edge: Option<u32>,
    ) -> Result<(u64, u32)> {
        let doc = document()?;
        let natural = game
            .cell(site)
            .ok_or(Error::Missing("site"))?
            .cost_multiplier
            .ok_or(Error::Invalid("unbuildable"))?;
        let (base, months) = match kind {
            PublicWork::Housing => (
                param(doc, "d021_housing_base_capital")?,
                param(doc, "d021_housing_construction_months")? as u32,
            ),
            PublicWork::Road => (
                param(doc, "d021_road_base_capital")?,
                param(doc, "d021_road_construction_months")? as u32,
            ),
            PublicWork::Vocational => (
                param(doc, "d021_vocational_school_base_capital")?,
                param(doc, "d021_vocational_school_construction_months")? as u32,
            ),
            PublicWork::Clinic => (currency(self.calibration.clinic_base_cost), 2),
            PublicWork::Power => (currency(self.calibration.power_base_cost), 2),
            PublicWork::Water => (currency(self.calibration.water_base_cost), 2),
        };
        let corridor = if kind == PublicWork::Road {
            let id = road_edge.ok_or(Error::Missing("road edge"))?;
            let edge = game.map["edges"]
                .as_array()
                .ok_or(Error::Missing("map edges"))?
                .iter()
                .find(|e| e["id"].as_u64() == Some(id as u64))
                .ok_or(Error::Missing("road edge"))?;
            let a = edge["a"].as_u64().ok_or(Error::Invalid("edge endpoint"))? as u32;
            let b = edge["b"].as_u64().ok_or(Error::Invalid("edge endpoint"))? as u32;
            if (a != site && b != site)
                || !self.sites.contains_key(&a)
                || !self.sites.contains_key(&b)
            {
                return Err(Error::Invalid("road must join domestic site"));
            }
            edge.pointer("/passage/cost_multiplier")
                .and_then(Value::as_f64)
                .ok_or(Error::Missing("natural corridor multiplier"))?
        } else {
            if road_edge.is_some() {
                return Err(Error::Invalid("edge only for road"));
            }
            1.0
        };
        scenario::positive(corridor)?;
        Ok((money(base * natural * corridor)?, months))
    }
    fn build_public(
        &mut self,
        game: &mut Game,
        id: &str,
        site: u32,
        kind: PublicWork,
        road_edge: Option<u32>,
        cause: &str,
    ) -> Result<()> {
        identifier(id)?;
        if self.projects.contains_key(id)
            || self.factories.contains_key(id)
            || !self.sites.contains_key(&site)
        {
            return Err(Error::Invalid("new domestic public project"));
        }
        if self.active_commitments() >= self.calibration.administrative_capacity {
            return Err(Error::Insufficient("administrative capacity"));
        }
        let (cost, months) = self.public_work_quote(game, site, kind, road_edge)?;
        let materials = (months * 2) as u64;
        if matches!(kind, PublicWork::Housing) {
            self.reserve(game, &format!("public/{id}"), site, LandUse::Housing)?;
        }
        let escrow = format!("public/{id}");
        game.apply_internal(
            format!("{cause}/fund"),
            Command::FundEscrow {
                escrow: escrow.clone(),
                payer: "treasury".into(),
                payee: "construction".into(),
                amount: cost,
            },
        )?;
        game.apply_internal(
            format!("{cause}/materials"),
            Command::Transfer {
                from: "capital_inventory".into(),
                to: "construction".into(),
                asset: Asset::Material("construction".into()),
                amount: materials,
            },
        )?;
        self.projects.insert(
            id.into(),
            PublicProject {
                id: id.into(),
                site,
                kind,
                road_edge,
                cost,
                construction_months: months,
                worked_months: 0,
                started_month: game.month,
                activate_month: None,
                active: false,
                water_suspended: false,
                first_operated_month: None,
                paused: false,
                escrow,
                reserved_materials: materials,
                expedite_months: 0,
            },
        );
        self.fact(
            game.month,
            "public_construction_started",
            vec![cause.into(), id.into()],
        );
        Ok(())
    }
    fn projected_site(
        &self,
        site: u32,
        promises: bool,
        month: u32,
    ) -> Result<(SiteState, Vec<String>)> {
        let mut s = self
            .sites
            .get(&site)
            .ok_or(Error::Missing("domestic site"))?
            .clone();
        let mut projects = vec![];
        if promises {
            for p in self
                .projects
                .values()
                .filter(|p| p.site == site && !p.active && !p.paused)
            {
                if p.activate_month
                    .unwrap_or(month + p.construction_months - p.worked_months)
                    <= month + 3
                {
                    self.add_capacity(&mut s, p.kind)?;
                    projects.push(p.id.clone());
                }
            }
        }
        Ok((s, projects))
    }
    fn add_capacity(&self, s: &mut SiteState, kind: PublicWork) -> Result<()> {
        match kind {
            PublicWork::Housing => {
                s.housing_capacity = s
                    .housing_capacity
                    .checked_add(self.calibration.housing_workers)
                    .ok_or(Error::Overflow)?
            }
            PublicWork::Clinic => {
                s.clinic_capacity = s
                    .clinic_capacity
                    .checked_add(self.calibration.clinic_people)
                    .ok_or(Error::Overflow)?
            }
            PublicWork::Road => {
                s.road_capacity = s
                    .road_capacity
                    .checked_add(self.calibration.road_output_capacity)
                    .ok_or(Error::Overflow)?
            }
            PublicWork::Power => {
                s.power_capacity = s
                    .power_capacity
                    .checked_add(self.calibration.power_output_capacity)
                    .ok_or(Error::Overflow)?
            }
            PublicWork::Vocational => {
                s.vocational_schools = s.vocational_schools.checked_add(1).ok_or(Error::Overflow)?
            }
            PublicWork::Water => s.water_access = true,
        }
        Ok(())
    }
    fn assess(&self, game: &Game, group: Group, site: u32) -> Result<Scorecard> {
        self.assess_excluding_commitment(game, group, site, None)
    }
    fn assess_excluding_commitment(
        &self,
        game: &Game,
        group: Group,
        site: u32,
        own_commitment: Option<&str>,
    ) -> Result<Scorecard> {
        let doc = document()?;
        let natural = game
            .cell(site)
            .ok_or(Error::Missing("site"))?
            .cost_multiplier
            .ok_or(Error::Invalid("unbuildable"))?;
        let capital = money(param(doc, "factory_base_capital")? * natural)?;
        let buffer = money(param(doc, "operating_cash_buffer")?)?;
        let cash = game
            .balance(group.account(), &Asset::Money)
            .ok_or(Error::Missing("group"))?;
        let cash_gap = capital.saturating_sub(cash.saturating_sub(buffer));
        let recognizes = self.distrust_until.get(&group).copied().unwrap_or(0);
        let (s, promised_projects) =
            self.projected_site(site, game.month > recognizes, game.month)?;
        let actual = &self.sites[&site];
        let workers = param(doc, "new_factory_workers")? as u32;
        let output = param(doc, "new_factory_output")?;
        let allocated_workers: u32 = self
            .factories
            .values()
            .filter(|f| f.site == site && f.phase != FactoryPhase::Closed)
            .map(|f| f.workers)
            .sum();
        let allocated_output = allocated_workers as f64;
        let local_idle = actual.idle_workers;
        let domestic_idle: u32 = self.sites.values().map(|s| s.idle_workers).sum();
        let migration_space = s
            .housing_capacity
            .saturating_sub(s.population)
            .min(s.clinic_capacity.saturating_sub(s.population));
        let potential_workers = local_idle
            .saturating_add(migration_space.min(domestic_idle.saturating_sub(local_idle)));
        let domestic_new: f64 = self
            .factories
            .values()
            .filter(|f| f.phase != FactoryPhase::Closed)
            .map(|f| {
                output
                    * if f.retrofitted {
                        1.0 + self.calibration.retrofit_output_increase
                    } else {
                        1.0
                    }
            })
            .sum::<f64>()
            + self
                .commitments
                .values()
                .filter(|c| {
                    c.status == CommitmentStatus::Pending && Some(c.id.as_str()) != own_commitment
                })
                .count() as f64
                * output;
        let import_cap = number(doc, "/demand/initial_imports/value")?;
        let remaining_domestic = import_cap - domestic_new.min(import_cap);
        let export_committed = (domestic_new - import_cap).max(0.0);
        let available_orders =
            remaining_domestic.max(0.0) + (self.export_orders - export_committed).max(0.0);
        let mut gaps = BTreeMap::new();
        gaps.insert("admission".into(), if s.admitted { 0.0 } else { 1.0 });
        gaps.insert(
            "cash".into(),
            currency(cash_gap.saturating_sub(s.subsidy_cap)),
        );
        gaps.insert(
            "labor".into(),
            workers.saturating_sub(potential_workers) as f64,
        );
        let migrant_skills: u32 = self
            .sites
            .iter()
            .filter(|(id, _)| **id != site)
            .map(|(_, s)| s.skilled_workers)
            .sum();
        let potential_skills = actual.skilled_workers
            + migration_space.min(migrant_skills)
            + if s.vocational_schools > 0 {
                self.calibration.trained_workers_per_quarter
            } else {
                0
            }
            + if self.cooperation_start.is_some() {
                self.calibration.cooperation_training_per_quarter
            } else {
                0
            };
        gaps.insert(
            "skills".into(),
            self.calibration
                .factory_skill_workers
                .saturating_sub(potential_skills) as f64,
        );
        gaps.insert(
            "road".into(),
            (output + allocated_output - s.road_capacity as f64).max(0.0),
        );
        gaps.insert(
            "power".into(),
            (output + allocated_output - s.power_capacity as f64).max(0.0),
        );
        gaps.insert(
            "water_access".into(),
            if s.water_access { 0.0 } else { 1.0 },
        );
        gaps.insert("demand".into(), (output - available_orders).max(0.0));
        let source = self.source(game, site)?;
        let land_used: u64 = game
            .reservations
            .values()
            .filter(|r| r.cell == site)
            .map(|r| r.land)
            .sum();
        let water_used: u64 = game
            .reservations
            .values()
            .filter(|r| {
                r.source == source
                    && !matches!(
                        r.phase,
                        AllocationPhase::WaitingForWater
                            | AllocationPhase::OperationalWaitingForWater
                            | AllocationPhase::Retired
                    )
            })
            .map(|r| r.water)
            .sum();
        gaps.insert(
            "land_quota".into(),
            (land_used + 1).saturating_sub(game.projection.land[&site]) as f64,
        );
        gaps.insert(
            "water_quota".into(),
            (water_used + 1).saturating_sub(game.projection.water[&source]) as f64,
        );
        let mut admission_gap = 0;
        for limit in game.admissions.values().filter(|a| {
            a.limit.source == source
                && a.limit.country == "player"
                && a.limit.start_month <= game.month
                && a.limit.end_month >= game.month
        }) {
            let used: u64 = game
                .reservations
                .values()
                .filter(|r| {
                    r.country == "player"
                        && !matches!(
                            r.phase,
                            AllocationPhase::WaitingForWater
                                | AllocationPhase::OperationalWaitingForWater
                                | AllocationPhase::Retired
                        )
                        && r.source == source
                        && !limit.grandfathered.contains(&r.id)
                })
                .map(|r| r.water)
                .sum();
            admission_gap =
                admission_gap.max((used + 1).saturating_sub(limit.limit.new_water_slots));
        }
        gaps.insert("admission_quota".into(), admission_gap as f64);
        let conservative_sales = available_orders
            .min(output)
            .min(s.road_capacity.saturating_sub(allocated_workers) as f64)
            .min(s.power_capacity.saturating_sub(allocated_workers) as f64);
        let energy_price = if self
            .energy_long_start
            .is_some_and(|start| game.month < start + 24)
        {
            param(doc, "long_energy_price")?
        } else if self.energy_contract_start.is_some() {
            let demand = param(doc, "new_factory_energy")?
                * (1 + self
                    .factories
                    .values()
                    .filter(|f| f.phase != FactoryPhase::Closed)
                    .count()) as f64;
            let share = (self.calibration.ordinary_energy_import_limit / demand).min(1.0);
            share * param(doc, "ordinary_energy_price")?
                + (1.0 - share) * param(doc, "maritime_energy_price")?
        } else {
            param(doc, "maritime_energy_price")?
        };
        let projected_wages = workers as f64
            * param(doc, "monthly_wage")?
            * if self.labor_protection {
                self.calibration.labor_protection_wage_multiplier
            } else {
                1.0
            };
        let profit = conservative_sales
            * param(doc, "goods_price")?
            * (1.0 - param(doc, "manufacturing_sales_tax_rate")?)
            - projected_wages
            - param(doc, "factory_other_fixed_monthly_cost")?
            - conservative_sales / output * param(doc, "new_factory_energy")? * energy_price;
        let forecast_monthly_profit = if profit >= 0.0 {
            money(profit)? as i128
        } else {
            -(money(-profit)? as i128)
        };
        let self_capital = currency(capital.saturating_sub(s.subsidy_cap.min(cash_gap)));
        let static_payback_months = if profit > 0.0 {
            Some(self_capital / profit)
        } else {
            None
        };
        gaps.insert("profit".into(), (-profit).max(0.0));
        gaps.insert(
            "payback_months".into(),
            static_payback_months
                .map(|months| (months - self.calibration.maximum_payback_months).max(0.0))
                .unwrap_or(self.calibration.maximum_payback_months),
        );
        let conditional_ready = gaps.values().all(|v| *v == 0.0);
        let actual_ready = actual.idle_workers >= workers
            && actual.skilled_workers >= self.calibration.factory_skill_workers
            && actual.road_capacity as f64 >= output + allocated_output
            && actual.power_capacity as f64 >= output + allocated_output
            && actual.water_access;
        Ok(Scorecard {
            group,
            site,
            factory_capital: capital,
            cash_gap,
            gaps,
            promised_projects,
            autonomous_ready: conditional_ready && cash_gap == 0 && actual_ready,
            conditional_ready,
            recognizes_promises_after_month: recognizes,
            six_month_sales_forecast: vec![conservative_sales; 6],
            forecast_monthly_profit,
            static_payback_months,
        })
    }
    pub(crate) fn stage(&mut self, game: &mut Game, stage: Stage) -> Result<()> {
        match stage {
            Stage::CommandsEscrow => {
                self.apply_plans(game)?;
                self.prepay_long_energy(game)?;
            }
            Stage::SeasonAnnouncementsAi => self.announcements(game),
            Stage::OpeningCapacity => self.activate(game)?,
            Stage::Allocation => {
                self.migrate(game);
                self.enterprise_decisions(game)?;
            }
            Stage::ProductionConstruction => {
                self.construction(game)?;
                self.production(game)?;
            }
            Stage::LivelihoodTraining => self.train(game),
            Stage::DiplomacySecurity => self.diplomacy(game)?,
            Stage::TaxesLongTerm => self.economy(game)?,
            Stage::Archive => self.archive(game)?,
        }
        Ok(())
    }
    fn announcements(&mut self, game: &Game) {
        let m = game.month;
        if m == 2 && self.cooperation_start.is_none() {
            self.todo(
                "maritime/proposal".into(),
                TodoKind::ContractProposal,
                m,
                m + 1,
                "not_signed",
                &["sign", "decline"],
                vec!["scenario_month_2".into()],
            );
        }
        if m == 3 && self.energy_contract_start.is_none() {
            self.todo(
                "resource/proposal".into(),
                TodoKind::ContractProposal,
                m,
                m + 1,
                "not_signed",
                &["sign", "decline"],
                vec!["scenario_month_3".into()],
            );
        }
        if m == 9 && self.renewal.is_none() {
            self.todo(
                "aid/renewal".into(),
                TodoKind::Renewal,
                m,
                12,
                "aid_expires_month_13",
                &["water_priority", "energy_long_term", "decline"],
                vec!["transition_aid_expires_month_13".into()],
            );
        }
        if m == 12 && self.renewal.is_none() {
            self.fact(
                m,
                "aid_renewal_deadline_this_month",
                vec!["aid/renewal".into()],
            );
        }
        let treasury = game.balance("treasury", &Asset::Money).unwrap_or(0);
        if treasury < self.calibration.protected_treasury {
            self.todo(
                format!("funding/{m}"),
                TodoKind::Funding,
                m,
                m,
                "reduce_by_spending_priority",
                &["adjust_priority", "pause_project"],
                vec!["treasury_below_protected_threshold".into()],
            );
        }
        let risks: Vec<_> = self
            .commitments
            .values()
            .filter(|c| c.status == CommitmentStatus::Pending)
            .filter(|c| {
                c.required_projects.iter().any(|id| {
                    let p = &self.projects[id];
                    !p.active
                        && (p.paused || m + p.construction_months - p.worked_months > c.deadline)
                })
            })
            .map(|c| c.id.clone())
            .collect();
        for id in risks {
            let due = self.commitments[&id].deadline;
            self.todo(
                format!("delay/{id}"),
                TodoKind::PromiseDelay,
                m,
                due,
                "breach_on_deadline",
                &["expedite_with_normal_commands", "accept_breach"],
                vec![id],
            );
        }
    }
    fn operational_water_order(&self, game: &Game, source: u32) -> Vec<(u32, String)> {
        let mut order = vec![];
        for (id, p) in &self.projects {
            let reservation = format!("public/{id}");
            if p.kind == PublicWork::Housing
                && p.active
                && game
                    .reservations
                    .get(&reservation)
                    .is_some_and(|r| r.source == source)
                && let Some(month) = p.first_operated_month
            {
                order.push((month, reservation));
            }
        }
        for (id, f) in &self.factories {
            if f.phase != FactoryPhase::Closed
                && game
                    .reservations
                    .get(id)
                    .is_some_and(|r| r.source == source)
                && let Some(month) = f.first_operated_month
            {
                order.push((month, id.clone()));
            }
        }
        order.sort();
        order
    }
    fn limit_operating_water(
        &mut self,
        game: &mut Game,
        source: u32,
        admission: u64,
        priority: u64,
    ) -> Result<()> {
        let order = self.operational_water_order(game, source);
        let ids: BTreeSet<_> = order.iter().map(|(_, id)| id.clone()).collect();
        let other: u64 = game
            .reservations
            .values()
            .filter(|r| {
                r.source == source
                    && !ids.contains(&r.id)
                    && !matches!(
                        r.phase,
                        AllocationPhase::WaitingForWater
                            | AllocationPhase::OperationalWaitingForWater
                            | AllocationPhase::Retired
                    )
            })
            .map(|r| r.water)
            .sum();
        let total = game.projection.water[&source];
        let resource = priority.min(total.saturating_sub(other));
        let quota = admission.min(total.saturating_sub(other).saturating_sub(resource));
        let mut used: u64 = order
            .iter()
            .filter_map(|(_, id)| game.reservations.get(id))
            .filter(|r| r.phase == AllocationPhase::Operational)
            .map(|r| r.water)
            .sum();
        for (_, id) in order.into_iter().rev() {
            if used <= quota {
                break;
            }
            if game.reservations[&id].phase != AllocationPhase::Operational {
                continue;
            }
            let water = game.reservations[&id].water;
            game.apply_internal(
                format!("water/suspend/{id}/{}", game.month),
                Command::SetOperationalWaterWaiting {
                    reservation: id.clone(),
                    waiting: true,
                },
            )?;
            if let Some(project) = id
                .strip_prefix("public/")
                .and_then(|id| self.projects.get_mut(id))
            {
                project.water_suspended = true;
                let site = self.sites.get_mut(&project.site).unwrap();
                site.housing_capacity = site
                    .housing_capacity
                    .checked_sub(self.calibration.housing_workers)
                    .ok_or(Error::Invalid("housing suspension capacity"))?;
            } else {
                self.factories
                    .get_mut(&id)
                    .ok_or(Error::Missing("water suspended factory"))?
                    .water_suspended = true;
            }
            used -= water;
            self.fact(
                game.month,
                "water_priority_suspends_post_initial_facility",
                vec![id, "aid/renewal".into()],
            );
        }
        Ok(())
    }
    fn resume_operating_water(&mut self, game: &mut Game) -> Result<()> {
        let mut order = vec![];
        for source in game.projection.water.keys().copied() {
            order.extend(self.operational_water_order(game, source));
        }
        order.sort();
        for (_, id) in order {
            if game.reservations[&id].phase != AllocationPhase::OperationalWaitingForWater {
                continue;
            }
            match game.apply_internal(
                format!("water/operational/resume/{id}/{}", game.month),
                Command::SetOperationalWaterWaiting {
                    reservation: id.clone(),
                    waiting: false,
                },
            ) {
                Ok(_) => {}
                Err(Error::CapacityShortfalls { .. }) => continue,
                Err(error) => return Err(error),
            }
            if let Some(project) = id
                .strip_prefix("public/")
                .and_then(|id| self.projects.get_mut(id))
            {
                project.water_suspended = false;
                let site = self.sites.get_mut(&project.site).unwrap();
                site.housing_capacity = site
                    .housing_capacity
                    .checked_add(self.calibration.housing_workers)
                    .ok_or(Error::Overflow)?;
            } else {
                self.factories.get_mut(&id).unwrap().water_suspended = false;
            }
            self.fact(
                game.month,
                "previously_operational_facility_resumes_water",
                vec![id, "aid/renewal".into()],
            );
        }
        Ok(())
    }
    fn water_ready(&mut self, game: &mut Game, id: &str) -> Result<bool> {
        if game
            .reservations
            .get(id)
            .is_some_and(|r| r.phase == AllocationPhase::WaitingForWater)
        {
            match game.apply_internal(
                format!("water/resume/{id}/{}", game.month),
                Command::SetWaterWaiting {
                    reservation: id.into(),
                    waiting: false,
                },
            ) {
                Ok(_) => {}
                Err(Error::CapacityShortfalls { .. }) => {
                    self.fact(
                        game.month,
                        "completed_facility_waits_for_water",
                        vec![id.into(), "aid/renewal".into()],
                    );
                    return Ok(false);
                }
                Err(error) => return Err(error),
            }
        }
        Ok(true)
    }
    fn activate(&mut self, game: &mut Game) -> Result<()> {
        if game.month == 25 {
            let ids: Vec<_> = game
                .reservations
                .keys()
                .filter(|id| id.starts_with("resource-priority/"))
                .cloned()
                .collect();
            for id in ids {
                game.apply_internal(format!("expire/{id}"), Command::Release { reservation: id })?;
            }
        }
        self.resume_operating_water(game)?;
        if self
            .pending_export_orders
            .is_some_and(|(month, _)| month <= game.month)
        {
            self.export_orders = self.pending_export_orders.take().unwrap().1;
        }
        let changes: Vec<_> = self
            .institution_changes
            .iter()
            .filter(|(_, (month, _))| *month <= game.month)
            .map(|(kind, (_, enabled))| (*kind, *enabled))
            .collect();
        for (kind, enabled) in changes {
            match kind {
                Institution::ClinicalCoverage => self.clinical_coverage_institution = enabled,
                Institution::LaborProtection => self.labor_protection = enabled,
            }
            self.institution_changes.remove(&kind);
        }
        for f in self.factories.values_mut() {
            if f.retrofit_activate_month.is_some_and(|m| m <= game.month) {
                f.retrofitted = true;
                f.retrofit_activate_month = None;
            }
        }
        let projects: Vec<_> = self
            .projects
            .values()
            .filter(|p| !p.active && p.activate_month.is_some_and(|m| m <= game.month))
            .map(|p| p.id.clone())
            .collect();
        for id in projects {
            let p = self.projects[&id].clone();
            if p.kind == PublicWork::Housing && !self.water_ready(game, &format!("public/{id}"))? {
                continue;
            }
            let mut site = self.sites[&p.site].clone();
            self.add_capacity(&mut site, p.kind)?;
            if p.kind == PublicWork::Vocational {
                if site.idle_workers == 0 {
                    self.fact(
                        game.month,
                        "vocational_school_missing_teacher",
                        vec![id.clone()],
                    );
                    continue;
                }
                site.idle_workers -= 1;
                site.skilled_workers = site.skilled_workers.min(site.idle_workers);
                site.teachers += 1;
            }
            self.sites.insert(p.site, site);
            self.projects.get_mut(&id).unwrap().active = true;
            self.projects.get_mut(&id).unwrap().first_operated_month = Some(game.month);
            if p.kind == PublicWork::Housing {
                game.apply_internal(
                    format!("operate/public/{id}"),
                    Command::MarkOperational {
                        reservation: format!("public/{id}"),
                    },
                )?;
            }
            self.fact(game.month, "public_capacity_activated", vec![id]);
        }
        let ids: Vec<_> = self
            .factories
            .values()
            .filter(|f| {
                f.phase == FactoryPhase::Construction
                    && f.worked_months == f.construction_months
                    && f.activate_month <= game.month
            })
            .map(|f| f.id.clone())
            .collect();
        for id in ids {
            if !self.water_ready(game, &id)? {
                continue;
            }
            self.factories.get_mut(&id).unwrap().phase = FactoryPhase::Operating;
            self.factories.get_mut(&id).unwrap().first_operated_month = Some(game.month);
            game.apply_internal(
                format!("operate/{id}"),
                Command::MarkOperational { reservation: id },
            )?;
        }
        Ok(())
    }
    fn migrate(&mut self, game: &Game) {
        let mut source_remaining: BTreeMap<_, _> = self
            .sites
            .iter()
            .map(|(id, s)| {
                (
                    *id,
                    (s.population as f64 * self.calibration.migration_source_population_share)
                        .floor() as u32,
                )
            })
            .collect();
        let mut destinations: Vec<_> = self
            .sites
            .iter()
            .filter(|(_, s)| s.admitted)
            .map(|(id, _)| *id)
            .collect();
        destinations.sort_by_key(|id| (Some(*id) != self.selected_candidate, *id));
        for site in destinations {
            if [39, 74].contains(&site) {
                continue;
            }
            let s = &self.sites[&site];
            if s.road_capacity == 0 || s.power_capacity == 0 || !s.water_access {
                continue;
            }
            let target = s.road_capacity.min(s.power_capacity);
            let allocated: u32 = self
                .factories
                .values()
                .filter(|f| f.site == site && f.phase != FactoryPhase::Closed)
                .map(|f| f.workers)
                .sum();
            let capacity = s
                .housing_capacity
                .saturating_sub(s.population)
                .min(s.clinic_capacity.saturating_sub(s.population));
            let mut need = target
                .saturating_sub(s.idle_workers + allocated)
                .min(capacity)
                .min(self.calibration.domestic_migration_per_month);
            let origins: Vec<_> = self
                .sites
                .keys()
                .copied()
                .filter(|id| *id != site)
                .collect();
            for origin in origins {
                let source = self.sites.get_mut(&origin).unwrap();
                let moved = source.idle_workers.min(need).min(source_remaining[&origin]);
                let skilled_moved = moved
                    .saturating_sub(source.idle_workers.saturating_sub(source.skilled_workers));
                source.skilled_workers -= skilled_moved;
                *source_remaining.get_mut(&origin).unwrap() -= moved;
                source.idle_workers -= moved;
                source.enrolled_students = source
                    .enrolled_students
                    .min(source.idle_workers.saturating_sub(source.skilled_workers));
                source.population -= moved;
                let dest = self.sites.get_mut(&site).unwrap();
                dest.idle_workers += moved;
                dest.skilled_workers += skilled_moved;
                dest.population += moved;
                need -= moved;
                if moved > 0 {
                    self.fact(
                        game.month,
                        format!("domestic_idle_migration:{moved}"),
                        vec![format!("site/{origin}"), format!("site/{site}")],
                    );
                }
                if need == 0 {
                    break;
                }
            }
        }
    }
    fn train(&mut self, _game: &Game) {
        let education_active = self.budgets[&Service::Education] > 0;
        let mut foreign_slots = if self.cooperation_start.is_some()
            && self.cooperation_months > 0
            && self.cooperation_months.is_multiple_of(3)
        {
            self.calibration.cooperation_training_per_quarter
        } else {
            0
        };
        let mut sites: Vec<_> = self.sites.keys().copied().collect();
        sites.sort_by_key(|id| (Some(*id) != self.selected_candidate, *id));
        for id in sites {
            let site = self.sites.get_mut(&id).unwrap();
            let ordinary = site.idle_workers.saturating_sub(site.skilled_workers);
            if education_active && site.teachers > 0 && site.vocational_schools > 0 {
                if site.training_months == 0 {
                    site.enrolled_students = ordinary.min(
                        site.vocational_schools * self.calibration.trained_workers_per_quarter,
                    );
                }
                site.enrolled_students = site.enrolled_students.min(ordinary);
                site.training_months += 1;
                if site.training_months == 3 {
                    site.skilled_workers += site.enrolled_students;
                    site.enrolled_students = 0;
                    site.training_months = 0;
                }
            }
            let foreign = foreign_slots.min(
                site.idle_workers
                    .saturating_sub(site.skilled_workers)
                    .saturating_sub(site.enrolled_students),
            );
            site.skilled_workers += foreign;
            foreign_slots -= foreign;
        }
    }
    fn enterprise_decisions(&mut self, game: &mut Game) -> Result<()> {
        let pending: Vec<_> = self
            .commitments
            .values()
            .filter(|c| c.status == CommitmentStatus::Pending)
            .map(|c| c.id.clone())
            .collect();
        for id in pending {
            let c = self.commitments[&id].clone();
            if c.required_projects
                .iter()
                .all(|id| self.projects[id].active)
            {
                let card = self.assess_excluding_commitment(game, c.group, c.site, Some(&id))?;
                let actual = &self.sites[&c.site];
                if card.gaps.values().all(|g| *g == 0.0)
                    && actual.idle_workers >= 100
                    && actual.skilled_workers >= self.calibration.factory_skill_workers
                {
                    match self.start_factory(
                        game,
                        c.group,
                        c.site,
                        c.subsidy,
                        c.escrow.clone(),
                        vec![id.clone()],
                    ) {
                        Ok(()) => {
                            self.commitments.get_mut(&id).unwrap().status =
                                CommitmentStatus::Fulfilled
                        }
                        Err(Error::Insufficient(_)) | Err(Error::CapacityShortfalls { .. }) => {}
                        Err(e) => return Err(e),
                    }
                }
            }
        }
        for group in [Group::A, Group::B] {
            if self
                .factories
                .values()
                .any(|f| f.group == group && f.phase == FactoryPhase::Construction)
                || self
                    .commitments
                    .values()
                    .any(|c| c.group == group && c.status == CommitmentStatus::Pending)
            {
                continue;
            }
            let mut cards = vec![];
            for site in self.sites.keys().copied() {
                cards.push(self.assess(game, group, site)?);
            }
            cards.sort_by(investment_preference);
            if let Some(card) = cards.iter().find(|c| c.autonomous_ready) {
                match self.start_factory(
                    game,
                    group,
                    card.site,
                    0,
                    None,
                    vec!["autonomous_company_decision".into()],
                ) {
                    Ok(()) => continue,
                    Err(Error::Insufficient(_)) | Err(Error::CapacityShortfalls { .. }) => {}
                    Err(e) => return Err(e),
                }
            }
            for card in cards
                .iter()
                .filter(|c| c.conditional_ready && !c.autonomous_ready)
            {
                let key = format!("{group:?}/{}", card.site);
                if self.dismissed_intents.contains(&key)
                    || self
                        .intents
                        .values()
                        .any(|i| i.group == group && i.site == card.site)
                {
                    continue;
                }
                if !self.sites[&card.site].admitted {
                    continue;
                }
                let id = format!(
                    "intent/{group:?}/{}/{}/{}",
                    card.site, game.month, self.next_factory_id
                );
                let intent = InvestmentIntent {
                    id: id.clone(),
                    group,
                    site: card.site,
                    created_month: game.month,
                    due_month: game.month + 1,
                    subsidy: card.cash_gap,
                    required_projects: card.promised_projects.clone(),
                };
                self.intents.insert(id.clone(), intent);
                self.todo(
                    id.clone(),
                    TodoKind::Intent,
                    game.month,
                    game.month + 1,
                    "investment_rejected",
                    &["accept", "counteroffer_by_subsidy_cap", "reject"],
                    vec![key],
                );
            }
        }
        Ok(())
    }
    fn start_factory(
        &mut self,
        game: &mut Game,
        group: Group,
        site: u32,
        subsidy: u64,
        subsidy_escrow: Option<String>,
        causes: Vec<String>,
    ) -> Result<()> {
        let doc = document()?;
        let card = self.assess(game, group, site)?;
        let actual = &self.sites[&site];
        let used: u32 = self
            .factories
            .values()
            .filter(|f| f.site == site && f.phase != FactoryPhase::Closed)
            .map(|f| f.workers)
            .sum();
        if !actual.admitted
            || !actual.water_access
            || actual.road_capacity < used + 100
            || actual.power_capacity < used + 100
            || actual.idle_workers < 100
            || actual.skilled_workers < self.calibration.factory_skill_workers
        {
            return Err(Error::Insufficient("actual factory facilities and labor"));
        }
        if card.gaps.get("land_quota").copied().unwrap_or(0.0) > 0.0
            || card.gaps.get("water_quota").copied().unwrap_or(0.0) > 0.0
            || card.gaps.get("admission_quota").copied().unwrap_or(0.0) > 0.0
        {
            return Err(Error::Insufficient("factory quota"));
        }
        let id = format!("factory/{}", self.next_factory_id);
        let capital = card.factory_capital;
        let own = capital
            .checked_sub(subsidy)
            .ok_or(Error::Invalid("subsidy exceeds capital"))?;
        let balance = game.balance(group.account(), &Asset::Money).unwrap_or(0);
        if balance < own + money(param(doc, "operating_cash_buffer")?)? {
            return Err(Error::Insufficient("group capital and operating buffer"));
        }
        let materials = number(doc, "/materials/reference_project_total/value")? as u64;
        if game
            .balance("capital_inventory", &Asset::Material("construction".into()))
            .unwrap_or(0)
            < materials
        {
            return Err(Error::Insufficient("factory construction materials"));
        }
        // All fallible resource checks precede mutations in this batch.

        self.reserve(game, &id, site, LandUse::Factory)?;
        let escrow = format!("capital/{id}");
        if own > 0 {
            game.apply_internal(
                format!("fund/{id}"),
                Command::FundEscrow {
                    escrow: escrow.clone(),
                    payer: group.account().into(),
                    payee: "construction".into(),
                    amount: own,
                },
            )?;
        }
        game.apply_internal(
            format!("materials/{id}"),
            Command::Transfer {
                from: "capital_inventory".into(),
                to: "construction".into(),
                asset: Asset::Material("construction".into()),
                amount: materials,
            },
        )?;
        let s = self.sites.get_mut(&site).unwrap();
        let workers = param(doc, "new_factory_workers")? as u32;
        let assigned_skills = workers
            .saturating_sub(s.idle_workers.saturating_sub(s.skilled_workers))
            .max(self.calibration.factory_skill_workers);
        s.idle_workers = s
            .idle_workers
            .checked_sub(workers)
            .ok_or(Error::Insufficient("factory labor"))?;
        s.skilled_workers = s
            .skilled_workers
            .checked_sub(assigned_skills)
            .ok_or(Error::Insufficient("factory skills"))?;
        s.enrolled_students = s
            .enrolled_students
            .min(s.idle_workers.saturating_sub(s.skilled_workers));
        let months = param(doc, "d021_factory_construction_months")? as u32;
        self.factories.insert(
            id.clone(),
            Factory {
                id: id.clone(),
                group,
                site,
                phase: FactoryPhase::Construction,
                water_suspended: false,
                first_operated_month: None,
                started_month: game.month,
                activate_month: game.month + months,
                construction_months: months,
                worked_months: 0,
                capital,
                subsidy,
                subsidy_escrow,
                capital_escrow: if own > 0 { escrow } else { String::new() },
                reserved_materials: materials,
                loss_months: 0,
                close_month: None,
                workers,
                skilled_workers: assigned_skills,
                output: 0.0,
                domestic_sales: 0.0,
                export_sales: 0.0,
                ordinary_energy_used: 0.0,
                maritime_energy_used: 0.0,
                after_tax_profit: 0,
                operating_subsidy: 0,
                retrofitted: false,
                retrofit_activate_month: None,
            },
        );
        self.next_factory_id += 1;
        self.fact(
            game.month,
            "enterprise_construction_started",
            [causes, vec![id, format!("site/{site}")]].concat(),
        );
        Ok(())
    }
    fn construction(&mut self, game: &mut Game) -> Result<()> {
        let ids: Vec<_> = self
            .projects
            .values()
            .filter(|p| !p.active && !p.paused && p.activate_month.is_none())
            .map(|p| p.id.clone())
            .collect();
        for id in ids {
            let p = self.projects.get_mut(&id).unwrap();
            let advance = (1 + p.expedite_months).min(p.construction_months - p.worked_months);
            p.expedite_months = 0;
            p.worked_months += advance;
            let progress = (p.worked_months * 10_000 / p.construction_months) as u16;
            game.apply_internal(
                format!("progress/public/{id}/{}", game.month),
                Command::ProgressEscrow {
                    escrow: p.escrow.clone(),
                    basis_points: progress,
                },
            )?;
            game.settle_external(
                "construction",
                Asset::Material("construction".into()),
                -(2 * advance as i128),
                "public_construction_consumption",
            )?;
            p.reserved_materials -= 2 * advance as u64;
            if p.worked_months == p.construction_months {
                p.activate_month = Some(game.month + 1);
            }
        }
        for f in self.factories.values_mut().filter(|f| {
            f.phase == FactoryPhase::Construction && f.worked_months < f.construction_months
        }) {
            f.worked_months += 1;
            let progress = (f.worked_months * 10_000 / f.construction_months) as u16;
            if !f.capital_escrow.is_empty() {
                game.apply_internal(
                    format!("progress/capital/{}/{}", f.id, game.month),
                    Command::ProgressEscrow {
                        escrow: f.capital_escrow.clone(),
                        basis_points: progress,
                    },
                )?;
            }
            if let Some(escrow) = &f.subsidy_escrow {
                game.apply_internal(
                    format!("progress/subsidy/{}/{}", f.id, game.month),
                    Command::ProgressEscrow {
                        escrow: escrow.clone(),
                        basis_points: progress,
                    },
                )?;
            }
            let amount = if f.worked_months == f.construction_months {
                f.reserved_materials
            } else {
                4.min(f.reserved_materials)
            };
            game.settle_external(
                "construction",
                Asset::Material("construction".into()),
                -(amount as i128),
                "factory_construction_consumption",
            )?;
            f.reserved_materials -= amount;
        }
        let doc = document()?;
        game.settle_external(
            "capital_inventory",
            Asset::Material("construction".into()),
            number(doc, "/materials/capital_monthly_output/value")? as i128,
            "capital_material_production",
        )?;
        game.settle_external(
            "capital_inventory",
            Asset::Material("construction".into()),
            number(doc, "/materials/monthly_imports/value")? as i128,
            "background_material_import_paid_in_project_capital",
        )?;
        game.settle_external(
            "households",
            Asset::Material("food".into()),
            number(doc, "/food/background_imports/value")? as i128,
            "household_background_food_import",
        )?;
        game.settle_external(
            "households",
            Asset::Material("food".into()),
            -(number(doc, "/food/monthly_consumption/value")? as i128),
            "household_food_consumption",
        )?;
        Ok(())
    }
    fn diplomacy(&mut self, game: &mut Game) -> Result<()> {
        self.last_diplomatic_spend = 0;
        if self.cooperation_start.is_some() {
            let fee = self.calibration.cooperation_monthly_fee;
            let cash = game.balance("treasury", &Asset::Money).unwrap_or(0);
            if cash >= fee {
                game.settle_external(
                    "treasury",
                    Asset::Money,
                    -(fee as i128),
                    "market_training_cooperation_maintenance",
                )?;
                self.last_diplomatic_spend = fee;
            } else {
                self.cooperation_risk_since.get_or_insert(game.month);
            }
        }
        if self.cooperation_start.is_some() {
            if self.security < 50.0
                || self.last_diplomatic_spend < self.calibration.cooperation_monthly_fee
            {
                let since = *self.cooperation_risk_since.get_or_insert(game.month);
                self.todo(
                    "cooperation/risk".into(),
                    TodoKind::ContractRisk,
                    game.month,
                    since + 1,
                    "cooperation_suspended",
                    &["restore_security", "accept_suspension"],
                    vec!["security_below_50".into()],
                );
                if game.month > since {
                    self.cooperation_start = None;
                    self.export_orders = 0.0;
                    self.cooperation_months = 0;
                    self.fact(
                        game.month,
                        "cooperation_suspended",
                        vec!["cooperation/risk".into()],
                    );
                }
            } else {
                self.cooperation_risk_since = None;
                if let Some(t) = self.todos.get_mut("cooperation/risk") {
                    t.resolved = true;
                }
                self.cooperation_months += 1;
                if self.cooperation_months >= self.calibration.cooperation_delay_months {
                    self.export_orders =
                        self.calibration.cooperation_export_orders_per_month as f64;
                }
            }
        }
        Ok(())
    }
    fn prepay_long_energy(&mut self, game: &mut Game) -> Result<()> {
        self.current_energy_contract_paid = 0;
        if self
            .energy_long_start
            .is_some_and(|start| game.month < start + 24)
        {
            let due = game.obligations["resource_energy_long_term"]
                .terms
                .monthly_due()?;
            let paid = due.min(game.balance("treasury", &Asset::Money).unwrap_or(0));
            if paid > 0 {
                game.apply_internal(
                    format!("E/payment/{}", game.month),
                    Command::PayObligation {
                        obligation: "resource_energy_long_term".into(),
                        due_month: game.month,
                        amount: paid,
                    },
                )?;
            }
            self.current_energy_contract_paid = paid;
            if paid < due {
                self.aid_breached = true;
                self.fact(
                    game.month,
                    "energy_long_term_payment_breach_aid_stops",
                    vec!["aid/renewal".into()],
                );
            }
        }
        Ok(())
    }
    fn production(&mut self, game: &mut Game) -> Result<()> {
        let doc = document()?;
        let month = game.month;
        self.pending_taxes.clear();
        for factory in self.factories.values_mut() {
            factory.domestic_sales = 0.0;
            factory.export_sales = 0.0;
        }
        let mut pending = vec![];
        for (due, group, quantity) in std::mem::take(&mut self.ordinary_energy_deliveries) {
            if due == month {
                game.settle_external(
                    group.account(),
                    Asset::Material("ordinary_energy".into()),
                    quantity as i128,
                    "ordinary_energy_delivery",
                )?;
            } else {
                pending.push((due, group, quantity));
            }
        }
        self.ordinary_energy_deliveries = pending;
        let price = param(doc, "goods_price")?;
        let baseline_wages = number(doc, "/demand/wage_income")?;
        let factory_units = 4.25;
        let existing_factory_wages =
            factory_units * param(doc, "new_factory_workers")? * param(doc, "monthly_wage")?;
        game.settle_external(
            "households",
            Asset::Money,
            money(baseline_wages - existing_factory_wages)? as i128,
            "existing_nonmanufacturing_employer_wages",
        )?;
        self.monthly_existing_payroll =
            money((baseline_wages - existing_factory_wages) * param(doc, "payroll_tax_rate")?)?;
        for (group, name) in [(Group::A, "group_a"), (Group::B, "group_b")] {
            let output = number(
                doc,
                &format!("/d021_existing_ownership_and_retention/groups/{name}/existing_output"),
            )?;
            let units = output / param(doc, "new_factory_output")?;
            let sales = money(output * price)?;
            game.apply_internal(
                format!("existing/sales/{group:?}/{month}"),
                Command::Transfer {
                    from: "households".into(),
                    to: group.account().into(),
                    asset: Asset::Money,
                    amount: sales,
                },
            )?;
            game.settle_external(
                group.account(),
                Asset::Material("manufactured".into()),
                money(output)? as i128,
                "existing_manufacturing_production",
            )?;
            game.settle_external(
                group.account(),
                Asset::Material("manufactured".into()),
                -(money(output)? as i128),
                "existing_household_goods_delivery",
            )?;
            let wage = money(
                units
                    * param(doc, "new_factory_workers")?
                    * param(doc, "monthly_wage")?
                    * if self.labor_protection {
                        self.calibration.labor_protection_wage_multiplier
                    } else {
                        1.0
                    },
            )?;
            self.monthly_existing_payroll = self
                .monthly_existing_payroll
                .checked_add(money(currency(wage) * param(doc, "payroll_tax_rate")?)?)
                .ok_or(Error::Overflow)?;
            game.apply_internal(
                format!("existing/wages/{group:?}/{month}"),
                Command::Transfer {
                    from: group.account().into(),
                    to: "households".into(),
                    asset: Asset::Money,
                    amount: wage,
                },
            )?;
            let sales_tax = money(currency(sales) * param(doc, "manufacturing_sales_tax_rate")?)?;
            let maintenance = money(units * param(doc, "factory_other_fixed_monthly_cost")?)?;
            let energy = money(
                units * param(doc, "new_factory_energy")? * param(doc, "ordinary_energy_price")?,
            )?;
            game.apply_internal(
                format!("existing/energy/{group:?}/{month}"),
                Command::Transfer {
                    from: group.account().into(),
                    to: "resource_supplier".into(),
                    asset: Asset::Money,
                    amount: energy,
                },
            )?;
            game.settle_external(
                group.account(),
                Asset::Money,
                -(maintenance as i128),
                "existing_manufacturing_maintenance",
            )?;
            let surplus = sales - wage - sales_tax - maintenance - energy;
            let retained = if !self.labor_protection {
                money(number(
                    doc,
                    &format!(
                        "/d021_existing_ownership_and_retention/groups/{name}/monthly_existing_retained_income"
                    ),
                )?)?
            } else {
                money(currency(surplus) * param(doc, "group_profit_retention_share")?)?
            };
            game.settle_external(
                group.account(),
                Asset::Money,
                -((surplus - retained) as i128),
                "existing_outside_shareholder_distribution",
            )?;
            self.pending_taxes
                .push((format!("existing/{group:?}"), group, sales_tax, 0));
        }
        let import_budget = self.calibration.foreign_buyer_monthly_budget;
        if self.export_orders > 0.0 {
            game.settle_external(
                "foreign_buyer",
                Asset::Money,
                import_budget as i128,
                "foreign_contract_buyer_budget",
            )?;
        }
        let mut domestic_remaining = number(doc, "/demand/initial_imports/value")?;
        let household_ceiling = number(doc, "/demand/payable_demand_ceiling/value")?;
        domestic_remaining = domestic_remaining.min((household_ceiling - 425.0).max(0.0));
        let mut export_remaining = self
            .export_orders
            .min(currency(self.calibration.foreign_buyer_monthly_budget) / price);
        let mut road_remaining: BTreeMap<_, _> = self
            .sites
            .iter()
            .map(|(id, s)| (*id, s.road_capacity as f64))
            .collect();
        let mut power_remaining: BTreeMap<_, _> = self
            .sites
            .iter()
            .map(|(id, s)| (*id, s.power_capacity as f64))
            .collect();
        let long_active = self
            .energy_long_start
            .is_some_and(|start| month < start + 24);
        let total_energy_available = if long_active {
            (currency(self.current_energy_contract_paid) / param(doc, "long_energy_price")?)
                .min(number(doc, "/energy/renewal_quantity")?)
        } else {
            0.0
        };
        let mut energy_used = 0.0;
        let mut close = vec![];
        let ids: Vec<_> = self
            .factories
            .values()
            .filter(|f| !matches!(f.phase, FactoryPhase::Construction | FactoryPhase::Closed))
            .map(|f| f.id.clone())
            .collect();
        let mut group_demand = BTreeMap::<Group, f64>::new();
        let mut plant_demand = BTreeMap::new();
        for id in &ids {
            let f = &self.factories[id];
            let demand = if f.water_suspended {
                0.0
            } else {
                param(doc, "new_factory_energy")?
            } * if f.retrofitted {
                1.0 + self.calibration.retrofit_output_increase
            } else {
                1.0
            };
            *group_demand.entry(f.group).or_default() += demand;
            plant_demand.insert(id.clone(), demand);
        }
        let plant_demands: BTreeMap<_, _> = plant_demand
            .iter()
            .map(|(id, need)| Ok((id.clone(), money(*need)?)))
            .collect::<Result<_>>()?;
        let long_allocations = apportion(&plant_demands, money(total_energy_available)?)?;
        let mut ordinary_allocations = BTreeMap::new();
        for group in group_demand.keys() {
            let demands = plant_demands
                .iter()
                .filter(|(id, _)| self.factories[*id].group == *group)
                .map(|(id, need)| (id.clone(), *need))
                .collect();
            let stock = game
                .balance(group.account(), &Asset::Material("ordinary_energy".into()))
                .unwrap_or(0);
            ordinary_allocations.extend(apportion(&demands, stock)?);
        }
        // Assign domestic orders before physical production. First actual operation
        // wins; equal-month activations retain their stable ID order. Unfilled
        // domestic orders are not reassigned within the month. Exports retain the
        // existing ID-ordered allocation from the actual remaining buyer budget.
        let mut order_ids = ids.clone();
        order_ids.sort_by_key(|id| (self.factories[id].first_operated_month, id.clone()));
        let mut domestic_unassigned = domestic_remaining;
        let mut orders = BTreeMap::new();
        for id in order_ids {
            let f = &self.factories[&id];
            let capacity = if f.water_suspended {
                0.0
            } else {
                param(doc, "new_factory_output")?
                    * if f.retrofitted {
                        1.0 + self.calibration.retrofit_output_increase
                    } else {
                        1.0
                    }
            };
            let domestic = capacity.min(domestic_unassigned);
            domestic_unassigned -= domestic;
            orders.insert(id, domestic);
        }
        // Keep the original ID order and deduct road/power only for actual sales.
        for id in ids {
            let f = self.factories.get_mut(&id).unwrap();
            let s = &self.sites[&f.site];
            let full_output = param(doc, "new_factory_output")?
                * if f.retrofitted {
                    1.0 + self.calibration.retrofit_output_increase
                } else {
                    1.0
                };
            let current_energy_price = if long_active {
                param(doc, "long_energy_price")?
            } else {
                let need = full_output / param(doc, "new_factory_output")?
                    * param(doc, "new_factory_energy")?;
                let ordinary =
                    currency(ordinary_allocations.get(&id).copied().unwrap_or(0)).min(need);
                (ordinary * param(doc, "ordinary_energy_price")?
                    + (need - ordinary) * param(doc, "maritime_energy_price")?)
                    / need
            };
            let current_wage = param(doc, "new_factory_workers")?
                * param(doc, "monthly_wage")?
                * if self.labor_protection {
                    self.calibration.labor_protection_wage_multiplier
                } else {
                    1.0
                };
            let full_profit = full_output
                * param(doc, "goods_price")?
                * (1.0 - param(doc, "manufacturing_sales_tax_rate")?)
                - current_wage
                - param(doc, "factory_other_fixed_monthly_cost")?
                - full_output / param(doc, "new_factory_output")?
                    * param(doc, "new_factory_energy")?
                    * current_energy_price;
            let allocated_energy = if long_active {
                currency(long_allocations.get(&id).copied().unwrap_or(0))
            } else {
                plant_demand[&id]
            };
            let domestic_order = orders[&id];
            let restored = domestic_order + export_remaining >= full_output
                && road_remaining[&f.site] >= full_output
                && power_remaining[&f.site] >= full_output
                && allocated_energy
                    >= full_output / param(doc, "new_factory_output")?
                        * param(doc, "new_factory_energy")?
                && full_profit > 0.0;
            let rate = if f.water_suspended {
                0.0
            } else if (f.phase == FactoryPhase::Reduced || f.phase == FactoryPhase::ClosureWarned)
                && !restored
            {
                0.5
            } else {
                1.0
            };
            let max_output = param(doc, "new_factory_output")?
                * rate
                * if f.retrofitted {
                    1.0 + self.calibration.retrofit_output_increase
                } else {
                    1.0
                };
            let _ = s;
            let road = road_remaining[&f.site];
            let power = power_remaining[&f.site];
            let energy_output = allocated_energy / param(doc, "new_factory_energy")?
                * param(doc, "new_factory_output")?;
            let possible = max_output.min(road).min(power).min(energy_output);
            let domestic = possible.min(domestic_order);
            domestic_remaining -= domestic;
            let exports = (possible - domestic).min(export_remaining);
            export_remaining -= exports;
            f.output = domestic + exports;
            f.domestic_sales = domestic;
            f.export_sales = exports;
            *road_remaining.get_mut(&f.site).unwrap() -= f.output;
            *power_remaining.get_mut(&f.site).unwrap() -= f.output;
            let energy =
                f.output / param(doc, "new_factory_output")? * param(doc, "new_factory_energy")?;
            energy_used += energy;
            let ordinary = if long_active {
                0.0
            } else {
                currency(ordinary_allocations.get(&id).copied().unwrap_or(0)).min(energy)
            };
            let maritime = if long_active { 0.0 } else { energy - ordinary };
            f.ordinary_energy_used = ordinary;
            f.maritime_energy_used = maritime;
            if ordinary > 0.0 {
                game.settle_external(
                    f.group.account(),
                    Asset::Material("ordinary_energy".into()),
                    -(money(ordinary)? as i128),
                    "ordinary_energy_consumption",
                )?;
            }
            if maritime > 0.0 {
                game.settle_external(
                    f.group.account(),
                    Asset::Material("maritime_energy".into()),
                    money(maritime)? as i128,
                    "maritime_energy_delivery",
                )?;
                game.settle_external(
                    f.group.account(),
                    Asset::Material("maritime_energy".into()),
                    -(money(maritime)? as i128),
                    "maritime_energy_consumption",
                )?;
            }
            let energy_cost = if long_active {
                energy * param(doc, "long_energy_price")?
            } else {
                ordinary * param(doc, "ordinary_energy_price")?
                    + maritime * param(doc, "maritime_energy_price")?
            };
            let wage = param(doc, "new_factory_workers")?
                * param(doc, "monthly_wage")?
                * if self.labor_protection {
                    self.calibration.labor_protection_wage_multiplier
                } else {
                    1.0
                };
            let sales = money(f.output * param(doc, "goods_price")?)?;
            let sales_tax = money(currency(sales) * param(doc, "manufacturing_sales_tax_rate")?)?;
            let payroll_tax = money(wage * param(doc, "payroll_tax_rate")?)?;
            let wages = money(wage)?;
            let energy_bill = money(energy_cost)?;
            let cash_energy_bill = if long_active {
                energy_bill
            } else {
                money(maritime * param(doc, "maritime_energy_price")?)?
            };
            let fixed = money(param(doc, "factory_other_fixed_monthly_cost")?)?;
            f.after_tax_profit = sales as i128
                - sales_tax as i128
                - wages as i128
                - energy_bill as i128
                - fixed as i128;
            game.settle_external(
                f.group.account(),
                Asset::Material("manufactured".into()),
                money(f.output)? as i128,
                "factory_goods_production_scaled_quantity",
            )?;
            game.settle_external(
                f.group.account(),
                Asset::Material("manufactured".into()),
                -(money(f.output)? as i128),
                "market_goods_delivery_scaled_quantity",
            )?;
            let domestic_bill = money(domestic * price)?;
            let export_bill = sales - domestic_bill;
            for (part, buyer, amount) in [
                ("domestic", "households", domestic_bill),
                ("export", "foreign_buyer", export_bill),
            ] {
                if amount > 0 {
                    game.apply_internal(
                        format!("sales/{id}/{part}/{month}"),
                        Command::Transfer {
                            from: buyer.into(),
                            to: f.group.account().into(),
                            asset: Asset::Money,
                            amount,
                        },
                    )?;
                }
            }
            let account = f.group.account();
            let mut actual_payroll = payroll_tax;
            for (reason, amount) in [
                ("factory_wages", wages),
                ("new_factory_energy", cash_energy_bill),
                ("factory_maintenance", fixed),
            ] {
                let paid = amount.min(
                    game.balance(account, &Asset::Money)
                        .unwrap_or(0)
                        .saturating_sub(sales_tax),
                );
                if reason == "factory_wages" {
                    actual_payroll = money(currency(paid) * param(doc, "payroll_tax_rate")?)?;
                }
                if reason == "factory_wages" && paid > 0 {
                    game.apply_internal(
                        format!("wage/{id}/{month}"),
                        Command::Transfer {
                            from: account.into(),
                            to: "households".into(),
                            asset: Asset::Money,
                            amount: paid,
                        },
                    )?;
                } else if reason == "new_factory_energy" && paid > 0 && long_active {
                    game.apply_internal(
                        format!("energy/{id}/{month}"),
                        Command::Transfer {
                            from: account.into(),
                            to: if long_active {
                                "treasury".into()
                            } else {
                                "resource_supplier".into()
                            },
                            asset: Asset::Money,
                            amount: paid,
                        },
                    )?;
                } else {
                    game.settle_external(account, Asset::Money, -(paid as i128), reason)?;
                }
                if paid < amount {
                    f.after_tax_profit = f.after_tax_profit.min(-1);
                    if long_active && reason == "new_factory_energy" {
                        self.aid_breached = true;
                    }
                }
            }
            self.pending_taxes
                .push((id.clone(), f.group, sales_tax, actual_payroll));
            if f.after_tax_profit > 0 {
                let distribution = money(
                    f.after_tax_profit as f64 / MONEY_SCALE as f64
                        * (1.0 - param(doc, "group_profit_retention_share")?),
                )?;
                let amount = distribution.min(game.balance(account, &Asset::Money).unwrap_or(0));
                game.settle_external(
                    account,
                    Asset::Money,
                    -(amount as i128),
                    "outside_shareholder_distribution_no_reinjection",
                )?;
                f.loss_months = 0;
                f.close_month = None;
                f.phase = FactoryPhase::Operating;
                if let Some(t) = self.todos.get_mut(&format!("closure/{id}")) {
                    t.resolved = true;
                }
            } else if f.after_tax_profit < 0 {
                f.loss_months += 1;
                if f.loss_months == 3 {
                    f.phase = FactoryPhase::Reduced;
                }
                if f.loss_months == 6 {
                    f.phase = FactoryPhase::ClosureWarned;
                    f.close_month = Some(month + 1);
                }
                if f.loss_months >= 7 && f.close_month.is_some_and(|m| m <= month) {
                    close.push(id.clone());
                }
            } else {
                f.loss_months = 0;
                f.close_month = None;
                f.phase = FactoryPhase::Operating;
            }
        }
        let warned: Vec<_> = self
            .factories
            .values()
            .filter(|f| f.loss_months == 6)
            .map(|f| f.id.clone())
            .collect();
        for id in warned {
            self.todo(
                format!("closure/{id}"),
                TodoKind::Closure,
                month,
                month + 1,
                "factory_closes_next_month",
                &["restore_normal_orders_power_roads", "allow_close"],
                vec![id],
            );
        }
        for id in close {
            let f = self.factories.get_mut(&id).unwrap();
            f.phase = FactoryPhase::Closed;
            f.water_suspended = false;
            f.output = 0.0;
            // Keep the final operating month's actual sales even when closing at month end.
            let site = self.sites.get_mut(&f.site).unwrap();
            site.idle_workers += f.workers;
            site.skilled_workers += f.skilled_workers;
            game.apply_internal(
                format!("retire/{id}"),
                Command::Retire {
                    reservation: id.clone(),
                },
            )?;
            self.fact(month, "closed_factory_retains_land", vec![id.clone()]);
            self.fact(month, "factory_closed_after_seven_loss_months", vec![id]);
        }
        let imported = money(domestic_remaining * price)?;
        game.settle_external(
            "households",
            Asset::Money,
            -(imported as i128),
            "remaining_household_manufactured_imports",
        )?;
        let food_bill = money(number(doc, "/food/monthly_import_bill")?)?;
        game.settle_external(
            "households",
            Asset::Money,
            -(food_bill as i128),
            "household_food_import_payment",
        )?;
        self.monthly_energy_used = energy_used;
        if self.energy_contract_start.is_some() && !long_active {
            let mut demand = BTreeMap::<Group, u64>::new();
            for f in self.factories.values().filter(|f| {
                f.phase != FactoryPhase::Closed
                    && !f.water_suspended
                    && f.activate_month <= month + 1
            }) {
                let need = money(
                    param(doc, "new_factory_energy")?
                        * if f.retrofitted {
                            1.0 + self.calibration.retrofit_output_increase
                        } else {
                            1.0
                        },
                )?;
                *demand.entry(f.group).or_default() += need;
            }
            for (group, need) in &mut demand {
                *need = need.saturating_sub(
                    game.balance(group.account(), &Asset::Material("ordinary_energy".into()))
                        .unwrap_or(0),
                );
                let affordable = money(
                    currency(game.balance(group.account(), &Asset::Money).unwrap_or(0))
                        / param(doc, "ordinary_energy_price")?,
                )?;
                *need = (*need).min(affordable);
            }
            let allocated = apportion(
                &demand,
                money(self.calibration.ordinary_energy_import_limit)?,
            )?;
            for (group, quantity) in allocated {
                let amount = money(currency(quantity) * param(doc, "ordinary_energy_price")?)?;
                if amount > 0 {
                    game.apply_internal(
                        format!("ordinary/prepay/{group:?}/{month}"),
                        Command::Transfer {
                            from: group.account().into(),
                            to: "resource_supplier".into(),
                            asset: Asset::Money,
                            amount,
                        },
                    )?;
                    self.ordinary_energy_deliveries
                        .push((month + 1, group, quantity));
                }
            }
        }
        Ok(())
    }
    fn economy(&mut self, game: &mut Game) -> Result<()> {
        let doc = document()?;
        let month = game.month;
        let mut recurring_income = 0i128;
        let baseline_payroll = self.monthly_existing_payroll;
        game.apply_internal(
            format!("existing/payroll/{month}"),
            Command::Transfer {
                from: "households".into(),
                to: "treasury".into(),
                asset: Asset::Money,
                amount: baseline_payroll,
            },
        )?;
        recurring_income += baseline_payroll as i128;
        for (id, group, sales_tax, payroll_tax) in std::mem::take(&mut self.pending_taxes) {
            for (part, account, amount) in [
                ("sales", group.account(), sales_tax),
                ("payroll", "households", payroll_tax),
            ] {
                let paid = amount.min(game.balance(account, &Asset::Money).unwrap_or(0));
                if paid > 0 {
                    game.apply_internal(
                        format!("tax/{id}/{part}/{month}"),
                        Command::Transfer {
                            from: account.into(),
                            to: "treasury".into(),
                            asset: Asset::Money,
                            amount: paid,
                        },
                    )?;
                }
                recurring_income += paid as i128;
            }
        }
        let energy_used = self.monthly_energy_used;
        self.last_energy_unused_bill = 0;
        if let Some(start) = self.energy_long_start
            && month < start + 24
        {
            let due = game.obligations["resource_energy_long_term"]
                .terms
                .monthly_due()?;
            let paid = self.current_energy_contract_paid;
            let resale = money(energy_used * param(doc, "long_energy_price")?)?;
            self.last_energy_unused_bill = due.saturating_sub(resale);
            recurring_income -= self.last_energy_unused_bill as i128;
            let _ = paid;
        }
        let renewed = self
            .renewal
            .is_some_and(|(_, choice)| choice != RenewalChoice::Decline);
        self.last_aid = if !self.aid_breached && (month < 13 || renewed) {
            money(param(doc, "monthly_transition_aid")?)?
        } else {
            0
        };
        game.settle_external(
            "treasury",
            Asset::Money,
            self.last_aid as i128,
            "resource_transition_aid",
        )?;
        let mut recurring_spend = 0i128;
        let mut paid_service = BTreeMap::new();
        for kind in self.spending_priority.clone() {
            let maintenance = if kind == Service::Maintenance {
                self.projects.values().filter(|p| p.active).count() as u64
                    * self.calibration.public_work_monthly_maintenance
            } else {
                0
            };
            let due = self.budgets[&kind]
                .checked_add(maintenance)
                .ok_or(Error::Overflow)?;
            let cash = game.balance("treasury", &Asset::Money).unwrap_or(0);
            let paid = due.min(cash.saturating_sub(self.calibration.protected_treasury));
            game.settle_external(
                "treasury",
                Asset::Money,
                -(paid as i128),
                &format!("recurring_service_{kind:?}"),
            )?;
            recurring_spend += paid as i128;
            paid_service.insert(kind, paid);
            if paid < due {
                self.fact(
                    month,
                    format!("service_reduced_{kind:?}"),
                    vec!["spending_priority".into()],
                );
            }
        }
        let safety_baseline = money(number(doc, "/fiscal/spending_breakdown/safety/value")?)?;
        let safety_paid = paid_service[&Service::Safety];
        let safety_ratio = safety_paid as f64 / safety_baseline as f64;
        self.security = if safety_ratio >= self.calibration.security_recovery_budget_ratio {
            (self.security + 1.0).min(70.0)
        } else if safety_paid >= safety_baseline {
            self.security
        } else if safety_ratio >= self.calibration.security_decline_budget_ratio {
            (self.security - 1.0).max(0.0)
        } else {
            (self.security - 3.0).max(0.0)
        };
        let health_baseline = money(number(doc, "/fiscal/spending_breakdown/health/value")?)?;
        self.health_funding_ratio =
            (paid_service[&Service::Health] as f64 / health_baseline as f64).min(1.0);
        self.last_recurring_balance = recurring_income + self.last_aid as i128
            - recurring_spend
            - self.last_diplomatic_spend as i128;
        Ok(())
    }
    fn archive(&mut self, game: &mut Game) -> Result<()> {
        let month = game.month;
        let overdue: Vec<_> = self
            .intents
            .values()
            .filter(|i| i.due_month <= month)
            .map(|i| i.id.clone())
            .collect();
        for id in overdue {
            let i = self.intents.remove(&id).unwrap();
            self.dismissed_intents
                .insert(format!("{:?}/{}", i.group, i.site));
            if let Some(t) = self.todos.get_mut(&id) {
                t.resolved = true;
            }
            self.fact(month, "investment_intent_expired_rejected", vec![id]);
        }
        let breaches: Vec<_> = self
            .commitments
            .values()
            .filter(|c| c.status == CommitmentStatus::Pending && c.deadline <= month)
            .map(|c| c.id.clone())
            .collect();
        for id in breaches {
            let c = self.commitments.get_mut(&id).unwrap();
            c.status = CommitmentStatus::Breached;
            self.distrust_until.insert(c.group, month + 6);
            if let Some(escrow) = c.escrow.clone() {
                game.apply_internal(
                    format!("breach/refund/{id}"),
                    Command::CancelEscrow { escrow },
                )?;
            }
            self.fact(
                month,
                "conditional_promise_breached_six_month_group_distrust",
                vec![id],
            );
        }
        for todo in self
            .todos
            .values_mut()
            .filter(|t| t.due_month <= month && !t.resolved)
        {
            todo.resolved = true;
        }
        let mut output_by_site = BTreeMap::from([(39, 300.0), (74, 125.0)]);
        for f in self
            .factories
            .values()
            .filter(|f| !matches!(f.phase, FactoryPhase::Closed | FactoryPhase::Construction))
        {
            *output_by_site.entry(f.site).or_default() += f.output;
        }
        let scale_met = self.selected_candidate.is_some_and(|site| {
            let own = output_by_site.get(&site).copied().unwrap_or(0.0);
            let larger = output_by_site
                .iter()
                .filter(|(id, output)| **id != site && **output > own)
                .count();
            own > 0.0
                && larger == 1
                && own >= output_by_site[&39] * self.calibration.capital_output_ratio
        });
        let sustainable = scale_met
            && self.selected_candidate.is_some_and(|site| {
                self.factories
                    .values()
                    .filter(|f| {
                        f.site == site
                            && !matches!(f.phase, FactoryPhase::Construction | FactoryPhase::Closed)
                    })
                    .all(|f| f.after_tax_profit > 0 && f.operating_subsidy == 0)
            });
        self.history.push(MonthRecord {
            month,
            output_by_site,
            scale_met,
            sustainable,
            housing_ok: self
                .sites
                .values()
                .all(|s| s.housing_capacity >= s.population),
            clinic_ok: self.sites.values().all(|s| {
                s.clinic_capacity as f64 * self.health_funding_ratio >= s.population as f64
            }),
            security_ok: self.security >= 50.0,
            treasury: game.balance("treasury", &Asset::Money).unwrap(),
            recurring_balance: self.last_recurring_balance,
            aid: self.last_aid,
        });
        if month == 24 {
            self.report = Some(self.make_report(game)?);
        }
        let commands = std::mem::take(&mut self.planned);
        self.completed_plans.push(CompletedPlan { month, commands });
        Ok(())
    }
    fn energy_remaining_liability(&self, month: u32) -> Result<u64> {
        let Some(start) = self.energy_long_start else {
            return Ok(0);
        };
        let remaining = (start + 24).saturating_sub(month + 1);
        let doc = document()?;
        let monthly = money(
            number(doc, "/energy/renewal_quantity/value")? * param(doc, "long_energy_price")?,
        )?;
        monthly.checked_mul(remaining as u64).ok_or(Error::Overflow)
    }
    fn make_report(&self, game: &Game) -> Result<GovernanceReport> {
        let last: Vec<_> = self.history.iter().rev().take(3).collect();
        let sustained = last.len() == 3 && last.iter().all(|r| r.sustainable && r.scale_met);
        let guards = last.len() == 3
            && last
                .iter()
                .all(|r| r.housing_ok && r.clinic_ok && r.security_ok);
        let ever_scale = self.history.iter().any(|r| r.scale_met);
        let first_sustained_month = self
            .history
            .windows(3)
            .find(|w| {
                w.iter().all(|r| {
                    r.sustainable && r.scale_met && r.housing_ok && r.clinic_ok && r.security_ok
                })
            })
            .map(|w| w[2].month);
        let grade = if sustained && guards {
            GoalGrade::Achieved
        } else if ever_scale {
            GoalGrade::Partial
        } else {
            GoalGrade::NotAchieved
        };
        let mut failures = vec![];
        if !ever_scale {
            failures
                .push("selected_inland_output_never_reached_second_rank_and_capital_ratio".into());
        }
        if !sustained {
            failures.push("last_three_month_scale_and_unsubsidized_profit_not_sustained".into());
        }
        if !guards {
            failures.push("last_three_month_livelihood_or_security_guardrail_failed".into());
        }
        let fiscal_recovered = last.len() == 3 && last.iter().all(|r| r.recurring_balance >= 0);
        if !fiscal_recovered {
            failures.push("fiscal_recovery_separate_last_three_recurring_negative".into());
        }
        let mut causes = self.facts.clone();
        if let Some((month, RenewalChoice::WaterPriority)) = self.renewal {
            for group in [Group::A, Group::B] {
                if let Some(site) = self.selected_candidate {
                    let card = self.assess(game, group, site)?;
                    if card
                        .gaps
                        .get("admission_quota")
                        .is_some_and(|gap| *gap > 0.0)
                        || card.gaps.get("water_quota").is_some_and(|gap| *gap > 0.0)
                    {
                        causes.push(Fact {
                            month,
                            effect: "water_priority_limits_next_factory".into(),
                            causes: vec!["aid/renewal".into(), format!("site/{site}")],
                            deadline: None,
                            default_result: None,
                        });
                        break;
                    }
                }
            }
        }
        Ok(GovernanceReport {
            month: 24,
            grade,
            first_sustained_month,
            fiscal_recovered,
            failures,
            causes,
            housing_failed_months: self
                .history
                .iter()
                .filter(|r| !r.housing_ok)
                .map(|r| r.month)
                .collect(),
            clinic_failed_months: self
                .history
                .iter()
                .filter(|r| !r.clinic_ok)
                .map(|r| r.month)
                .collect(),
            security_failed_months: self
                .history
                .iter()
                .filter(|r| !r.security_ok)
                .map(|r| r.month)
                .collect(),
            treasury_net_energy_liability: game.balance("treasury", &Asset::Money).unwrap_or(0)
                as i128
                - game
                    .payable("resource_energy_long_term")
                    .map(|p| p.unpaid as i128)
                    .unwrap_or(0),
            unpaid_energy_arrears: game
                .payable("resource_energy_long_term")
                .map(|p| {
                    p.unpaid
                        .saturating_sub(self.energy_remaining_liability(24).unwrap_or(0))
                })
                .unwrap_or(0),
            future_energy_liability: self.energy_remaining_liability(24)?,
            final_three_recurring: self
                .history
                .iter()
                .rev()
                .take(3)
                .map(|r| r.recurring_balance)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect(),
        })
    }
    fn validate(&self, game: &Game) -> Result<()> {
        self.calibration.validate()?;
        if self.rules_version != GOVERNANCE_RULES_VERSION {
            return Err(Error::Invalid("unsupported governance rules version"));
        }
        if self.clinical_coverage_institution
            || self
                .institution_changes
                .contains_key(&Institution::ClinicalCoverage)
        {
            return Err(Error::Invalid(
                "clinical coverage switching is outside the first governance slice",
            ));
        }
        if self
            .sites
            .keys()
            .copied()
            .ne(game.scenario.countries["player"]
                .iter()
                .copied()
                .collect::<BTreeSet<_>>())
            || self
                .sites
                .values()
                .map(|s| s.population as u64)
                .sum::<u64>()
                != 10_000
            || self.history.len() != game.month.saturating_sub(1) as usize
            || self.completed_plans.len() != self.history.len()
            || !self.security.is_finite()
            || !(0.0..=70.0).contains(&self.security)
            || !self.health_funding_ratio.is_finite()
            || !(0.0..=1.0).contains(&self.health_funding_ratio)
            || !self.export_orders.is_finite()
            || self.export_orders < 0.0
        {
            return Err(Error::Invalid("governance state invariants"));
        }
        if self.sites.values().any(|s| {
            s.skilled_workers > s.idle_workers
                || s.enrolled_students > s.idle_workers.saturating_sub(s.skilled_workers)
        }) {
            return Err(Error::Invalid("skilled and enrolled worker subsets"));
        }
        let mut labor: u64 = self
            .sites
            .values()
            .map(|s| (s.idle_workers + s.teachers) as u64)
            .sum();
        for (id, f) in &self.factories {
            if id != &f.id
                || !self.sites.contains_key(&f.site)
                || !f.output.is_finite()
                || f.output < 0.0
                || !f.domestic_sales.is_finite()
                || !f.export_sales.is_finite()
                || f.domestic_sales < 0.0
                || f.export_sales < 0.0
                || (f.phase != FactoryPhase::Closed
                    && (f.output - f.domestic_sales - f.export_sales).abs() > 1e-8)
                || f.operating_subsidy != 0
                || f.worked_months > f.construction_months
                || f.started_month == 0
                || f.activate_month != f.started_month + f.construction_months
                || f.subsidy > f.capital
            {
                return Err(Error::Invalid("factory business invariants"));
            }
            let reservation = game
                .reservations
                .get(id)
                .ok_or(Error::Invalid("factory reservation"))?;
            let first = game
                .applied
                .values()
                .filter_map(|a| match &a.command {
                    Command::MarkOperational { reservation } if reservation == id => {
                        Some(a.receipt.month)
                    }
                    _ => None,
                })
                .min();
            if f.first_operated_month != first
                || f.first_operated_month
                    .is_some_and(|month| month > game.month)
            {
                return Err(Error::Invalid("factory first actual operation"));
            }
            let valid_phase = match f.phase {
                FactoryPhase::Construction => {
                    !f.water_suspended
                        && f.first_operated_month.is_none()
                        && matches!(
                            reservation.phase,
                            AllocationPhase::Reserved | AllocationPhase::WaitingForWater
                        )
                }
                FactoryPhase::Closed => {
                    !f.water_suspended && reservation.phase == AllocationPhase::Retired
                }
                _ => {
                    f.first_operated_month.is_some()
                        && reservation.phase
                            == if f.water_suspended {
                                AllocationPhase::OperationalWaitingForWater
                            } else {
                                AllocationPhase::Operational
                            }
                }
            };
            if !valid_phase {
                return Err(Error::Invalid("factory operational water phase"));
            }
            if f.phase != FactoryPhase::Closed {
                labor += f.workers as u64;
            }
        }
        if labor != 295 {
            return Err(Error::Invalid("domestic idle labor conservation"));
        }
        for (idx, r) in self.history.iter().enumerate() {
            if r.month != idx as u32 + 1 {
                return Err(Error::Invalid("monthly business history"));
            }
        }
        if self
            .selected_candidate
            .is_some_and(|id| !game.scenario.candidates.contains(&id))
        {
            return Err(Error::Invalid("goal site"));
        }
        for site in [39, 74] {
            if !self.sites[&site].admitted {
                return Err(Error::Invalid("initial open region cannot close"));
            }
        }
        for (id, p) in &self.projects {
            if p.active != p.first_operated_month.is_some()
                || p.first_operated_month
                    .is_some_and(|month| month > game.month)
                || (p.water_suspended && (!p.active || p.kind != PublicWork::Housing))
            {
                return Err(Error::Invalid("public first actual operation"));
            }
            if p.kind == PublicWork::Housing {
                let reservation_id = format!("public/{id}");
                let reservation = game
                    .reservations
                    .get(&reservation_id)
                    .ok_or(Error::Invalid("housing reservation"))?;
                let first = game
                    .applied
                    .values()
                    .filter_map(|a| match &a.command {
                        Command::MarkOperational { reservation }
                            if reservation == &reservation_id =>
                        {
                            Some(a.receipt.month)
                        }
                        _ => None,
                    })
                    .min();
                if p.first_operated_month != first {
                    return Err(Error::Invalid("housing first actual operation"));
                }
                let valid = if p.active {
                    reservation.phase
                        == if p.water_suspended {
                            AllocationPhase::OperationalWaitingForWater
                        } else {
                            AllocationPhase::Operational
                        }
                } else {
                    matches!(
                        reservation.phase,
                        AllocationPhase::Reserved | AllocationPhase::WaitingForWater
                    )
                };
                if !valid {
                    return Err(Error::Invalid("housing operational water phase"));
                }
            }
            if id != &p.id
                || p.worked_months > p.construction_months
                || p.construction_months == 0
                || p.started_month == 0
                || p.started_month > game.month
                || !self.sites.contains_key(&p.site)
            {
                return Err(Error::Invalid("public project state"));
            }
        }
        for (id, t) in &self.todos {
            if id != &t.id
                || t.created_month == 0
                || t.due_month < t.created_month
                || t.options.is_empty()
            {
                return Err(Error::Invalid("governance todo"));
            }
        }
        Ok(())
    }
}

fn investment_preference(a: &Scorecard, b: &Scorecard) -> std::cmp::Ordering {
    match (a.static_payback_months, b.static_payback_months) {
        (Some(a), Some(b)) => a.total_cmp(&b),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    }
    .then_with(|| a.site.cmp(&b.site))
}

fn external_party(reason: &str) -> &'static str {
    match reason {
        "existing_nonmanufacturing_employer_wages" => "existing_nonmanufacturing_employers",
        "ordinary_energy_delivery" | "ordinary_energy_consumption" => {
            "resource_country_energy_producer"
        }
        "maritime_energy_delivery" | "maritime_energy_consumption" => "maritime_energy_exporter",
        "new_factory_energy" => "maritime_energy_exporter",
        "foreign_contract_buyer_budget" => "maritime_contract_buyer_finance",
        "existing_manufacturing_maintenance" | "factory_maintenance" => "maintenance_suppliers",
        "existing_outside_shareholder_distribution"
        | "outside_shareholder_distribution_no_reinjection" => "outside_shareholders",
        "remaining_household_manufactured_imports" => "maritime_goods_exporter",
        "household_food_import_payment" | "household_background_food_import" => "food_exporter",
        "capital_material_production" => "capital_material_producers",
        "background_material_import_paid_in_project_capital" => "maritime_material_supplier",
        "equipment_retrofit" | "equipment_retrofit_materials" => "equipment_supplier",
        "public_construction_consumption" | "factory_construction_consumption" => {
            "construction_process"
        }
        "household_food_consumption" => "household_consumption",
        "existing_manufacturing_production" | "factory_goods_production_scaled_quantity" => {
            "manufacturing_process"
        }
        "existing_household_goods_delivery" | "market_goods_delivery_scaled_quantity" => {
            "manufactured_goods_consumers"
        }
        "market_training_cooperation_maintenance" => "maritime_training_partner",
        "resource_transition_aid" => "resource_government",
        s if s.starts_with("recurring_service_") => "public_service_suppliers",
        _ => "unspecified_settlement",
    }
}

#[cfg(test)]
mod order_priority_tests {
    use super::*;

    // A narrow production-stage fixture: deliberately cross operation priority and
    // ID priority without changing approved scenario inputs or inventing a command.
    fn plant(id: &str, group: Group, site: u32, first: u32) -> Factory {
        Factory {
            id: id.into(),
            group,
            site,
            phase: FactoryPhase::Operating,
            water_suspended: false,
            first_operated_month: Some(first),
            started_month: 1,
            activate_month: 4,
            construction_months: 3,
            worked_months: 3,
            capital: 0,
            subsidy: 0,
            subsidy_escrow: None,
            capital_escrow: String::new(),
            reserved_materials: 0,
            loss_months: 0,
            close_month: None,
            workers: 100,
            skilled_workers: 20,
            output: 0.0,
            domestic_sales: 0.0,
            export_sales: 0.0,
            ordinary_energy_used: 0.0,
            maritime_energy_used: 0.0,
            after_tax_profit: 0,
            operating_subsidy: 0,
            retrofitted: true,
            retrofit_activate_month: None,
        }
    }
    fn production_fixture(road: u32, power: u32) -> (Game, GovernanceState) {
        let mut game = Game::confirmed_game().unwrap();
        game.month = 10;
        let mut state = game.governance.take().unwrap();
        state.sites.get_mut(&29).unwrap().road_capacity = road;
        state.sites.get_mut(&29).unwrap().power_capacity = power;
        state
            .factories
            .insert("factory/1".into(), plant("factory/1", Group::A, 29, 9));
        state
            .factories
            .insert("factory/2".into(), plant("factory/2", Group::B, 29, 8));
        (game, state)
    }

    #[test]
    fn actual_operation_priority_crosses_id_without_reordering_road_or_power() {
        for (road, power) in [(100, 240), (240, 100)] {
            let (mut game, mut state) = production_fixture(road, power);
            state.production(&mut game).unwrap();
            // Domestic orders: earlier operation #2 gets120; later #1 gets80.
            // Physical allocation still follows ID: #1 sells80, leaving20 for #2.
            assert_eq!(state.factories["factory/1"].domestic_sales, 80.0);
            assert_eq!(state.factories["factory/2"].domestic_sales, 20.0);
            assert_eq!(state.factories["factory/1"].export_sales, 0.0);
            assert_eq!(state.factories["factory/2"].export_sales, 0.0);
        }
    }

    #[test]
    fn unfilled_domestic_orders_are_not_reassigned_within_the_month() {
        let (mut game, mut state) = production_fixture(240, 240);
        // Earlier #2 gets120 orders but its site cannot fulfil any; #1 may only
        // sell its own80 despite having spare road, power and productive capacity.
        state.factories.get_mut("factory/2").unwrap().site = 8;
        state.sites.get_mut(&8).unwrap().road_capacity = 0;
        state.production(&mut game).unwrap();
        assert_eq!(state.factories["factory/1"].domestic_sales, 80.0);
        assert_eq!(state.factories["factory/2"].domestic_sales, 0.0);
    }

    #[test]
    fn cross_site_capital_and_candidate_share_domestic_orders_and_export_budget() {
        let (mut game, mut state) = production_fixture(240, 240);
        state.factories.get_mut("factory/2").unwrap().site = 39;
        state.sites.get_mut(&39).unwrap().road_capacity = 240;
        state.sites.get_mut(&39).unwrap().power_capacity = 240;
        state.export_orders = 100.0;
        state.production(&mut game).unwrap();
        assert_eq!(state.factories["factory/2"].domestic_sales, 120.0);
        assert_eq!(state.factories["factory/1"].domestic_sales, 80.0);
        assert_eq!(state.factories["factory/1"].export_sales, 40.0);
        assert_eq!(state.factories["factory/2"].export_sales, 0.0);
    }

    #[test]
    fn competing_exports_keep_id_priority_and_unused_budget_remains_available() {
        for first_road in [600, 0] {
            let (mut game, mut state) = production_fixture(600, 600);
            // Technical capacity stress only: make both plants able to compete for
            // the same100 export budget; approved defaults remain unchanged.
            state.calibration.retrofit_output_increase = 2.0;
            state.export_orders = 100.0;
            state.factories.get_mut("factory/1").unwrap().site = 8;
            state.sites.get_mut(&8).unwrap().road_capacity = first_road;
            state.sites.get_mut(&8).unwrap().power_capacity = 600;
            state.production(&mut game).unwrap();
            assert_eq!(state.factories["factory/1"].domestic_sales, 0.0);
            assert_eq!(state.factories["factory/2"].domestic_sales, 200.0);
            // Earlier actual operation still gets domestic orders, but export
            // priority stays ID-based and consumes only completed export sales.
            assert_eq!(
                state.factories["factory/1"].export_sales,
                if first_road == 0 { 0.0 } else { 100.0 }
            );
            assert_eq!(
                state.factories["factory/2"].export_sales,
                if first_road == 0 { 100.0 } else { 0.0 }
            );
        }
    }

    #[test]
    fn equal_operation_month_uses_stable_id_and_water_suspension_gets_no_orders() {
        let (mut game, mut state) = production_fixture(240, 240);
        state
            .factories
            .get_mut("factory/1")
            .unwrap()
            .first_operated_month = Some(8);
        state.production(&mut game).unwrap();
        assert_eq!(state.factories["factory/1"].domestic_sales, 120.0);
        assert_eq!(state.factories["factory/2"].domestic_sales, 80.0);
        let (mut game, mut state) = production_fixture(240, 240);
        state
            .factories
            .get_mut("factory/2")
            .unwrap()
            .water_suspended = true;
        state.production(&mut game).unwrap();
        assert_eq!(state.factories["factory/1"].domestic_sales, 120.0);
        assert_eq!(state.factories["factory/2"].domestic_sales, 0.0);
        assert_eq!(state.factories["factory/2"].first_operated_month, Some(8));
    }
}
