use jinglun_gameplay::*;
use std::sync::OnceLock;
fn fixture() -> Game {
    static GAME: OnceLock<Game> = OnceLock::new();
    GAME.get_or_init(|| Game::confirmed_game().unwrap()).clone()
}
#[test]
fn passive_game_runs_real_nine_stages_and_load_replays_business() {
    let mut game = fixture();
    let original = game.map().clone();
    assert_eq!(
        game.balance("treasury", &Asset::Money),
        Some(money(55.0).unwrap())
    );
    for month in 1..=24 {
        let receipt = game.end_month(month).unwrap();
        assert!(!receipt.foundation_mode);
        assert_eq!(receipt.stages.len(), 9);
        if month == 12 {
            game = Game::load(&game.save().unwrap()).unwrap();
        }
    }
    let s = game.governance().unwrap();
    let report = s.report.as_ref().unwrap();
    println!(
        "B2 {:?}, treasury={}, recurring={:?}, factories={:?}",
        report.grade,
        currency(game.balance("treasury", &Asset::Money).unwrap()),
        report.final_three_recurring,
        s.factories
            .values()
            .map(|f| (f.site, f.started_month, f.phase))
            .collect::<Vec<_>>()
    );
    assert_eq!(report.grade, GoalGrade::NotAchieved);
    assert!(!report.fiscal_recovered);
    assert!(game.balance("treasury", &Asset::Money).unwrap() > 0);
    assert_eq!(game.map(), &original);
    assert!(!game.system_receipts().is_empty());
    let reloaded = Game::load(&game.save().unwrap()).unwrap();
    assert_eq!(reloaded.save().unwrap(), game.save().unwrap());
}
#[test]
fn plans_preview_and_undo_are_pure_and_failed_commands_atomic() {
    let mut game = fixture();
    let initial = game.save().unwrap();
    let before = game.scorecard(Group::A, 8).unwrap();
    let forecast = game.treasury_forecast().unwrap();
    game.queue_governance(
        "sub",
        GovernanceCommand::SetSubsidy {
            site: 8,
            cap: money(40.0).unwrap(),
        },
    )
    .unwrap();
    let after = game.scorecard(Group::A, 8).unwrap();
    assert!(after.gaps["cash"] < before.gaps["cash"]);
    game.withdraw_governance("sub").unwrap();
    assert_eq!(game.scorecard(Group::A, 8).unwrap(), before);
    assert_eq!(game.treasury_forecast().unwrap(), forecast);
    assert_eq!(game.save().unwrap(), initial);
    assert!(
        game.queue_governance("bad", GovernanceCommand::OpenRegion { site: 2 })
            .is_err()
    );
    assert_eq!(game.save().unwrap(), initial);
}

#[test]
fn opening_sources_and_material_owners_are_explicit() {
    let game = fixture();
    for site in [8, 29, 39, 74] {
        let view = game.cell(site).unwrap();
        println!(
            "site {site} land={} nonzero_sources={} total_water_slots={}",
            view.land_slots,
            view.water_sources
                .values()
                .filter(|(_, slots)| *slots > 0)
                .count(),
            view.water_sources
                .values()
                .map(|(_, slots)| *slots)
                .sum::<u64>()
        );
    }
    assert_eq!(
        game.balance("households", &Asset::Material("food".into())),
        Some(20)
    );
    assert_eq!(
        game.balance("capital_inventory", &Asset::Material("construction".into())),
        Some(12)
    );
    assert_eq!(
        game.balance("treasury", &Asset::Material("food".into())),
        Some(0)
    );
}

#[test]
fn water_renewal_continues_work_but_waits_for_water_before_activation() {
    let calibration = GovernanceCalibration {
        water_priority_fraction: 1.0,
        ..Default::default()
    };
    let mut game = Game::confirmed_game_with_calibration(calibration).unwrap();
    for month in 1..=7 {
        game.end_month(month).unwrap();
    }
    game.queue_governance(
        "house",
        GovernanceCommand::BuildPublic {
            id: "house".into(),
            site: 8,
            kind: PublicWork::Housing,
            road_edge: None,
        },
    )
    .unwrap();
    game.end_month(8).unwrap();
    assert_eq!(
        game.governance().unwrap().projects["house"].worked_months,
        1
    );
    game.queue_governance(
        "renew",
        GovernanceCommand::RenewAid {
            choice: RenewalChoice::WaterPriority,
        },
    )
    .unwrap();
    game.end_month(9).unwrap();
    let s = game.governance().unwrap();
    assert_eq!(s.projects["house"].worked_months, 2);
    assert!(!s.projects["house"].active);
    assert_eq!(
        game.reservations()["public/house"].phase,
        AllocationPhase::WaitingForWater
    );
    let paid_before = game.escrow_remaining("public/house").unwrap();
    assert_eq!(paid_before, 0);
    game.end_month(10).unwrap();
    assert!(!game.governance().unwrap().projects["house"].active);
    game = Game::load(&game.save().unwrap()).unwrap();
    for month in 11..=25 {
        game.end_month(month).unwrap();
    }
    assert!(game.governance().unwrap().projects["house"].active);
    assert_eq!(
        game.reservations()["public/house"].phase,
        AllocationPhase::Operational
    );
}

#[test]
fn energy_long_term_has_gross_payments_unexpired_liability_and_private_resale() {
    let mut game = fixture();
    for month in 1..=8 {
        game.end_month(month).unwrap();
    }
    game.queue_governance(
        "renew",
        GovernanceCommand::RenewAid {
            choice: RenewalChoice::EnergyLongTerm,
        },
    )
    .unwrap();
    game.end_month(9).unwrap();
    let obligation = game.payable("resource_energy_long_term").unwrap();
    assert_eq!(
        obligation.total_committed,
        money(20.0 * 0.17 * 24.0).unwrap()
    );
    assert_eq!(obligation.paid, money(3.4).unwrap());
    assert_eq!(
        game.governance().unwrap().last_energy_unused_bill,
        money(3.4).unwrap()
    );
    for month in 10..=24 {
        game.end_month(month).unwrap();
    }
    let report = game.governance().unwrap().report.as_ref().unwrap();
    assert_eq!(report.future_energy_liability, money(3.4 * 8.0).unwrap());
    assert_eq!(
        report.treasury_net_energy_liability,
        game.balance("treasury", &Asset::Money).unwrap() as i128
            - game.payable("resource_energy_long_term").unwrap().unpaid as i128
    );
    assert_eq!(
        Game::load(&game.save().unwrap()).unwrap().save().unwrap(),
        game.save().unwrap()
    );
}

fn rewrite_checked_save(game: &Game, change: impl FnOnce(&mut serde_json::Value)) -> Vec<u8> {
    let mut value: serde_json::Value = serde_json::from_slice(&game.save().unwrap()).unwrap();
    change(&mut value["payload"]);
    let bytes = serde_json::to_vec(&value["payload"]).unwrap();
    let hash = bytes.iter().fold(0xcbf29ce484222325u64, |hash, byte| {
        (hash ^ *byte as u64).wrapping_mul(0x100000001b3)
    });
    value["checksum"] = format!("fnv1a64:{hash:016x}").into();
    serde_json::to_vec(&value).unwrap()
}
#[test]
fn business_replay_rejects_balanced_counterfeit_receipts_and_plausible_state_changes() {
    let mut game = fixture();
    game.end_month(1).unwrap();
    let state_tamper = rewrite_checked_save(&game, |payload| {
        payload["governance"]["security"] = 63.0.into()
    });
    assert!(Game::load(&state_tamper).is_err());
    let balanced_tamper = rewrite_checked_save(&game, |payload| {
        let receipts = payload["system_receipts"].as_array_mut().unwrap();
        let receipt = receipts
            .iter_mut()
            .find(|r| r["reason"] == "existing_nonmanufacturing_employer_wages")
            .unwrap();
        let old = receipt["delta"].as_i64().unwrap();
        receipt["delta"] = (old + 1).into();
        receipt["external_counterbalance"] = (-(old + 1)).into();
        let cash = payload["ledger"]["accounts"]["households"]["money"]
            .as_u64()
            .unwrap();
        payload["ledger"]["accounts"]["households"]["money"] = (cash + 1).into();
    });
    assert!(Game::load(&balanced_tamper).is_err());
}
#[test]
fn currency_quantization_is_central_and_checked() {
    assert_eq!(money(0.0000000049).unwrap(), 0);
    assert_eq!(money(0.000000005).unwrap(), 1);
    assert_eq!(currency(money(55.0).unwrap()), 55.0);
    for invalid in [f64::NAN, f64::INFINITY, -0.1, u64::MAX as f64] {
        assert!(money(invalid).is_err());
    }
}
