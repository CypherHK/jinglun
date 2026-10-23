use jinglun_gameplay::*;
use std::collections::BTreeMap;
use std::sync::OnceLock;

fn game() -> Game {
    static SAVE: OnceLock<Vec<u8>> = OnceLock::new();
    Game::load(SAVE.get_or_init(|| {
        Game::initialize(
            Rules::pending_sample(),
            InitialState {
                status: Confirmation::Draft,
                provenance: "technical fixtures; not confirmed economic initial values".into(),
                accounts: vec![
                    InitialAccount {
                        id: "government".into(),
                        balances: BTreeMap::from([
                            (Asset::Money, 1000),
                            (Asset::Material("test".into()), 10),
                        ]),
                    },
                    InitialAccount {
                        id: "contractor".into(),
                        balances: BTreeMap::new(),
                    },
                    InitialAccount {
                        id: "overflow".into(),
                        balances: BTreeMap::from([(Asset::Money, u64::MAX)]),
                    },
                ],
            },
        )
        .unwrap()
        .save()
        .unwrap()
    }))
    .unwrap()
}
fn reserve(
    id: &str,
    country: &str,
    cell: u32,
    source: u32,
    land: u64,
    water: u64,
    purpose: LandUse,
) -> Command {
    Command::Reserve(Reservation {
        id: id.into(),
        country: country.into(),
        cell,
        source,
        land,
        water,
        purpose,
        phase: AllocationPhase::Reserved,
    })
}
fn assert_rollback(g: &mut Game, id: &str, command: Command) {
    let before = g.save().unwrap();
    assert!(g.apply(id, command).is_err());
    assert_eq!(before, g.save().unwrap());
}
#[test]
fn full_real_map_binding_and_candidate_raw_ids() {
    let g = game();
    assert_eq!(g.map()["cells"].as_array().unwrap().len(), 100);
    assert_eq!(
        g.map()["cells"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| c["is_land"] == true)
            .count(),
        46
    );
    assert_eq!(g.country_components()["player"], 1);
    assert_eq!(g.country_components()["resource"], 1);
    assert_eq!(g.country_components()["maritime"], 2);
    let a = g.cell(8).unwrap();
    let b = g.cell(29).unwrap();
    assert!((a.buildable_area_km2 - 35456.22698884383).abs() < 1e-6);
    assert!((b.buildable_area_km2 - 708692.8406036485).abs() < 1e-6);
    assert_eq!(a.land_slots, 3);
    assert_eq!(b.land_slots, 7);
    assert_eq!(a.water_sources[&10542].1, 5);
    assert_eq!(b.water_sources[&21301].1, 3);
    assert_eq!(a.water_sources[&12587], (0.0, 0));
    assert_eq!(g.map(), Game::load(&g.save().unwrap()).unwrap().map());
}
#[test]
fn transfer_material_conservation_overflow_and_failed_id_reuse() {
    let mut g = game();
    assert_rollback(
        &mut g,
        "try",
        Command::Transfer {
            from: "government".into(),
            to: "contractor".into(),
            asset: Asset::Money,
            amount: 1001,
        },
    );
    let cmd = Command::Transfer {
        from: "government".into(),
        to: "contractor".into(),
        asset: Asset::Material("test".into()),
        amount: 7,
    };
    let receipt = g.apply("try", cmd.clone()).unwrap();
    let before = g.save().unwrap();
    assert_eq!(g.apply("try", cmd).unwrap(), receipt);
    assert_eq!(before, g.save().unwrap());
    assert_eq!(
        g.balance("government", &Asset::Material("test".into())),
        Some(3)
    );
    assert_eq!(
        g.balance("contractor", &Asset::Material("test".into())),
        Some(7)
    );
    assert_rollback(
        &mut g,
        "try",
        Command::Transfer {
            from: "government".into(),
            to: "contractor".into(),
            asset: Asset::Money,
            amount: 1,
        },
    );
    assert_rollback(
        &mut g,
        "overflow",
        Command::Transfer {
            from: "government".into(),
            to: "overflow".into(),
            asset: Asset::Money,
            amount: 1,
        },
    );
}
#[test]
fn escrow_cumulative_rounding_refund_fixed_payee_and_retry() {
    let mut g = game();
    g.apply(
        "fund",
        Command::FundEscrow {
            escrow: "project".into(),
            payer: "government".into(),
            payee: "contractor".into(),
            amount: 101,
        },
    )
    .unwrap();
    assert_eq!(g.balance("government", &Asset::Money), Some(899));
    g.apply(
        "one-third",
        Command::ProgressEscrow {
            escrow: "project".into(),
            basis_points: 3333,
        },
    )
    .unwrap();
    assert_eq!(g.balance("contractor", &Asset::Money), Some(33));
    g.apply(
        "two-thirds",
        Command::ProgressEscrow {
            escrow: "project".into(),
            basis_points: 6666,
        },
    )
    .unwrap();
    assert_eq!(g.balance("contractor", &Asset::Money), Some(67));
    assert_rollback(
        &mut g,
        "reverse",
        Command::ProgressEscrow {
            escrow: "project".into(),
            basis_points: 1,
        },
    );
    assert_rollback(
        &mut g,
        "steal",
        Command::Transfer {
            from: "project".into(),
            to: "contractor".into(),
            asset: Asset::Money,
            amount: 1,
        },
    );
    let cancel = Command::CancelEscrow {
        escrow: "project".into(),
    };
    let receipt = g.apply("cancel", cancel.clone()).unwrap();
    g.end_month(1).unwrap();
    let before = g.save().unwrap();
    assert_eq!(g.apply("cancel", cancel).unwrap(), receipt);
    assert_eq!(before, g.save().unwrap());
    assert_eq!(g.balance("government", &Asset::Money), Some(933));
    assert_eq!(g.escrow_remaining("project"), Some(0));
    assert_rollback(
        &mut g,
        "late",
        Command::ProgressEscrow {
            escrow: "project".into(),
            basis_points: 10000,
        },
    );
    g.apply(
        "fund2",
        Command::FundEscrow {
            escrow: "project2".into(),
            payer: "government".into(),
            payee: "contractor".into(),
            amount: 101,
        },
    )
    .unwrap();
    g.apply(
        "finish",
        Command::ProgressEscrow {
            escrow: "project2".into(),
            basis_points: 10000,
        },
    )
    .unwrap();
    assert_eq!(g.escrow_remaining("project2"), Some(0));
    assert_eq!(g.balance("contractor", &Asset::Money), Some(168));
}
#[test]
fn shared_sparse_source_across_countries_and_land_uses_are_atomic() {
    let mut g = game();
    let (first, second, source, slots) = (0..100)
        .filter_map(|id| g.cell(id))
        .find_map(|a| {
            (0..100).filter_map(|id| g.cell(id)).find_map(|b| {
                if a.country == b.country {
                    return None;
                }
                a.water_sources
                    .iter()
                    .find(|(s, (_, n))| *n > 0 && b.water_sources.contains_key(s))
                    .map(|(s, (_, n))| (a.clone(), b, *s, *n))
            })
        })
        .expect("real shared international source");
    assert!(source > 100);
    g.apply(
        "first",
        reserve(
            "r1",
            &first.country,
            first.raw_id,
            source,
            1,
            slots,
            LandUse::Factory,
        ),
    )
    .unwrap();
    assert_rollback(
        &mut g,
        "second",
        reserve(
            "r2",
            &second.country,
            second.raw_id,
            source,
            1,
            1,
            LandUse::Housing,
        ),
    );
    g.apply(
        "release",
        Command::Release {
            reservation: "r1".into(),
        },
    )
    .unwrap();
    g.apply(
        "second",
        reserve(
            "r2",
            &second.country,
            second.raw_id,
            source,
            1,
            slots,
            LandUse::Housing,
        ),
    )
    .unwrap();
    assert_rollback(
        &mut g,
        "wrong-country",
        reserve("wrong", "unknown", 8, 10542, 1, 0, LandUse::Factory),
    );
    assert_rollback(
        &mut g,
        "wrong-source",
        reserve("wrong", "player", 8, 999999, 1, 0, LandUse::Factory),
    );
    let mut g = game();
    g.apply(
        "factory",
        reserve("f", "player", 8, 10542, 2, 0, LandUse::Factory),
    )
    .unwrap();
    assert_rollback(
        &mut g,
        "housing",
        reserve("h", "player", 8, 10542, 2, 0, LandUse::Housing),
    );
}
#[test]
fn admission_is_separate_from_physical_and_preserves_operational_use() {
    let mut g = game();
    g.apply(
        "existing",
        reserve("existing", "player", 8, 10542, 1, 3, LandUse::Housing),
    )
    .unwrap();
    g.apply(
        "operational",
        Command::MarkOperational {
            reservation: "existing".into(),
        },
    )
    .unwrap();
    g.apply(
        "limit",
        Command::AddAdmissionLimit(AdmissionLimit {
            id: "explicit-test-restriction".into(),
            country: "player".into(),
            source: 10542,
            new_water_slots: 1,
            start_month: 1,
            end_month: 2,
        }),
    )
    .unwrap();
    assert_eq!(g.reservations()["existing"].water, 3);
    assert_rollback(
        &mut g,
        "new",
        reserve("new", "player", 8, 10542, 1, 2, LandUse::Factory),
    );
    g.apply(
        "new",
        reserve("new", "player", 8, 10542, 1, 1, LandUse::Factory),
    )
    .unwrap();
    assert_rollback(
        &mut g,
        "physical",
        reserve("physical", "player", 8, 10542, 1, 2, LandUse::Housing),
    );
    assert_eq!(
        g.save().unwrap(),
        Game::load(&g.save().unwrap()).unwrap().save().unwrap()
    );
    g.end_month(1).unwrap();
    g.end_month(2).unwrap();
    g.apply(
        "expired-admission",
        reserve("after-expiry", "player", 8, 10542, 1, 1, LandUse::Housing),
    )
    .unwrap();
    g.apply(
        "release-grandfathered",
        Command::Release {
            reservation: "existing".into(),
        },
    )
    .unwrap();
    let mut restored = Game::load(&g.save().unwrap()).unwrap();
    restored
        .apply(
            "released-capacity",
            reserve("replacement", "player", 8, 10542, 1, 3, LandUse::Factory),
        )
        .unwrap();
    assert_eq!(restored.reservations()["replacement"].water, 3);
}
#[test]
fn obligation_is_explicit_cash_never_negative_todo_deadline_does_not_move() {
    let mut g = game();
    g.apply(
        "contract",
        Command::RecordObligation(PaymentObligation {
            id: "explicit-24-month-term".into(),
            payer: "government".into(),
            payee: "contractor".into(),
            start_month: 1,
            months: 24,
            quantity_per_month: 2,
            unit_price: 3,
        }),
    )
    .unwrap();
    let todo = TodoDescriptor {
        id: "description-only".into(),
        created_month: 1,
        due_month: 2,
        explicit_default_result: "caller describes default; foundation never executes it".into(),
        priority: 10,
        options: vec!["accept".into(), "decline".into()],
    };
    g.apply("todo", Command::AddTodo(todo.clone())).unwrap();
    assert_eq!(
        g.payable("explicit-24-month-term").unwrap(),
        PayableView {
            total_committed: 144,
            paid: 0,
            unpaid: 144,
            arrears: 0
        }
    );
    assert_rollback(
        &mut g,
        "overpay",
        Command::PayObligation {
            obligation: "explicit-24-month-term".into(),
            due_month: 1,
            amount: 7,
        },
    );
    g.apply(
        "pay",
        Command::PayObligation {
            obligation: "explicit-24-month-term".into(),
            due_month: 1,
            amount: 2,
        },
    )
    .unwrap();
    g.end_month(1).unwrap();
    assert_eq!(g.payable("explicit-24-month-term").unwrap().arrears, 4);
    assert_eq!(g.balance("government", &Asset::Money), Some(998));
    g.end_month(2).unwrap();
    assert_eq!(g.todos()["description-only"], todo);
    assert_eq!(g.payable("explicit-24-month-term").unwrap().arrears, 10);
    let restored = Game::load(&g.save().unwrap()).unwrap();
    assert_eq!(restored.todos()["description-only"].due_month, 2);
    assert_eq!(restored.payable("explicit-24-month-term").unwrap().paid, 2);
}
#[test]
fn fixed_month_order_next_month_and_save_resume_match() {
    let mut continuous = game();
    continuous
        .apply(
            "marker",
            Command::ScheduleActivation {
                capability: "technical-test".into(),
                amount: 7,
            },
        )
        .unwrap();
    assert_eq!(continuous.available("technical-test"), 0);
    let mut restored = Game::load(&continuous.save().unwrap()).unwrap();
    for month in 1..=3 {
        let receipt = continuous.end_month(month).unwrap();
        assert_eq!(receipt, restored.end_month(month).unwrap());
        assert_eq!(
            receipt.stages,
            [
                "commands_escrow",
                "season_announcements_ai",
                "opening_capacity",
                "allocation",
                "production_construction",
                "livelihood_training",
                "diplomacy_security",
                "taxes_long_term",
                "archive"
            ]
        );
        assert!(receipt.foundation_mode);
    }
    assert_eq!(continuous.available("technical-test"), 7);
    assert_eq!(continuous.save().unwrap(), restored.save().unwrap());
    let before = continuous.save().unwrap();
    assert!(continuous.end_month(3).is_err());
    assert_eq!(before, continuous.save().unwrap());
}
#[test]
fn overflowing_month_activation_rolls_back_entire_month() {
    let mut g = game();
    g.apply(
        "max",
        Command::ScheduleActivation {
            capability: "technical".into(),
            amount: u64::MAX,
        },
    )
    .unwrap();
    g.end_month(1).unwrap();
    g.apply(
        "one",
        Command::ScheduleActivation {
            capability: "technical".into(),
            amount: 1,
        },
    )
    .unwrap();
    let before = g.save().unwrap();
    assert!(g.end_month(2).is_err());
    assert_eq!(g.month(), 2);
    assert_eq!(before, g.save().unwrap());
}

#[test]
fn capacity_rejection_reports_all_shortfalls_without_consuming_command_id() {
    let mut g = game();
    let before = g.save().unwrap();
    match g.apply(
        "all-gaps",
        reserve("too-large", "player", 8, 10542, 4, 6, LandUse::Factory),
    ) {
        Err(Error::CapacityShortfalls {
            land,
            water,
            admission,
        }) => {
            assert_eq!(land, BTreeMap::from([(8, 1)]));
            assert_eq!(water, BTreeMap::from([(10542, 1)]));
            assert!(admission.is_empty());
        }
        result => panic!("expected all capacity shortfalls, got {result:?}"),
    }
    assert_eq!(before, g.save().unwrap());
    g.apply(
        "all-gaps",
        reserve("fits", "player", 8, 10542, 1, 1, LandUse::Factory),
    )
    .unwrap();
}

#[test]
fn admission_starts_now_and_rejected_future_start_does_not_consume_id() {
    let mut g = game();
    let mut limit = AdmissionLimit {
        id: "current-month-only".into(),
        country: "player".into(),
        source: 10542,
        new_water_slots: 0,
        start_month: 2,
        end_month: 24,
    };
    assert_rollback(
        &mut g,
        "restriction",
        Command::AddAdmissionLimit(limit.clone()),
    );
    limit.start_month = 1;
    limit.end_month = 1;
    g.apply("restriction", Command::AddAdmissionLimit(limit.clone()))
        .unwrap();
    assert_rollback(
        &mut g,
        "new-use",
        reserve("new-use", "player", 8, 10542, 1, 1, LandUse::Housing),
    );
    let mut restored = Game::load(&g.save().unwrap()).unwrap();
    restored.end_month(1).unwrap();
    restored
        .apply(
            "new-use",
            reserve("new-use", "player", 8, 10542, 1, 1, LandUse::Housing),
        )
        .unwrap();
    restored
        .apply(
            "operational",
            Command::MarkOperational {
                reservation: "new-use".into(),
            },
        )
        .unwrap();
    restored.end_month(2).unwrap();
    limit.id = "past-start".into();
    assert_rollback(
        &mut restored,
        "past-start",
        Command::AddAdmissionLimit(limit),
    );
}

#[test]
fn water_waiting_lifecycle_replays_in_execution_order_through_release() {
    let mut g = game();
    let cell = g.cell(8).unwrap().clone();
    let source = *cell
        .water_sources
        .iter()
        .find(|(_, (_, slots))| *slots > 0)
        .unwrap()
        .0;
    g.apply(
        "z_reserve",
        reserve("factory", &cell.country, 8, source, 1, 1, LandUse::Factory),
    )
    .unwrap();
    g.apply(
        "y_wait",
        Command::SetWaterWaiting {
            reservation: "factory".into(),
            waiting: true,
        },
    )
    .unwrap();
    assert_rollback(
        &mut g,
        "invalid_activate",
        Command::MarkOperational {
            reservation: "factory".into(),
        },
    );
    g.apply(
        "x_resume",
        Command::SetWaterWaiting {
            reservation: "factory".into(),
            waiting: false,
        },
    )
    .unwrap();
    g.apply(
        "w_operate",
        Command::MarkOperational {
            reservation: "factory".into(),
        },
    )
    .unwrap();
    g.apply(
        "v_release",
        Command::Release {
            reservation: "factory".into(),
        },
    )
    .unwrap();
    let bytes = g.save().unwrap();
    assert_eq!(Game::load(&bytes).unwrap().save().unwrap(), bytes);
    // A checksum-valid save must still reject a transition before its reservation exists.
    let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    value["payload"]["applied"]["z_reserve"]["execution_order"] = 1.into();
    value["payload"]["applied"]["y_wait"]["execution_order"] = 0.into();
    let payload = serde_json::to_vec(&value["payload"]).unwrap();
    let hash = payload.iter().fold(0xcbf29ce484222325u64, |hash, byte| {
        (hash ^ *byte as u64).wrapping_mul(0x100000001b3)
    });
    value["checksum"] = format!("fnv1a64:{hash:016x}").into();
    assert!(Game::load(&serde_json::to_vec(&value).unwrap()).is_err());
}

#[test]
fn retirement_preserves_land_capacity_and_cannot_reactivate() {
    let mut g = game();
    let cell = g.cell(8).unwrap().clone();
    let source = *cell
        .water_sources
        .iter()
        .find(|(_, (_, slots))| *slots > 0)
        .unwrap()
        .0;
    g.apply(
        "reserve",
        reserve(
            "r",
            &cell.country,
            8,
            source,
            cell.land_slots,
            1,
            LandUse::Factory,
        ),
    )
    .unwrap();
    g.apply(
        "operate",
        Command::MarkOperational {
            reservation: "r".into(),
        },
    )
    .unwrap();
    g.apply(
        "retire",
        Command::Retire {
            reservation: "r".into(),
        },
    )
    .unwrap();
    assert_rollback(
        &mut g,
        "reactivate",
        Command::MarkOperational {
            reservation: "r".into(),
        },
    );
    assert_rollback(
        &mut g,
        "steal-land",
        reserve("new", &cell.country, 8, source, 1, 1, LandUse::Housing),
    );
    let bytes = g.save().unwrap();
    assert_eq!(Game::load(&bytes).unwrap().save().unwrap(), bytes);
    g.apply(
        "demolish-primitive",
        Command::Release {
            reservation: "r".into(),
        },
    )
    .unwrap();
    let bytes = g.save().unwrap();
    assert_eq!(Game::load(&bytes).unwrap().save().unwrap(), bytes);
}

#[test]
fn operational_water_suspend_resume_retire_and_explicit_protection_are_receipt_backed() {
    let mut g = game();
    let cell = g.cell(8).unwrap().clone();
    let source = *cell
        .water_sources
        .iter()
        .find(|(_, (_, slots))| *slots > 0)
        .unwrap()
        .0;
    for id in ["initial", "new"] {
        g.apply(
            format!("r/{id}"),
            reserve(id, &cell.country, 8, source, 1, 1, LandUse::Factory),
        )
        .unwrap();
        g.apply(
            format!("o/{id}"),
            Command::MarkOperational {
                reservation: id.into(),
            },
        )
        .unwrap();
    }
    g.apply(
        "suspend",
        Command::SetOperationalWaterWaiting {
            reservation: "new".into(),
            waiting: true,
        },
    )
    .unwrap();
    g.apply(
        "W",
        Command::AddAdmissionLimitProtected {
            limit: AdmissionLimit {
                id: "W".into(),
                country: cell.country.clone(),
                source,
                new_water_slots: 0,
                start_month: 1,
                end_month: 24,
            },
            protected: ["initial".into()].into_iter().collect(),
        },
    )
    .unwrap();
    assert_rollback(
        &mut g,
        "blocked-resume",
        Command::SetOperationalWaterWaiting {
            reservation: "new".into(),
            waiting: false,
        },
    );
    let bytes = g.save().unwrap();
    assert_eq!(Game::load(&bytes).unwrap().save().unwrap(), bytes);
    let mut restored = Game::load(&bytes).unwrap();
    for month in 1..=24 {
        restored.end_month(month).unwrap();
    }
    restored
        .apply(
            "resume-after-expiry",
            Command::SetOperationalWaterWaiting {
                reservation: "new".into(),
                waiting: false,
            },
        )
        .unwrap();
    assert_eq!(
        restored.reservations()["new"].phase,
        AllocationPhase::Operational
    );
    let restored_bytes = restored.save().unwrap();
    assert_eq!(
        Game::load(&restored_bytes).unwrap().save().unwrap(),
        restored_bytes
    );
    g.apply(
        "retire-suspended",
        Command::Retire {
            reservation: "new".into(),
        },
    )
    .unwrap();
    assert_rollback(
        &mut g,
        "revive",
        Command::SetOperationalWaterWaiting {
            reservation: "new".into(),
            waiting: false,
        },
    );
    let bytes = g.save().unwrap();
    assert_eq!(Game::load(&bytes).unwrap().save().unwrap(), bytes);
}
