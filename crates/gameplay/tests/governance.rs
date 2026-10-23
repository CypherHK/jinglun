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
