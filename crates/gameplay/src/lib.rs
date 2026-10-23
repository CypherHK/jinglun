//! Foundation primitives only. No economy, population, tax, AI, or facility rules.
mod descriptors;
mod governance;
pub mod initial_values;
pub use governance::*;
mod scenario;
pub use descriptors::{
    AdmissionLimit, AllocationPhase, PayableView, PaymentObligation, TodoDescriptor,
};
use descriptors::{AdmissionState, ObligationState};
mod save;

pub use scenario::{GENERATOR_VERSION, SCENARIO_VERSION, Scenario};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fmt;

pub type Result<T> = std::result::Result<T, Error>;
#[derive(Debug)]
pub enum Error {
    Invalid(&'static str),
    Conflict,
    Overflow,
    Missing(&'static str),
    Insufficient(&'static str),
    CapacityShortfalls {
        land: BTreeMap<u32, u64>,
        water: BTreeMap<u32, u64>,
        admission: BTreeMap<String, u64>,
    },
    Json(serde_json::Error),
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}
impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Confirmation {
    Draft,
    Unconfirmed,
    Confirmed,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Rules {
    pub version: String,
    pub status: Confirmation,
    pub land_km2_per_slot: f64,
    pub land_min: u64,
    pub land_max: u64,
    pub water_m3_per_slot: f64,
    pub water_min: u64,
    pub water_max: u64,
}
impl Rules {
    /// Calibration sample only. These values are not approved gameplay balance.
    pub fn pending_sample() -> Self {
        Self {
            version: "foundation-quota-sample-1".into(),
            status: Confirmation::Unconfirmed,
            land_km2_per_slot: 100_000.0,
            land_min: 3,
            land_max: 8,
            water_m3_per_slot: 400_000_000.0,
            water_min: 0,
            water_max: 8,
        }
    }
    fn validate(&self) -> Result<()> {
        if self.version != "foundation-quota-sample-1"
            || self.land_min > self.land_max
            || self.water_min > self.water_max
        {
            return Err(Error::Invalid("rules version or bounds"));
        }
        scenario::positive(self.land_km2_per_slot)?;
        scenario::positive(self.water_m3_per_slot)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(into = "String", try_from = "String")]
pub enum Asset {
    Money,
    Material(String),
}
impl From<Asset> for String {
    fn from(asset: Asset) -> Self {
        match asset {
            Asset::Money => "money".into(),
            Asset::Material(name) => format!("material/{name}"),
        }
    }
}
impl TryFrom<String> for Asset {
    type Error = &'static str;
    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        if value == "money" {
            Ok(Self::Money)
        } else if let Some(name) = value.strip_prefix("material/") {
            if name.trim().is_empty() {
                Err("empty material")
            } else {
                Ok(Self::Material(name.into()))
            }
        } else {
            Err("unknown asset")
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct InitialAccount {
    pub id: String,
    pub balances: BTreeMap<Asset, u64>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct InitialState {
    pub status: Confirmation,
    pub provenance: String,
    pub accounts: Vec<InitialAccount>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum LandUse {
    Factory,
    Housing,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Reservation {
    pub id: String,
    pub country: String,
    pub cell: u32,
    pub source: u32,
    pub land: u64,
    pub water: u64,
    pub purpose: LandUse,
    pub phase: AllocationPhase,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub enum Command {
    Transfer {
        from: String,
        to: String,
        asset: Asset,
        amount: u64,
    },
    FundEscrow {
        escrow: String,
        payer: String,
        payee: String,
        amount: u64,
    },
    ProgressEscrow {
        escrow: String,
        basis_points: u16,
    },
    CancelEscrow {
        escrow: String,
    },
    Reserve(Reservation),
    Release {
        reservation: String,
    },
    MarkOperational {
        reservation: String,
    },
    AddAdmissionLimit(AdmissionLimit),
    RecordObligation(PaymentObligation),
    PayObligation {
        obligation: String,
        due_month: u32,
        amount: u64,
    },
    AddTodo(TodoDescriptor),
    /// Technical availability marker only; defines no product facility behavior.
    ScheduleActivation {
        capability: String,
        amount: u64,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Receipt {
    pub command_id: String,
    pub month: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Applied {
    admission_grandfathered: Option<std::collections::BTreeSet<String>>,
    command: Command,
    receipt: Receipt,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Escrow {
    payer: String,
    payee: String,
    total: u64,
    paid: u64,
    progress: u16,
    cancelled: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Activation {
    capability: String,
    amount: u64,
    month: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Ledger {
    accounts: BTreeMap<String, BTreeMap<Asset, u64>>,
    escrows: BTreeMap<String, Escrow>,
}
impl Ledger {
    fn transfer(&mut self, from: &str, to: &str, asset: &Asset, amount: u64) -> Result<()> {
        if amount == 0 || from == to {
            return Err(Error::Invalid(
                "nonzero transfer between counterparties required",
            ));
        }
        let from_balance = self
            .accounts
            .get(from)
            .ok_or(Error::Missing("payer"))?
            .get(asset)
            .copied()
            .unwrap_or(0);
        let to_balance = self
            .accounts
            .get(to)
            .ok_or(Error::Missing("payee"))?
            .get(asset)
            .copied()
            .unwrap_or(0);
        let debit = from_balance
            .checked_sub(amount)
            .ok_or(Error::Insufficient("balance"))?;
        let credit = to_balance.checked_add(amount).ok_or(Error::Overflow)?;
        self.accounts
            .get_mut(from)
            .unwrap()
            .insert(asset.clone(), debit);
        self.accounts
            .get_mut(to)
            .unwrap()
            .insert(asset.clone(), credit);
        Ok(())
    }
    fn credit_money(&mut self, to: &str, amount: u64) -> Result<()> {
        let account = self
            .accounts
            .get_mut(to)
            .ok_or(Error::Missing("escrow counterparty"))?;
        let new = account
            .get(&Asset::Money)
            .copied()
            .unwrap_or(0)
            .checked_add(amount)
            .ok_or(Error::Overflow)?;
        account.insert(Asset::Money, new);
        Ok(())
    }
    fn totals(&self) -> Result<BTreeMap<Asset, u128>> {
        let mut totals = BTreeMap::new();
        for account in self.accounts.values() {
            for (asset, amount) in account {
                *totals.entry(asset.clone()).or_insert(0u128) = totals
                    .get(asset)
                    .copied()
                    .unwrap_or(0)
                    .checked_add(*amount as u128)
                    .ok_or(Error::Overflow)?;
            }
        }
        for escrow in self.escrows.values().filter(|e| !e.cancelled) {
            let remaining = escrow
                .total
                .checked_sub(escrow.paid)
                .ok_or(Error::Invalid("escrow paid"))?;
            *totals.entry(Asset::Money).or_insert(0u128) += remaining as u128;
        }
        totals.retain(|_, amount| *amount != 0);
        Ok(totals)
    }
}

#[derive(Clone, Debug)]
pub struct Game {
    map: Value,
    scenario: Scenario,
    rules: Rules,
    initial: InitialState,
    projection: scenario::Projection,
    ledger: Ledger,
    reservations: BTreeMap<String, Reservation>,
    month: u32,
    pending: Vec<Activation>,
    available: BTreeMap<String, u64>,
    applied: BTreeMap<String, Applied>,
    obligations: BTreeMap<String, ObligationState>,
    admissions: BTreeMap<String, AdmissionState>,
    todos: BTreeMap<String, TodoDescriptor>,
    governance: Option<GovernanceState>,
    system_receipts: Vec<SystemReceipt>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CellView {
    pub raw_id: u32,
    pub country: String,
    pub buildable_area_km2: f64,
    pub cost_multiplier: Option<f64>,
    pub land_slots: u64,
    pub water_sources: BTreeMap<u32, (f64, u64)>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MonthReceipt {
    pub completed_month: u32,
    pub stages: Vec<&'static str>,
    pub foundation_mode: bool,
}
#[derive(Clone, Copy)]
enum Stage {
    CommandsEscrow,
    SeasonAnnouncementsAi,
    OpeningCapacity,
    Allocation,
    ProductionConstruction,
    LivelihoodTraining,
    DiplomacySecurity,
    TaxesLongTerm,
    Archive,
}
const STAGES: [Stage; 9] = [
    Stage::CommandsEscrow,
    Stage::SeasonAnnouncementsAi,
    Stage::OpeningCapacity,
    Stage::Allocation,
    Stage::ProductionConstruction,
    Stage::LivelihoodTraining,
    Stage::DiplomacySecurity,
    Stage::TaxesLongTerm,
    Stage::Archive,
];
impl Stage {
    fn name(self) -> &'static str {
        match self {
            Self::CommandsEscrow => "commands_escrow",
            Self::SeasonAnnouncementsAi => "season_announcements_ai",
            Self::OpeningCapacity => "opening_capacity",
            Self::Allocation => "allocation",
            Self::ProductionConstruction => "production_construction",
            Self::LivelihoodTraining => "livelihood_training",
            Self::DiplomacySecurity => "diplomacy_security",
            Self::TaxesLongTerm => "taxes_long_term",
            Self::Archive => "archive",
        }
    }
}

impl Game {
    pub fn initialize(rules: Rules, initial: InitialState) -> Result<Self> {
        Self::from_map(scenario::generated()?, Scenario::fixed(), rules, initial)
    }
    fn from_map(
        map: Value,
        scenario: Scenario,
        rules: Rules,
        initial: InitialState,
    ) -> Result<Self> {
        rules.validate()?;
        let projection = scenario::project(&map, &scenario, &rules)?;
        let ledger = Self::initial_ledger(&initial)?;
        let game = Self {
            map,
            scenario,
            rules,
            initial,
            projection,
            ledger,
            reservations: BTreeMap::new(),
            month: 1,
            pending: vec![],
            available: BTreeMap::new(),
            applied: BTreeMap::new(),
            obligations: BTreeMap::new(),
            admissions: BTreeMap::new(),
            todos: BTreeMap::new(),
            governance: None,
            system_receipts: Vec::new(),
        };
        game.validate()?;
        Ok(game)
    }
    fn initial_ledger(initial: &InitialState) -> Result<Ledger> {
        identifier(&initial.provenance)?;
        let mut accounts = BTreeMap::new();
        for account in &initial.accounts {
            identifier(&account.id)?;
            for asset in account.balances.keys() {
                validate_asset(asset)?;
            }
            if accounts
                .insert(account.id.clone(), account.balances.clone())
                .is_some()
            {
                return Err(Error::Invalid("duplicate initial account"));
            }
        }
        if accounts.is_empty() {
            return Err(Error::Invalid("explicit initial accounts required"));
        }
        Ok(Ledger {
            accounts,
            escrows: BTreeMap::new(),
        })
    }
    pub fn apply(&mut self, command_id: impl Into<String>, command: Command) -> Result<Receipt> {
        if self.governance.is_some() {
            return Err(Error::Invalid(
                "use governance command plans in formal mode",
            ));
        }
        let command_id = command_id.into();
        identifier(&command_id)?;
        if let Some(applied) = self.applied.get(&command_id) {
            return if applied.command == command {
                Ok(applied.receipt.clone())
            } else {
                Err(Error::Conflict)
            };
        }
        let mut candidate = self.clone();
        candidate.apply_internal(command_id.clone(), command.clone())?;
        candidate.validate()?;
        let receipt = candidate.applied[&command_id].receipt.clone();
        *self = candidate;
        Ok(receipt)
    }
    fn apply_internal(&mut self, command_id: String, command: Command) -> Result<Receipt> {
        let candidate = self;
        candidate.execute(&command)?;
        let receipt = Receipt {
            command_id: command_id.clone(),
            month: candidate.month,
        };
        let admission_grandfathered = match &command {
            Command::AddAdmissionLimit(limit) => {
                Some(candidate.admissions[&limit.id].grandfathered.clone())
            }
            _ => None,
        };
        candidate.applied.insert(
            command_id,
            Applied {
                admission_grandfathered,
                command,
                receipt: receipt.clone(),
            },
        );
        Ok(receipt)
    }
    fn execute(&mut self, command: &Command) -> Result<()> {
        match command {
            Command::Transfer {
                from,
                to,
                asset,
                amount,
            } => {
                validate_asset(asset)?;
                self.ledger.transfer(from, to, asset, *amount)?;
            }
            Command::FundEscrow {
                escrow,
                payer,
                payee,
                amount,
            } => {
                identifier(escrow)?;
                if *amount == 0
                    || payer == payee
                    || self.ledger.escrows.contains_key(escrow)
                    || self.ledger.accounts.contains_key(escrow)
                {
                    return Err(Error::Invalid("new independent escrow required"));
                }
                if !self.ledger.accounts.contains_key(payee) {
                    return Err(Error::Missing("payee"));
                }
                let account = self
                    .ledger
                    .accounts
                    .get_mut(payer)
                    .ok_or(Error::Missing("payer"))?;
                let balance = account
                    .get(&Asset::Money)
                    .copied()
                    .unwrap_or(0)
                    .checked_sub(*amount)
                    .ok_or(Error::Insufficient("escrow funds"))?;
                account.insert(Asset::Money, balance);
                self.ledger.escrows.insert(
                    escrow.clone(),
                    Escrow {
                        payer: payer.clone(),
                        payee: payee.clone(),
                        total: *amount,
                        paid: 0,
                        progress: 0,
                        cancelled: false,
                    },
                );
            }
            Command::ProgressEscrow {
                escrow,
                basis_points,
            } => {
                let e = self
                    .ledger
                    .escrows
                    .get_mut(escrow)
                    .ok_or(Error::Missing("escrow"))?;
                if e.cancelled || *basis_points > 10_000 || *basis_points < e.progress {
                    return Err(Error::Invalid("monotonic active escrow progress required"));
                }
                let cumulative = ((e.total as u128 * *basis_points as u128) / 10_000) as u64;
                let amount = cumulative
                    .checked_sub(e.paid)
                    .ok_or(Error::Invalid("escrow cumulative payment"))?;
                e.paid = cumulative;
                e.progress = *basis_points;
                let payee = e.payee.clone();
                self.ledger.credit_money(&payee, amount)?;
            }
            Command::CancelEscrow { escrow } => {
                let e = self
                    .ledger
                    .escrows
                    .get_mut(escrow)
                    .ok_or(Error::Missing("escrow"))?;
                if e.cancelled || e.progress == 10_000 {
                    return Err(Error::Invalid("escrow already closed"));
                }
                let amount = e
                    .total
                    .checked_sub(e.paid)
                    .ok_or(Error::Invalid("escrow remaining"))?;
                e.cancelled = true;
                let payer = e.payer.clone();
                self.ledger.credit_money(&payer, amount)?;
            }
            Command::Reserve(request) => {
                identifier(&request.id)?;
                if request.phase != AllocationPhase::Reserved {
                    return Err(Error::Invalid("reserve before becoming operational"));
                }
                if self
                    .applied
                    .values()
                    .any(|a| matches!(&a.command, Command::Reserve(r) if r.id == request.id))
                    || self.reservations.contains_key(&request.id)
                {
                    return Err(Error::Conflict);
                }
                self.validate_reservation(request)?;
                self.reservations
                    .insert(request.id.clone(), request.clone());
                self.validate_capacity()?;
            }
            Command::Release { reservation } => {
                self.reservations
                    .remove(reservation)
                    .ok_or(Error::Missing("reservation"))?;
            }
            Command::MarkOperational { reservation } => {
                let r = self
                    .reservations
                    .get_mut(reservation)
                    .ok_or(Error::Missing("reservation"))?;
                if r.phase != AllocationPhase::Reserved {
                    return Err(Error::Invalid("already operational"));
                }
                r.phase = AllocationPhase::Operational;
            }
            Command::AddAdmissionLimit(limit) => {
                identifier(&limit.id)?;
                if limit.start_month != self.month
                    || limit.end_month < limit.start_month
                    || self.admissions.contains_key(&limit.id)
                    || !self.scenario.countries.contains_key(&limit.country)
                    || !self.projection.water.contains_key(&limit.source)
                {
                    return Err(Error::Invalid("admission identity or references"));
                }
                let grandfathered = self
                    .reservations
                    .values()
                    .filter(|r| {
                        r.phase == AllocationPhase::Operational
                            && r.country == limit.country
                            && r.source == limit.source
                    })
                    .map(|r| r.id.clone())
                    .collect();
                self.admissions.insert(
                    limit.id.clone(),
                    AdmissionState {
                        limit: limit.clone(),
                        grandfathered,
                    },
                );
                self.validate_capacity()?;
            }
            Command::RecordObligation(terms) => {
                terms.validate(&self.ledger.accounts)?;
                if terms.start_month < self.month {
                    return Err(Error::Invalid("obligation starts in past"));
                }
                if self.obligations.contains_key(&terms.id) {
                    return Err(Error::Conflict);
                }
                self.obligations.insert(
                    terms.id.clone(),
                    ObligationState {
                        terms: terms.clone(),
                        payments: BTreeMap::new(),
                    },
                );
            }
            Command::PayObligation {
                obligation,
                due_month,
                amount,
            } => {
                let state = self
                    .obligations
                    .get_mut(obligation)
                    .ok_or(Error::Missing("obligation"))?;
                let terms = &state.terms;
                if *due_month < terms.start_month
                    || *due_month >= terms.start_month + terms.months
                    || *due_month > self.month
                    || *amount == 0
                {
                    return Err(Error::Invalid("obligation payment month or amount"));
                }
                let paid = state.payments.entry(*due_month).or_default();
                *paid = paid.checked_add(*amount).ok_or(Error::Overflow)?;
                if *paid > terms.monthly_due()? {
                    return Err(Error::Invalid("obligation overpayment"));
                }
                self.ledger
                    .transfer(&terms.payer, &terms.payee, &Asset::Money, *amount)?;
            }
            Command::AddTodo(todo) => {
                todo.validate(self.month)?;
                if todo.created_month != self.month || self.todos.contains_key(&todo.id) {
                    return Err(Error::Invalid("new todo identity or month"));
                }
                self.todos.insert(todo.id.clone(), todo.clone());
            }
            Command::ScheduleActivation { capability, amount } => {
                identifier(capability)?;
                if *amount == 0 {
                    return Err(Error::Invalid("nonzero technical capability required"));
                }
                self.pending.push(Activation {
                    capability: capability.clone(),
                    amount: *amount,
                    month: self.month.checked_add(1).ok_or(Error::Overflow)?,
                });
            }
        }
        Ok(())
    }
    /// Stages without authorized business rules are deliberately inert.
    pub fn end_month(&mut self, expected_month: u32) -> Result<MonthReceipt> {
        if self.month != expected_month {
            return Err(Error::Conflict);
        }
        let mut candidate = self.clone();
        candidate.advance_month_unchecked()?;
        candidate.validate()?;
        let foundation_mode = candidate.governance.is_none();
        *self = candidate;
        Ok(MonthReceipt {
            completed_month: expected_month,
            stages: STAGES.map(Stage::name).to_vec(),
            foundation_mode,
        })
    }
    fn advance_month_unchecked(&mut self) -> Result<()> {
        let candidate = self;
        let next = candidate.month.checked_add(1).ok_or(Error::Overflow)?;
        for stage in STAGES {
            if let Some(mut state) = candidate.governance.take() {
                let result = state.stage(candidate, stage);
                candidate.governance = Some(state);
                result?;
            }
            if matches!(stage, Stage::Archive) {
                for item in &candidate.pending {
                    let value = candidate
                        .available
                        .entry(item.capability.clone())
                        .or_default();
                    *value = value.checked_add(item.amount).ok_or(Error::Overflow)?;
                }
                candidate.pending.clear();
                candidate.month = next;
            }
        }
        Ok(())
    }
    pub fn month(&self) -> u32 {
        self.month
    }
    pub fn balance(&self, account: &str, asset: &Asset) -> Option<u64> {
        self.ledger
            .accounts
            .get(account)
            .map(|a| a.get(asset).copied().unwrap_or(0))
    }
    pub fn escrow_remaining(&self, id: &str) -> Option<u64> {
        self.ledger
            .escrows
            .get(id)
            .map(|e| if e.cancelled { 0 } else { e.total - e.paid })
    }
    pub fn available(&self, capability: &str) -> u64 {
        self.available.get(capability).copied().unwrap_or(0)
    }
    pub fn payable(&self, id: &str) -> Result<PayableView> {
        let state = self
            .obligations
            .get(id)
            .ok_or(Error::Missing("obligation"))?;
        let monthly = state.terms.monthly_due()?;
        let total_committed = monthly
            .checked_mul(state.terms.months as u64)
            .ok_or(Error::Overflow)?;
        let paid = state.payments.values().try_fold(0u64, |sum, value| {
            sum.checked_add(*value).ok_or(Error::Overflow)
        })?;
        let elapsed = self
            .month
            .saturating_sub(state.terms.start_month)
            .min(state.terms.months);
        let paid_past = state
            .payments
            .iter()
            .filter(|(month, _)| **month < self.month)
            .try_fold(0u64, |sum, (_, value)| {
                sum.checked_add(*value).ok_or(Error::Overflow)
            })?;
        let arrears = monthly
            .checked_mul(elapsed as u64)
            .ok_or(Error::Overflow)?
            .checked_sub(paid_past)
            .ok_or(Error::Invalid("past payments exceed payable"))?;
        Ok(PayableView {
            total_committed,
            paid,
            unpaid: total_committed - paid,
            arrears,
        })
    }
    /// Pure descriptors. No deadline effects or three-item scheduling occur here.
    pub fn todos(&self) -> &BTreeMap<String, TodoDescriptor> {
        &self.todos
    }
    pub fn reservations(&self) -> &BTreeMap<String, Reservation> {
        &self.reservations
    }
    pub fn country_components(&self) -> &BTreeMap<String, usize> {
        &self.projection.components
    }
    pub fn map(&self) -> &Value {
        &self.map
    }
    pub fn rules(&self) -> &Rules {
        &self.rules
    }
    pub fn initial(&self) -> &InitialState {
        &self.initial
    }
    pub fn cell(&self, id: u32) -> Option<CellView> {
        let cell = self.projection.cells.get(&id)?;
        let country = self.projection.owners.get(&id)?.clone();
        Some(CellView {
            raw_id: id,
            country,
            buildable_area_km2: cell.construction.buildable_area_km2,
            cost_multiplier: cell.construction.cost_multiplier,
            land_slots: self.projection.land[&id],
            water_sources: cell
                .water_source_ids
                .iter()
                .map(|id| {
                    (
                        *id,
                        (self.projection.source_m3[id], self.projection.water[id]),
                    )
                })
                .collect(),
        })
    }
    fn validate_reservation(&self, r: &Reservation) -> Result<()> {
        if r.land == 0 && r.water == 0 {
            return Err(Error::Invalid("empty reservation"));
        }
        if self.projection.owners.get(&r.cell) != Some(&r.country) {
            return Err(Error::Invalid("cell country"));
        }
        if !self.projection.cells[&r.cell]
            .water_source_ids
            .contains(&r.source)
        {
            return Err(Error::Invalid("unassociated source"));
        }
        Ok(())
    }
    fn validate_capacity(&self) -> Result<()> {
        let mut land = BTreeMap::<u32, u64>::new();
        let mut water = BTreeMap::<u32, u64>::new();
        for (id, reservation) in &self.reservations {
            identifier(id)?;
            if id != &reservation.id {
                return Err(Error::Invalid("reservation key"));
            }
            self.validate_reservation(reservation)?;
            let l = land.entry(reservation.cell).or_default();
            *l = l.checked_add(reservation.land).ok_or(Error::Overflow)?;
            let w = water.entry(reservation.source).or_default();
            *w = w.checked_add(reservation.water).ok_or(Error::Overflow)?;
        }
        let land_shortfalls = land
            .iter()
            .filter_map(|(id, used)| {
                used.checked_sub(self.projection.land[id])
                    .filter(|n| *n > 0)
                    .map(|n| (*id, n))
            })
            .collect::<BTreeMap<_, _>>();
        let water_shortfalls = water
            .iter()
            .filter_map(|(id, used)| {
                used.checked_sub(self.projection.water[id])
                    .filter(|n| *n > 0)
                    .map(|n| (*id, n))
            })
            .collect::<BTreeMap<_, _>>();
        let mut admission_shortfalls = BTreeMap::new();
        for state in self
            .admissions
            .values()
            .filter(|s| self.month >= s.limit.start_month && self.month <= s.limit.end_month)
        {
            let mut used = 0u64;
            for r in self.reservations.values().filter(|r| {
                r.country == state.limit.country
                    && r.source == state.limit.source
                    && !state.grandfathered.contains(&r.id)
            }) {
                used = used.checked_add(r.water).ok_or(Error::Overflow)?;
            }
            if used > state.limit.new_water_slots {
                admission_shortfalls
                    .insert(state.limit.id.clone(), used - state.limit.new_water_slots);
            }
        }
        if !land_shortfalls.is_empty()
            || !water_shortfalls.is_empty()
            || !admission_shortfalls.is_empty()
        {
            return Err(Error::CapacityShortfalls {
                land: land_shortfalls,
                water: water_shortfalls,
                admission: admission_shortfalls,
            });
        }
        Ok(())
    }
    fn validate(&self) -> Result<()> {
        self.rules.validate()?;
        let initial = Self::initial_ledger(&self.initial)?;
        if self.month == 0
            || self.ledger.accounts.keys().ne(initial.accounts.keys())
            || !self.conserved_with_external(&initial)?
        {
            return Err(Error::Invalid("month, accounts or conserved balances"));
        }
        for account in self.ledger.accounts.values() {
            for asset in account.keys() {
                validate_asset(asset)?;
            }
        }
        for (id, e) in &self.ledger.escrows {
            identifier(id)?;
            if self.ledger.accounts.contains_key(id)
                || !self.ledger.accounts.contains_key(&e.payer)
                || !self.ledger.accounts.contains_key(&e.payee)
                || e.payer == e.payee
                || e.total == 0
                || e.progress > 10_000
                || e.paid != ((e.total as u128 * e.progress as u128) / 10_000) as u64
            {
                return Err(Error::Invalid("escrow relationships"));
            }
        }
        for (name, value) in &self.available {
            identifier(name)?;
            if *value == 0 {
                return Err(Error::Invalid("available capability"));
            }
        }
        for item in &self.pending {
            identifier(&item.capability)?;
            if item.amount == 0 || Some(item.month) != self.month.checked_add(1) {
                return Err(Error::Invalid("activation time"));
            }
        }
        self.validate_capacity()?;
        for (id, state) in &self.obligations {
            state.terms.validate(&self.ledger.accounts)?;
            if id != &state.terms.id {
                return Err(Error::Invalid("obligation key"));
            }
            for (month, paid) in &state.payments {
                if *month < state.terms.start_month
                    || *month >= state.terms.start_month + state.terms.months
                    || *month > self.month
                    || *paid == 0
                    || *paid > state.terms.monthly_due()?
                {
                    return Err(Error::Invalid("obligation payment"));
                }
            }
        }
        for (id, state) in &self.admissions {
            if state.limit.start_month == 0
                || state.limit.end_month < state.limit.start_month
                || id != &state.limit.id
                || !self.scenario.countries.contains_key(&state.limit.country)
                || !self.projection.water.contains_key(&state.limit.source)
            {
                return Err(Error::Invalid("admission references"));
            }
            for exempt in &state.grandfathered {
                if let Some(r) = self.reservations.get(exempt)
                    && (r.phase != AllocationPhase::Operational
                        || r.source != state.limit.source
                        || r.country != state.limit.country)
                {
                    return Err(Error::Invalid("admission grandfathering"));
                }
            }
        }
        for (id, todo) in &self.todos {
            if id != &todo.id {
                return Err(Error::Invalid("todo key"));
            }
            todo.validate(self.month)?;
        }
        for (id, applied) in &self.applied {
            identifier(id)?;
            if &applied.receipt.command_id != id
                || applied.receipt.month == 0
                || applied.receipt.month > self.month
            {
                return Err(Error::Invalid("command receipt"));
            }
        }
        self.validate_governance()?;
        self.validate_command_consistency()?;
        self.validate_effect_receipts()
    }
    fn validate_command_consistency(&self) -> Result<()> {
        // This checks deduplication against derived effects, without an event replay engine.
        let mut funded = BTreeMap::new();
        let mut scheduled_available = BTreeMap::<String, u128>::new();
        let mut scheduled_pending = BTreeMap::<String, u128>::new();
        for applied in self.applied.values() {
            match &applied.command {
                Command::FundEscrow {
                    escrow,
                    payer,
                    payee,
                    amount,
                } => {
                    if funded.insert(escrow, (payer, payee, amount)).is_some() {
                        return Err(Error::Invalid("duplicate escrow funding receipt"));
                    }
                }
                Command::ScheduleActivation { capability, amount } => {
                    let target = if applied.receipt.month < self.month {
                        &mut scheduled_available
                    } else {
                        &mut scheduled_pending
                    };
                    let total = target.entry(capability.clone()).or_default();
                    *total = total.checked_add(*amount as u128).ok_or(Error::Overflow)?;
                }
                _ => {}
            }
        }
        if funded.len() != self.ledger.escrows.len() {
            return Err(Error::Invalid("escrow funding receipt missing"));
        }
        for (id, e) in &self.ledger.escrows {
            if funded.get(id) != Some(&(&e.payer, &e.payee, &e.total)) {
                return Err(Error::Invalid("escrow funding mismatch"));
            }
        }
        let actual_available: BTreeMap<String, u128> = self
            .available
            .iter()
            .map(|(k, v)| (k.clone(), *v as u128))
            .collect();
        let mut actual_pending = BTreeMap::<String, u128>::new();
        for item in &self.pending {
            *actual_pending.entry(item.capability.clone()).or_default() += item.amount as u128;
        }
        if actual_available != scheduled_available || actual_pending != scheduled_pending {
            return Err(Error::Invalid("capability receipts mismatch"));
        }
        Ok(())
    }
    fn validate_effect_receipts(&self) -> Result<()> {
        let mut expected: BTreeMap<String, BTreeMap<Asset, i128>> =
            Self::initial_ledger(&self.initial)?
                .accounts
                .into_iter()
                .map(|(id, balances)| {
                    (
                        id,
                        balances
                            .into_iter()
                            .map(|(asset, value)| (asset, value as i128))
                            .collect(),
                    )
                })
                .collect();
        let mut reserved = BTreeMap::new();
        let mut released = std::collections::BTreeSet::new();
        let mut operational = std::collections::BTreeSet::new();
        let mut obligations = BTreeMap::new();
        let mut payments = BTreeMap::<(String, u32), u64>::new();
        let mut todos = BTreeMap::new();
        let mut admissions = BTreeMap::new();
        let mut progress = BTreeMap::<String, u16>::new();
        let mut cancelled = std::collections::BTreeSet::new();
        for applied in self.applied.values() {
            if !matches!(applied.command, Command::AddAdmissionLimit(_))
                && applied.admission_grandfathered.is_some()
            {
                return Err(Error::Invalid("unexpected admission snapshot"));
            }
            match &applied.command {
                Command::Transfer {
                    from,
                    to,
                    asset,
                    amount,
                } => {
                    validate_asset(asset)?;
                    if from == to || *amount == 0 {
                        return Err(Error::Invalid("transfer receipt"));
                    }
                    adjust(&mut expected, from, asset, -(*amount as i128))?;
                    adjust(&mut expected, to, asset, *amount as i128)?;
                }
                Command::Reserve(r) => {
                    if r.phase != AllocationPhase::Reserved
                        || reserved.insert(r.id.clone(), r.clone()).is_some()
                    {
                        return Err(Error::Invalid("reservation receipt"));
                    }
                    identifier(&r.id)?;
                    self.validate_reservation(r)?;
                }
                Command::Release { reservation } => {
                    if !released.insert(reservation.clone()) {
                        return Err(Error::Invalid("duplicate release receipt"));
                    }
                }
                Command::MarkOperational { reservation } => {
                    if !operational.insert(reservation.clone()) {
                        return Err(Error::Invalid("duplicate operational receipt"));
                    }
                }
                Command::RecordObligation(terms) => {
                    terms.validate(&self.ledger.accounts)?;
                    if terms.start_month < applied.receipt.month
                        || obligations
                            .insert(terms.id.clone(), terms.clone())
                            .is_some()
                    {
                        return Err(Error::Invalid("obligation receipt"));
                    }
                }
                Command::PayObligation {
                    obligation,
                    due_month,
                    amount,
                } => {
                    if *amount == 0 || *due_month > applied.receipt.month {
                        return Err(Error::Invalid("payment receipt"));
                    }
                    let sum = payments
                        .entry((obligation.clone(), *due_month))
                        .or_default();
                    *sum = sum.checked_add(*amount).ok_or(Error::Overflow)?;
                }
                Command::AddTodo(todo) => {
                    todo.validate(applied.receipt.month)?;
                    if todo.created_month != applied.receipt.month
                        || todos.insert(todo.id.clone(), todo.clone()).is_some()
                    {
                        return Err(Error::Invalid("todo receipt"));
                    }
                }
                Command::AddAdmissionLimit(limit) => {
                    if limit.start_month != applied.receipt.month
                        || self
                            .admissions
                            .get(&limit.id)
                            .map(|state| &state.grandfathered)
                            != applied.admission_grandfathered.as_ref()
                        || admissions.insert(limit.id.clone(), limit.clone()).is_some()
                    {
                        return Err(Error::Invalid("admission receipt"));
                    }
                }
                Command::ProgressEscrow {
                    escrow,
                    basis_points,
                } => {
                    if *basis_points > 10_000 {
                        return Err(Error::Invalid("progress receipt"));
                    }
                    let max = progress.entry(escrow.clone()).or_default();
                    *max = (*max).max(*basis_points);
                }
                Command::CancelEscrow { escrow } => {
                    if !cancelled.insert(escrow.clone()) {
                        return Err(Error::Invalid("cancel receipt"));
                    }
                }
                Command::FundEscrow { .. } | Command::ScheduleActivation { .. } => {}
            }
        }
        if released.iter().any(|id| !reserved.contains_key(id))
            || operational.iter().any(|id| !reserved.contains_key(id))
        {
            return Err(Error::Invalid("reservation lifecycle receipt"));
        }
        for id in &released {
            reserved.remove(id);
        }
        for id in &operational {
            if let Some(r) = reserved.get_mut(id) {
                r.phase = AllocationPhase::Operational;
            }
        }
        if reserved != self.reservations
            || todos != self.todos
            || admissions.len() != self.admissions.len()
            || obligations.len() != self.obligations.len()
        {
            return Err(Error::Invalid("descriptor receipt mismatch"));
        }
        for (id, limit) in &admissions {
            if self.admissions.get(id).map(|s| &s.limit) != Some(limit) {
                return Err(Error::Invalid("admission receipt mismatch"));
            }
        }
        for (id, state) in &self.obligations {
            if obligations.get(id) != Some(&state.terms) {
                return Err(Error::Invalid("obligation receipt mismatch"));
            }
            let mut expected_payments = BTreeMap::new();
            for ((obligation, month), paid) in &payments {
                if obligation == id {
                    expected_payments.insert(*month, *paid);
                }
            }
            if expected_payments != state.payments {
                return Err(Error::Invalid("payment receipt mismatch"));
            }
            for paid in state.payments.values() {
                adjust(
                    &mut expected,
                    &state.terms.payer,
                    &Asset::Money,
                    -(*paid as i128),
                )?;
                adjust(
                    &mut expected,
                    &state.terms.payee,
                    &Asset::Money,
                    *paid as i128,
                )?;
            }
        }
        if payments.keys().any(|(id, _)| !obligations.contains_key(id)) {
            return Err(Error::Invalid("unknown obligation receipt"));
        }
        if progress
            .keys()
            .chain(cancelled.iter())
            .any(|id| !self.ledger.escrows.contains_key(id))
        {
            return Err(Error::Invalid("unknown escrow receipt"));
        }
        for (id, e) in &self.ledger.escrows {
            if e.progress != progress.get(id).copied().unwrap_or(0)
                || e.cancelled != cancelled.contains(id)
                || (e.cancelled && e.progress == 10_000)
            {
                return Err(Error::Invalid("escrow receipt mismatch"));
            }
            adjust(&mut expected, &e.payer, &Asset::Money, -(e.total as i128))?;
            adjust(&mut expected, &e.payee, &Asset::Money, e.paid as i128)?;
            if e.cancelled {
                adjust(
                    &mut expected,
                    &e.payer,
                    &Asset::Money,
                    (e.total - e.paid) as i128,
                )?;
            }
        }
        for receipt in &self.system_receipts {
            receipt.validate(self)?;
            adjust(
                &mut expected,
                &receipt.account,
                &receipt.asset,
                receipt.delta,
            )?;
        }
        for (id, balances) in &self.ledger.accounts {
            let expected_balances = expected
                .get_mut(id)
                .ok_or(Error::Invalid("account receipts"))?;
            for (asset, value) in balances {
                if expected_balances.remove(asset).unwrap_or(0) != *value as i128 {
                    return Err(Error::Invalid("account receipt balance"));
                }
            }
            if expected_balances.values().any(|value| *value != 0) {
                return Err(Error::Invalid("missing account receipt balance"));
            }
        }
        Ok(())
    }
}
fn identifier(id: &str) -> Result<()> {
    if id.trim().is_empty() || id.len() > 256 {
        Err(Error::Invalid(
            "nonempty identifier up to 256 bytes required",
        ))
    } else {
        Ok(())
    }
}
fn validate_asset(asset: &Asset) -> Result<()> {
    if let Asset::Material(name) = asset {
        identifier(name)?;
    }
    Ok(())
}

fn adjust(
    accounts: &mut BTreeMap<String, BTreeMap<Asset, i128>>,
    id: &str,
    asset: &Asset,
    delta: i128,
) -> Result<()> {
    let balance = accounts
        .get_mut(id)
        .ok_or(Error::Invalid("receipt counterparty"))?
        .entry(asset.clone())
        .or_default();
    *balance = balance.checked_add(delta).ok_or(Error::Overflow)?;
    Ok(())
}
