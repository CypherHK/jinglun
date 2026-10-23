use jinglun_gameplay::*;
use std::collections::BTreeMap;
fn main() -> Result<()> {
    let initial = InitialState {
        status: Confirmation::Draft,
        provenance: "Technical demonstration only; no confirmed economic starting values".into(),
        accounts: vec![
            InitialAccount {
                id: "technical-payer".into(),
                balances: BTreeMap::from([
                    (Asset::Money, 101),
                    (Asset::Material("test-material".into()), 12),
                ]),
            },
            InitialAccount {
                id: "technical-contractor".into(),
                balances: BTreeMap::new(),
            },
        ],
    };
    let mut game = Game::initialize(Rules::pending_sample(), initial)?;
    if let Some(path) = std::env::args().nth(1) {
        std::fs::write(path, serde_json::to_vec(game.map())?)
            .map_err(|_| Error::Invalid("map export path"))?;
    }
    println!("Draft initial state; Unconfirmed quota calibration; foundation mode only");
    println!(
        "Country connected components {:?}",
        game.country_components()
    );
    for id in [8, 29] {
        println!("raw cell {id}: {:?}", game.cell(id).unwrap());
    }
    game.apply(
        "fund",
        Command::FundEscrow {
            escrow: "demo".into(),
            payer: "technical-payer".into(),
            payee: "technical-contractor".into(),
            amount: 101,
        },
    )?;
    game.apply(
        "half",
        Command::ProgressEscrow {
            escrow: "demo".into(),
            basis_points: 5000,
        },
    )?;
    game.apply(
        "technical-capacity",
        Command::ScheduleActivation {
            capability: "demo-capacity".into(),
            amount: 1,
        },
    )?;
    assert_eq!(game.available("demo-capacity"), 0);
    println!("month receipt {:?}", game.end_month(1)?);
    let bytes = game.save()?;
    let mut restored = Game::load(&bytes)?;
    assert_eq!(game.map(), restored.map());
    restored.apply(
        "complete",
        Command::ProgressEscrow {
            escrow: "demo".into(),
            basis_points: 10000,
        },
    )?;
    restored.end_month(2)?;
    println!(
        "full map save {} bytes; restored month {}; recipient {}; available {}",
        bytes.len(),
        restored.month(),
        restored
            .balance("technical-contractor", &Asset::Money)
            .unwrap(),
        restored.available("demo-capacity")
    );
    Ok(())
}
