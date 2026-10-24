use crate::{Error, Result};
use serde_json::Value;

const DOCUMENT: &str = include_str!("../data/jinglun-040-initial-draft.json");
const APPROVED_COMMIT: &str = "3f8bfb423d6d686da03c25b7fcab706e3ddd6af9";

pub fn number(document: &Value, pointer: &str) -> Result<f64> {
    let field = document
        .pointer(pointer)
        .ok_or(Error::Missing("initial value pointer"))?;
    let value = field.get("value").unwrap_or(field);
    let number = value
        .as_f64()
        .ok_or(Error::Invalid("initial value must be numeric"))?;
    if !number.is_finite() {
        return Err(Error::Invalid("initial value must be finite"));
    }
    Ok(number)
}

pub fn confirmed_document() -> Result<Value> {
    let document: Value = serde_json::from_str(DOCUMENT)?;
    validate(&document)?;
    Ok(document)
}

fn validate(document: &Value) -> Result<()> {
    for (pointer, expected) in [
        ("/schema", "jinglun.initial-values-draft.v2"),
        ("/scenario", "jinglun-040"),
        ("/status", "confirmed"),
        ("/activation", "enabled"),
        ("/approval/status", "confirmed"),
        ("/approval/activation", "enabled"),
        ("/approval/decision", "D022"),
        (
            "/current_design_inputs/source_hashes/DECISIONS.md",
            "479f1798f100026b4a4bc016b126bfccad0e67e8b00950e5f59f03f8eb987c24",
        ),
        (
            "/current_design_inputs/source_hashes/handoffs~12026-10-23-first-governance-slice.md",
            "a665021b83e94d0f6c006068dcabb4d4c38517abf609fdad5ea2e148220603f2",
        ),
        (
            "/current_design_inputs/scope",
            "D024/D025/D026/D027/D028/D029/D031 current confirmed inputs. D031 keeps D022 domestic demand capacity 200 and changes only bounded B1/B8/B15 acceptance. Labor and domestic capacity trials remain off and unconfirmed; D030 does not confirm the 10% labor proposal. D022 initial-value approval and all rule parameters are unchanged.",
        ),
        ("/approval/approved_commit", APPROVED_COMMIT),
        (
            "/approval/approved_document_sha256",
            "ea19c3f2dca82da57ad9380eb9fc5feee3687895102cdeb8857608195682e815",
        ),
        (
            "/approval/source_hashes/DECISIONS.md",
            "de3194c256572273e4eee98868bdd2c840ae2bbfaa8f5249cd7ab01a1b31ef7c",
        ),
        (
            "/approval/source_hashes/handoffs~12026-10-23-first-governance-slice.md",
            "fbab198b77ad12b9ec45d48db07044f61a8da2799d0274604da8d19a8f60d6e1",
        ),
        (
            "/provenance/map_sha256",
            "4c8714aaca565e0c4b932420aa320cb92a901ef63c29a897e507540db4de37a8",
        ),
        ("/provenance/generator_version", "0.6.0"),
        (
            "/provenance/generator_sha256",
            "f073efd8d6bf12b7531ccc1cac1a190266538e192598bac39ae5309e2e9266b5",
        ),
        (
            "/provenance/source_hashes/DECISIONS.md",
            "f6441d8397aca40fccd1a4eac8ad2723e1b4661902219926c85ad060a9c5d77f",
        ),
        (
            "/provenance/source_hashes/handoffs~12026-10-23-first-governance-slice.md",
            "f774e6cc009358301d61a3df84c8ed6e0ec360529612c13666f5b5d687589063",
        ),
    ] {
        if document.pointer(pointer).and_then(Value::as_str) != Some(expected) {
            return Err(Error::Invalid("initial values confirmation or provenance"));
        }
    }
    validate_activation(document)?;
    for (pointer, expected) in [
        ("/parameters/total_population", 10000.0),
        ("/parameters/opening_treasury", 55.0),
        ("/parameters/group_a_starting_cash", 50.0),
        ("/parameters/group_b_starting_cash", 55.0),
        ("/parameters/group_profit_retention_share", 0.5),
        ("/parameters/new_factory_workers", 100.0),
        ("/parameters/new_factory_output", 100.0),
        ("/parameters/new_factory_energy", 10.0),
        ("/parameters/factory_base_capital", 40.0),
        ("/parameters/operating_cash_buffer", 10.0),
        ("/parameters/monthly_transition_aid", 1.586),
        ("/parameters/d021_factory_construction_months", 3.0),
        ("/parameters/ordinary_energy_price", 0.1),
        ("/parameters/maritime_energy_price", 0.135),
        ("/parameters/long_energy_price", 0.17),
        ("/regions/39/idle", 122.0),
        ("/regions/8/idle", 19.0),
        ("/regions/29/idle", 26.0),
    ] {
        if number(document, pointer)? != expected {
            return Err(Error::Invalid("confirmed initial constant changed"));
        }
    }
    Ok(())
}

fn validate_activation(node: &Value) -> Result<()> {
    match node {
        Value::Object(object) => {
            if (object.contains_key("status")
                || object.contains_key("activation")
                || object.contains_key("value"))
                && (object.get("status").and_then(Value::as_str) != Some("confirmed")
                    || object.get("activation").and_then(Value::as_str) != Some("enabled"))
            {
                return Err(Error::Invalid("initial values not confirmed and enabled"));
            }
            for child in object.values() {
                validate_activation(child)?;
            }
        }
        Value::Array(array) => {
            for child in array {
                validate_activation(child)?;
            }
        }
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confirmed_data_can_be_read_as_fields_or_values() {
        let document = confirmed_document().unwrap();
        assert_eq!(
            number(&document, "/parameters/opening_treasury").unwrap(),
            55.0
        );
        assert_eq!(
            number(&document, "/parameters/opening_treasury/value").unwrap(),
            55.0
        );
        assert!(number(&document, "/missing").is_err());
        assert!(number(&document, "/scenario").is_err());
    }

    #[test]
    fn rejects_unapproved_or_inactive_initialization() {
        for (pointer, replacement) in [
            ("/approval/approved_commit", Value::from("other")),
            (
                "/current_design_inputs/source_hashes/DECISIONS.md",
                Value::from("stale"),
            ),
            ("/parameters/opening_treasury/value", Value::from(56)),
            (
                "/regions/8/status",
                Value::from("pending_user_confirmation"),
            ),
            ("/regions/8/activation", Value::from("disabled")),
        ] {
            let mut document = confirmed_document().unwrap();
            *document.pointer_mut(pointer).unwrap() = replacement;
            assert!(validate(&document).is_err(), "{pointer}");
        }
    }
}
