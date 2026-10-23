use crate::*;

const SCHEMA_VERSION: u32 = 1;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Save {
    schema_version: u32,
    generator_version: String,
    scenario_version: String,
    rules_version: String,
    // FNV-1a detects accidental corruption; it is not a security signature.
    checksum: String,
    payload: Snapshot,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Snapshot {
    map: Value,
    scenario: Scenario,
    rules: Rules,
    initial: InitialState,
    ledger: Ledger,
    reservations: BTreeMap<String, Reservation>,
    month: u32,
    pending: Vec<Activation>,
    available: BTreeMap<String, u64>,
    applied: BTreeMap<String, Applied>,
    obligations: BTreeMap<String, ObligationState>,
    admissions: BTreeMap<String, AdmissionState>,
    todos: BTreeMap<String, TodoDescriptor>,
}
fn checksum(snapshot: &Snapshot) -> Result<String> {
    // Canonical Value ordering also makes input object key order immaterial.
    let bytes = serde_json::to_vec(&serde_json::to_value(snapshot)?)?;
    let hash = bytes.iter().fold(0xcbf29ce484222325u64, |hash, byte| {
        (hash ^ *byte as u64).wrapping_mul(0x100000001b3)
    });
    Ok(format!("fnv1a64:{hash:016x}"))
}
impl Game {
    pub fn save(&self) -> Result<Vec<u8>> {
        self.validate()?;
        let payload = Snapshot {
            map: self.map.clone(),
            scenario: self.scenario.clone(),
            rules: self.rules.clone(),
            initial: self.initial.clone(),
            ledger: self.ledger.clone(),
            reservations: self.reservations.clone(),
            month: self.month,
            pending: self.pending.clone(),
            available: self.available.clone(),
            applied: self.applied.clone(),
            obligations: self.obligations.clone(),
            admissions: self.admissions.clone(),
            todos: self.todos.clone(),
        };
        let save = Save {
            schema_version: SCHEMA_VERSION,
            generator_version: GENERATOR_VERSION.into(),
            scenario_version: self.scenario.version.clone(),
            rules_version: self.rules.version.clone(),
            checksum: checksum(&payload)?,
            payload,
        };
        Ok(serde_json::to_vec(&save)?)
    }
    pub fn load(bytes: &[u8]) -> Result<Self> {
        let save: Save = serde_json::from_slice(bytes)?;
        if save.schema_version != SCHEMA_VERSION
            || save.generator_version != GENERATOR_VERSION
            || save.scenario_version != SCENARIO_VERSION
            || save.rules_version != save.payload.rules.version
        {
            return Err(Error::Invalid("unsupported save versions"));
        }
        if save.checksum != checksum(&save.payload)? {
            return Err(Error::Invalid("save checksum"));
        }
        let s = save.payload;
        s.rules.validate()?;
        let projection = scenario::project(&s.map, &s.scenario, &s.rules)?;
        let game = Self {
            map: s.map,
            scenario: s.scenario,
            rules: s.rules,
            initial: s.initial,
            projection,
            ledger: s.ledger,
            reservations: s.reservations,
            month: s.month,
            pending: s.pending,
            available: s.available,
            applied: s.applied,
            obligations: s.obligations,
            admissions: s.admissions,
            todos: s.todos,
        };
        game.validate()?;
        Ok(game)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Game {
        Game::initialize(
            Rules::pending_sample(),
            InitialState {
                status: Confirmation::Draft,
                provenance: "technical test only".into(),
                accounts: vec![
                    InitialAccount {
                        id: "payer".into(),
                        balances: BTreeMap::from([(Asset::Money, 100)]),
                    },
                    InitialAccount {
                        id: "payee".into(),
                        balances: BTreeMap::new(),
                    },
                ],
            },
        )
        .unwrap()
    }
    fn rewrite(bytes: &[u8], change: impl FnOnce(&mut Snapshot)) -> Vec<u8> {
        let mut save: Save = serde_json::from_slice(bytes).unwrap();
        change(&mut save.payload);
        save.checksum = checksum(&save.payload).unwrap();
        serde_json::to_vec(&save).unwrap()
    }
    #[test]
    fn unknown_natural_fields_roundtrip_and_invariants_are_validated_after_checksum() {
        let game = fixture();
        let bytes = rewrite(&game.save().unwrap(), |s| {
            s.map["future_nature_field"] =
                serde_json::json!({"nested": [0.12345678901234567, "preserve me"]});
        });
        let loaded = Game::load(&bytes).unwrap();
        let full_map = loaded.map().clone();
        assert_eq!(
            Game::load(&loaded.save().unwrap()).unwrap().map(),
            &full_map
        );
        for bad in [
            rewrite(&bytes, |s| s.month = 0),
            rewrite(&bytes, |s| {
                s.ledger
                    .accounts
                    .get_mut("payer")
                    .unwrap()
                    .insert(Asset::Money, 101);
            }),
            rewrite(&bytes, |s| {
                s.map["cells"][8]["water_source_ids"] = serde_json::json!([999999]);
            }),
            rewrite(&bytes, |s| {
                s.pending.push(Activation {
                    capability: "test".into(),
                    amount: 1,
                    month: 0,
                })
            }),
            rewrite(&bytes, |s| s.rules.land_km2_per_slot = -1.0),
            rewrite(&bytes, |s| {
                s.rules.land_min = 9;
                s.rules.land_max = 1;
            }),
        ] {
            assert!(Game::load(&bad).is_err());
        }
        let ledger_without_receipt = rewrite(&bytes, |s| {
            s.ledger
                .accounts
                .get_mut("payer")
                .unwrap()
                .insert(Asset::Money, 99);
            s.ledger
                .accounts
                .get_mut("payee")
                .unwrap()
                .insert(Asset::Money, 1);
        });
        assert!(Game::load(&ledger_without_receipt).is_err());
        let mut escrow = fixture();
        escrow
            .apply(
                "fund",
                Command::FundEscrow {
                    escrow: "project".into(),
                    payer: "payer".into(),
                    payee: "payee".into(),
                    amount: 100,
                },
            )
            .unwrap();
        let wrong_party = rewrite(&escrow.save().unwrap(), |s| {
            s.ledger.escrows.get_mut("project").unwrap().payee = "missing".into();
        });
        assert!(Game::load(&wrong_party).is_err());
        let missing_receipt = rewrite(&escrow.save().unwrap(), |s| {
            s.applied.clear();
        });
        assert!(Game::load(&missing_receipt).is_err());
        let mut activation = fixture();
        activation
            .apply(
                "schedule",
                Command::ScheduleActivation {
                    capability: "technical".into(),
                    amount: 1,
                },
            )
            .unwrap();
        let early = rewrite(&activation.save().unwrap(), |s| {
            s.pending.clear();
            s.available.insert("technical".into(), 1);
        });
        assert!(Game::load(&early).is_err());
        let mut value: Value = serde_json::from_slice(&bytes).unwrap();
        value["schema_version"] = 99.into();
        assert!(Game::load(&serde_json::to_vec(&value).unwrap()).is_err());
    }
}
