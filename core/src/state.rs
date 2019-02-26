use crate::{
    action::ActionWrapper,
    agent::{
        chain_store::ChainStore,
        state::{AgentState, AgentStateSnapshot},
    },
    context::Context,
    dht::dht_store::DhtStore,
    network::state::NetworkState,
    nucleus::state::NucleusState,
};
use holochain_core_types::{
    cas::{
        content::{Address, AddressableContent},
        storage::ContentAddressableStorage,
    },
    chain_header::ChainHeader,
    dna::Dna,
    eav::{Attribute, EaviQuery, IndexFilter},
    entry::{entry_type::EntryType, Entry},
    error::{HcResult, HolochainError},
};
use std::{
    collections::HashSet,
    convert::TryInto,
    sync::{Arc, RwLock},
};

/// The Store of the Holochain instance Object, according to Redux pattern.
/// It's composed of all sub-module's state slices.
/// To plug in a new module, its state slice needs to be added here.
#[derive(Clone, PartialEq, Debug)]
pub struct State {
    nucleus: Arc<NucleusState>,
    agent: Arc<AgentState>,
    dht: Arc<DhtStore>,
    network: Arc<NetworkState>,
    // @TODO eventually drop stale history
    // @see https://github.com/holochain/holochain-rust/issues/166
    pub history: HashSet<ActionWrapper>,
}

impl State {
    pub fn new(context: Arc<Context>) -> Self {
        // @TODO file table
        // @see https://github.com/holochain/holochain-rust/pull/246

        let chain_cas = &(*context).chain_storage;
        let dht_cas = &(*context).dht_storage;
        let eav = context.eav_storage.clone();
        State {
            nucleus: Arc::new(NucleusState::new()),
            agent: Arc::new(AgentState::new(ChainStore::new(chain_cas.clone()))),
            dht: Arc::new(DhtStore::new(dht_cas.clone(), eav)),
            network: Arc::new(NetworkState::new()),
            history: HashSet::new(),
        }
    }

    pub fn new_with_agent(context: Arc<Context>, agent_state: Arc<AgentState>) -> Self {
        // @TODO file table
        // @see https://github.com/holochain/holochain-rust/pull/246

        let cas = context.dht_storage.clone();
        let eav = context.eav_storage.clone();

        fn get_dna(
            agent_state: &Arc<AgentState>,
            cas: Arc<RwLock<dyn ContentAddressableStorage>>,
        ) -> HcResult<Dna> {
            let dna_entry_header = agent_state
                .chain_store()
                .iter_type(&agent_state.top_chain_header(), &EntryType::Dna)
                .last()
                .ok_or(HolochainError::ErrorGeneric(
                    "No DNA entry found in source chain while creating state from agent"
                        .to_string(),
                ))?;
            let json = (*cas.read().unwrap()).fetch(dna_entry_header.entry_address())?;
            let entry: Entry =
                json.map(|e| e.try_into())
                    .ok_or(HolochainError::ErrorGeneric(
                        "No DNA entry found in storage while creating state from agent".to_string(),
                    ))??;
            match entry {
                Entry::Dna(dna) => Ok(dna),
                _ => Err(HolochainError::SerializationError(
                    "Tried to get Dna from non-Dna Entry".into(),
                )),
            }
        }

        let mut nucleus_state = NucleusState::new();
        nucleus_state.dna = get_dna(&agent_state, cas.clone()).ok();
        State {
            nucleus: Arc::new(nucleus_state),
            agent: agent_state,
            dht: Arc::new(DhtStore::new(cas.clone(), eav.clone())),
            network: Arc::new(NetworkState::new()),
            history: HashSet::new(),
        }
    }

    pub fn reduce(&self, context: Arc<Context>, action_wrapper: ActionWrapper) -> Self {
        let mut new_state = State {
            nucleus: crate::nucleus::reduce(
                Arc::clone(&context),
                Arc::clone(&self.nucleus),
                &action_wrapper,
            ),
            agent: crate::agent::state::reduce(
                Arc::clone(&context),
                Arc::clone(&self.agent),
                &action_wrapper,
            ),
            dht: crate::dht::dht_reducers::reduce(
                Arc::clone(&context),
                Arc::clone(&self.dht),
                &action_wrapper,
            ),
            network: crate::network::reducers::reduce(
                Arc::clone(&context),
                Arc::clone(&self.network),
                &action_wrapper,
            ),
            history: self.history.clone(),
        };

        new_state.history.insert(action_wrapper);
        new_state
    }

    pub fn nucleus(&self) -> Arc<NucleusState> {
        Arc::clone(&self.nucleus)
    }

    pub fn agent(&self) -> Arc<AgentState> {
        Arc::clone(&self.agent)
    }

    pub fn dht(&self) -> Arc<DhtStore> {
        Arc::clone(&self.dht)
    }

    pub fn network(&self) -> Arc<NetworkState> {
        Arc::clone(&self.network)
    }

    pub fn try_from_agent_snapshot(
        context: Arc<Context>,
        snapshot: AgentStateSnapshot,
    ) -> HcResult<State> {
        let agent_state = AgentState::new_with_top_chain_header(
            ChainStore::new(context.dht_storage.clone()),
            snapshot.top_chain_header().clone(),
        );
        Ok(State::new_with_agent(
            context.clone(),
            Arc::new(agent_state),
        ))
    }

    /// Get all headers for an entry by first looking in the DHT meta store
    /// for header addresses, then resolving them with the DHT CAS
    pub fn get_headers(&self, entry_address: Address) -> Result<Vec<ChainHeader>, HolochainError> {
        let headers: Vec<ChainHeader> = self
            .agent()
            .iter_chain()
            .filter(|h| h.entry_address() == &entry_address)
            .collect();
        let header_addresses: Vec<Address> = headers.iter().map(|h| h.address()).collect();
        let mut dht_headers = self
            .dht()
            .meta_storage()
            .read()
            .unwrap()
            // fetch all EAV references to chain headers for this entry
            .fetch_eavi(&EaviQuery::new(
                Some(entry_address).into(),
                Some(Attribute::EntryHeader).into(),
                None.into(),
                IndexFilter::LatestByAttribute,
            ))?
            .into_iter()
            // get the header addresses
            .map(|eavi| eavi.value())
            // don't include the chain header twice
            .filter(|a| !header_addresses.contains(a))
            // fetch the header content from CAS
            .map(|a| self.dht().content_storage().read().unwrap().fetch(&a))
            // rearrange
            .collect::<Result<Vec<Option<_>>, _>>()
            .map(|r| {
                r.into_iter()
                    // ignore None values
                    .flatten()
                    .map(|content| ChainHeader::try_from_content(&content))
                    .collect::<Result<Vec<_>, _>>()
            })??;
        {
            let mut all_headers = headers;
            all_headers.append(&mut dht_headers);
            Ok(all_headers)
        }
    }
}

pub fn test_store(context: Arc<Context>) -> State {
    State::new(context)
}
