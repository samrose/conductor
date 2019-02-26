use crate::{
    agent::actions::commit::commit_entry,
    dht::actions::remove_entry::remove_entry,
    nucleus::{
        actions::{build_validation_package::*, validate::*},
        ribosome::{api::ZomeApiResult, Runtime},
    },
    workflows::get_entry_result::get_entry_result_workflow,
};
use futures::future::{self, TryFutureExt};
use holochain_core_types::{
    cas::content::{Address, AddressableContent},
    entry::{deletion_entry::DeletionEntry, Entry},
    error::HolochainError,
    validation::{EntryAction, EntryLifecycle, ValidationData},
};
use holochain_wasm_utils::api_serialization::get_entry::*;
use std::convert::TryFrom;
use wasmi::{RuntimeArgs, RuntimeValue};

/// ZomeApiFunction::RemoveEntry function code
/// args: [0] encoded MemoryAllocation
/// Expected Address argument
/// Stores/returns a RibosomeEncodedValue
pub fn invoke_remove_entry(runtime: &mut Runtime, args: &RuntimeArgs) -> ZomeApiResult {
    let zome_call_data = runtime.zome_call_data()?;
    // deserialize args
    let args_str = runtime.load_json_string_from_args(&args);
    let try_address = Address::try_from(args_str.clone());

    // Exit on error
    if try_address.is_err() {
        zome_call_data.context.log(format!(
            "err/zome: invoke_remove_entry failed to deserialize Address: {:?}",
            args_str
        ));
        return ribosome_error_code!(ArgumentDeserializationFailed);
    }
    let deleted_entry_address = try_address.unwrap();

    // Get Current entry's latest version
    let get_args = GetEntryArgs {
        address: deleted_entry_address,
        options: Default::default(),
    };
    let maybe_entry_result = zome_call_data.context.block_on(get_entry_result_workflow(
        &zome_call_data.context,
        &get_args,
    ));
    if let Err(_err) = maybe_entry_result {
        return ribosome_error_code!(Unspecified);
    }
    let entry_result = maybe_entry_result.unwrap();
    if !entry_result.found() {
        return ribosome_error_code!(Unspecified);
    }
    let deleted_entry_address = entry_result.latest().unwrap().address();

    // Create deletion entry
    let deletion_entry = Entry::Deletion(DeletionEntry::new(deleted_entry_address.clone()));

    // Resolve future
    let result: Result<(), HolochainError> = zome_call_data.context.block_on(
        // 1. Build the context needed for validation of the entry
        build_validation_package(&deletion_entry, zome_call_data.context.clone())
            .and_then(|validation_package| {
                future::ready(Ok(ValidationData {
                    package: validation_package,
                    lifecycle: EntryLifecycle::Chain,
                    action: EntryAction::Delete,
                }))
            })
            // 2. Validate the entry
            .and_then(|validation_data| {
                validate_entry(
                    deletion_entry.clone(),
                    validation_data,
                    &zome_call_data.context,
                )
            })
            // 3. Commit the valid entry to chain and DHT
            .and_then(|_| {
                commit_entry(
                    deletion_entry.clone(),
                    Some(deleted_entry_address.clone()),
                    &zome_call_data.context,
                )
            })
            // 4. Remove the entry in DHT metadata
            .and_then(|_| {
                remove_entry(
                    &zome_call_data.context,
                    zome_call_data.context.action_channel(),
                    deleted_entry_address.clone(),
                    deletion_entry.address().clone(),
                )
            }),
    );

    runtime.store_result(result)
}
