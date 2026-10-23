use crate::*;
use std::collections::BTreeSet;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PaymentObligation {
    pub id: String,
    pub payer: String,
    pub payee: String,
    pub start_month: u32,
    pub months: u32,
    pub quantity_per_month: u64,
    pub unit_price: u64,
}
impl PaymentObligation {
    pub fn monthly_due(&self) -> Result<u64> {
        self.quantity_per_month
            .checked_mul(self.unit_price)
            .ok_or(Error::Overflow)
    }
    pub(crate) fn validate(&self, accounts: &BTreeMap<String, BTreeMap<Asset, u64>>) -> Result<()> {
        identifier(&self.id)?;
        if self.start_month == 0
            || self.months == 0
            || self.payer == self.payee
            || !accounts.contains_key(&self.payer)
            || !accounts.contains_key(&self.payee)
        {
            return Err(Error::Invalid("obligation parties or term"));
        }
        self.start_month
            .checked_add(self.months)
            .ok_or(Error::Overflow)?;
        let due = self.monthly_due()?;
        if due == 0 {
            return Err(Error::Invalid("nonzero obligation required"));
        }
        due.checked_mul(self.months as u64).ok_or(Error::Overflow)?;
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ObligationState {
    pub terms: PaymentObligation,
    pub payments: BTreeMap<u32, u64>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PayableView {
    pub total_committed: u64,
    pub paid: u64,
    pub unpaid: u64,
    pub arrears: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum AllocationPhase {
    Reserved,
    WaitingForWater,
    Operational,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AdmissionLimit {
    pub id: String,
    pub country: String,
    pub source: u32,
    pub new_water_slots: u64,
    pub start_month: u32,
    pub end_month: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct AdmissionState {
    pub limit: AdmissionLimit,
    pub grandfathered: BTreeSet<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TodoDescriptor {
    pub id: String,
    pub created_month: u32,
    pub due_month: u32,
    pub explicit_default_result: String,
    pub priority: u32,
    pub options: Vec<String>,
}
impl TodoDescriptor {
    pub(crate) fn validate(&self, current_month: u32) -> Result<()> {
        identifier(&self.id)?;
        identifier(&self.explicit_default_result)?;
        if self.created_month == 0
            || self.created_month > current_month
            || self.due_month < self.created_month
            || self.options.is_empty()
        {
            return Err(Error::Invalid("todo dates or options"));
        }
        let mut unique = BTreeSet::new();
        for option in &self.options {
            identifier(option)?;
            if !unique.insert(option) {
                return Err(Error::Invalid("duplicate todo option"));
            }
        }
        Ok(())
    }
}
