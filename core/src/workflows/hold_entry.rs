use crate::{
    context::Context,
    dht::actions::hold::hold_entry,
    network::{
        actions::get_validation_package::get_validation_package, entry_with_header::EntryWithHeader,
    },
    nucleus::actions::validate::validate_entry,
};

use holochain_core_types::{
    cas::content::Address,
    error::HolochainError,
    validation::{EntryAction, EntryLifecycle, ValidationData},
};
use std::sync::Arc;

pub async fn hold_entry_workflow<'a>(
    entry_with_header: EntryWithHeader,
    context: Arc<Context>,
) -> Result<Address, HolochainError> {
    let EntryWithHeader { entry, header } = &entry_with_header;

    // 1. Get validation package from source
    let maybe_validation_package = await!(get_validation_package(header.clone(), &context))?;
    let validation_package = maybe_validation_package
        .ok_or("Could not get validation package from source".to_string())?;

    // 2. Create validation data struct
    let validation_data = ValidationData {
        package: validation_package,
        lifecycle: EntryLifecycle::Dht,
        action: EntryAction::Create,
    };

    // 3. Validate the entry
    await!(validate_entry(entry.clone(), validation_data, &context))?;

    // 3. If valid store the entry in the local DHT shard
    await!(hold_entry(entry_with_header, context))
}

#[cfg(test)]
// too slow!
#[cfg(feature = "broken-tests")]
pub mod tests {
    use super::*;
    use crate::{
        network::test_utils::*, nucleus::actions::tests::*, workflows::author_entry::author_entry,
    };
    use futures::executor::block_on;
    use holochain_core_types::entry::test_entry;
    use test_utils::*;

    #[test]
    /// Test that an invalid entry will be rejected by this workflow.
    ///
    /// This test simulates an attack where a node is changing its local copy of the DNA to
    /// allow otherwise invalid entries while spoofing the unmodified dna_address.
    ///
    /// hold_entry_workflow is then expected to fail in its validation step
    fn test_reject_invalid_entry_on_hold_workflow() {
        // Hacked DNA that regards everything as valid
        let hacked_dna =
            create_test_dna_with_wat("test_zome", "test_cap", Some(&test_wat_always_valid()));
        // Original DNA that regards nothing as valid
        let mut dna =
            create_test_dna_with_wat("test_zome", "test_cap", Some(&test_wat_always_invalid()));
        dna.uuid = String::from("test_reject_invalid_entry_on_hold_workflow");

        // Address of the original DNA
        let dna_address = dna.address();

        let (_, context1) =
            test_instance_with_spoofed_dna(hacked_dna, dna_address, "alice").unwrap();
        let (_instance2, context2) = instance_by_name("jack", dna);

        // Commit entry on attackers node
        let entry = test_entry();
        let _entry_address = context1
            .block_on(author_entry(&entry, None, &context1))
            .unwrap();

        // Get header which we need to trigger hold_entry_workflow
        let agent1_state = context1.state().unwrap().agent();
        let header = agent1_state
            .get_most_recent_header_for_entry(&entry)
            .expect("There must be a header in the author's source chain after commit");
        let entry_with_header = EntryWithHeader { entry, header };

        // Call hold_entry_workflow on victim DHT node
        let result = context2.block_on(hold_entry_workflow(&entry_with_header, &context2));

        // ... and expect validation to fail with message defined in test WAT:
        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap(),
            HolochainError::ValidationFailed(String::from("FAIL wat")),
        );
    }
}
