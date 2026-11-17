//! PayPerCall endpoint registry.
//!
//! A public, permissionless list of paid APIs. Each listing says what the API
//! does, what one call costs (in USDC base units, 7 decimals) and which Stellar
//! address gets paid, so AI agents can discover paid APIs straight from the
//! chain instead of from a central marketplace.
//!
//! - Anyone can register a listing; the registering address becomes its seller.
//! - Only the seller can update, pause or remove their listing.
//! - An admin can remove abusive listings and upgrade the contract.
//!
//! The listing only stores the public, paid endpoint (the gateway URL). The
//! seller's real upstream URL never goes on-chain.
#![no_std]

use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, Address, BytesN,
    ContractExecutable, Env, String, Vec,
};

#[cfg(test)]
mod test;

// ---------- limits ----------

pub const ID_MIN_LEN: u32 = 2;
pub const ID_MAX_LEN: u32 = 63;
pub const NAME_MAX_LEN: u32 = 80;
pub const DESCRIPTION_MAX_LEN: u32 = 500;
pub const ENDPOINT_MAX_LEN: u32 = 256;
pub const CATEGORY_MAX_LEN: u32 = 32;
/// Max listings one seller can hold, which keeps the per-seller index small.
pub const MAX_LISTINGS_PER_SELLER: u32 = 50;
/// Max listings returned by one `list` call.
pub const MAX_PAGE_SIZE: u32 = 50;

// ---------- storage lifetime (in ledgers, ~5s each) ----------

const DAY_IN_LEDGERS: u32 = 17_280;
const TTL_THRESHOLD: u32 = 7 * DAY_IN_LEDGERS;
const TTL_EXTEND_TO: u32 = 30 * DAY_IN_LEDGERS;

// ---------- types ----------

/// One paid API, as stored on-chain.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Listing {
    /// URL-safe id, unique in the registry (same rules as the gateway).
    pub id: String,
    /// Address that registered the listing and can change it.
    pub seller: Address,
    /// Address that receives each payment.
    pub pay_to: Address,
    pub name: String,
    pub description: String,
    /// Public paid URL an agent calls, e.g. https://gw.example.com/v1/call/<id>.
    pub endpoint: String,
    /// Price per call in USDC base units (1 USDC = 10_000_000).
    pub price: i128,
    /// Optional free-form category, e.g. "data" or "finance". May be empty.
    pub category: String,
    /// Paused listings stay in the registry but should not be called.
    pub active: bool,
    pub created_ledger: u32,
    pub updated_ledger: u32,
}

/// The fields a seller sets when registering or updating a listing.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ListingInput {
    pub name: String,
    pub description: String,
    pub endpoint: String,
    pub price: i128,
    pub pay_to: Address,
    pub category: String,
}

#[contracttype]
#[derive(Clone)]
enum DataKey {
    Admin,
    Count,
    Listing(String),
    /// Position in the global index -> listing id.
    IdAt(u32),
    /// Listing id -> position in the global index.
    Position(String),
    /// Seller -> ids of their listings.
    Seller(Address),
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    AlreadyExists = 1,
    NotFound = 2,
    InvalidId = 3,
    InvalidPrice = 4,
    InvalidName = 5,
    InvalidDescription = 6,
    InvalidEndpoint = 7,
    InvalidCategory = 8,
    TooManyListings = 9,
}

// ---------- events ----------

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Registered {
    #[topic]
    pub id: String,
    #[topic]
    pub seller: Address,
    pub price: i128,
    pub pay_to: Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Updated {
    #[topic]
    pub id: String,
    pub price: i128,
    pub pay_to: Address,
    pub active: bool,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Removed {
    #[topic]
    pub id: String,
    /// The seller, or the admin when it was a moderation removal.
    pub by: Address,
}

// ---------- contract ----------

#[contract]
pub struct Registry;

#[contractimpl]
impl Registry {
    pub fn __constructor(env: Env, admin: Address) {
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Count, &0u32);
    }

    // ----- seller actions -----

    /// Add a new paid API. `seller` must sign and becomes the listing's owner.
    pub fn register(
        env: Env,
        seller: Address,
        id: String,
        input: ListingInput,
    ) -> Result<Listing, Error> {
        seller.require_auth();
        check_id(&id)?;
        check_input(&input)?;

        let key = DataKey::Listing(id.clone());
        if env.storage().persistent().has(&key) {
            return Err(Error::AlreadyExists);
        }

        let mut ids = seller_ids(&env, &seller);
        if ids.len() >= MAX_LISTINGS_PER_SELLER {
            return Err(Error::TooManyListings);
        }

        let now = env.ledger().sequence();
        let listing = Listing {
            id: id.clone(),
            seller: seller.clone(),
            pay_to: input.pay_to,
            name: input.name,
            description: input.description,
            endpoint: input.endpoint,
            price: input.price,
            category: input.category,
            active: true,
            created_ledger: now,
            updated_ledger: now,
        };
        save_listing(&env, &listing);

        // Global index, so `list` can page through every listing.
        let count = count(&env);
        set_persistent(&env, &DataKey::IdAt(count), &id);
        set_persistent(&env, &DataKey::Position(id.clone()), &count);
        env.storage().instance().set(&DataKey::Count, &(count + 1));

        ids.push_back(id.clone());
        set_persistent(&env, &DataKey::Seller(seller.clone()), &ids);
        bump_instance(&env);

        Registered {
            id,
            seller,
            price: listing.price,
            pay_to: listing.pay_to.clone(),
        }
        .publish(&env);
        Ok(listing)
    }

    /// Change a listing's details. Only its seller can do this.
    pub fn update(env: Env, id: String, input: ListingInput) -> Result<Listing, Error> {
        let mut listing = load_listing(&env, &id)?;
        listing.seller.require_auth();
        check_input(&input)?;

        listing.name = input.name;
        listing.description = input.description;
        listing.endpoint = input.endpoint;
        listing.price = input.price;
        listing.pay_to = input.pay_to;
        listing.category = input.category;
        listing.updated_ledger = env.ledger().sequence();
        save_listing(&env, &listing);
        publish_updated(&env, &listing);
        Ok(listing)
    }

    /// Pause (`false`) or resume (`true`) a listing. Only its seller can do this.
    pub fn set_active(env: Env, id: String, active: bool) -> Result<Listing, Error> {
        let mut listing = load_listing(&env, &id)?;
        listing.seller.require_auth();

        listing.active = active;
        listing.updated_ledger = env.ledger().sequence();
        save_listing(&env, &listing);
        publish_updated(&env, &listing);
        Ok(listing)
    }

    /// Delete a listing. Only its seller can do this. The id becomes free again.
    pub fn remove(env: Env, id: String) -> Result<(), Error> {
        let listing = load_listing(&env, &id)?;
        listing.seller.require_auth();
        delete_listing(&env, &listing);
        Removed {
            id,
            by: listing.seller,
        }
        .publish(&env);
        Ok(())
    }

    // ----- reads -----

    pub fn get(env: Env, id: String) -> Result<Listing, Error> {
        load_listing(&env, &id)
    }

    /// Number of listings in the registry.
    pub fn count(env: Env) -> u32 {
        count(&env)
    }

    /// Page through all listings: `start` is a position, `limit` is capped at 50.
    /// Order is stable except that removing a listing moves the last one into its place.
    pub fn list(env: Env, start: u32, limit: u32) -> Vec<Listing> {
        let total = count(&env);
        let end = start.saturating_add(limit.min(MAX_PAGE_SIZE)).min(total);
        let mut out = Vec::new(&env);
        let mut i = start;
        while i < end {
            if let Some(id) = get_persistent::<String>(&env, &DataKey::IdAt(i)) {
                if let Ok(listing) = load_listing(&env, &id) {
                    out.push_back(listing);
                }
            }
            i += 1;
        }
        out
    }

    /// Ids of every listing a seller owns.
    pub fn listings_of(env: Env, seller: Address) -> Vec<String> {
        seller_ids(&env, &seller)
    }

    pub fn admin(env: Env) -> Address {
        admin(&env)
    }

    // ----- admin -----

    /// Remove an abusive or broken listing.
    pub fn admin_remove(env: Env, id: String) -> Result<(), Error> {
        let admin = admin(&env);
        admin.require_auth();
        let listing = load_listing(&env, &id)?;
        delete_listing(&env, &listing);
        Removed { id, by: admin }.publish(&env);
        Ok(())
    }

    pub fn set_admin(env: Env, new_admin: Address) {
        admin(&env).require_auth();
        env.storage().instance().set(&DataKey::Admin, &new_admin);
        bump_instance(&env);
    }

    /// Replace the contract code, keeping all stored listings.
    pub fn upgrade(env: Env, new_wasm_hash: BytesN<32>) {
        admin(&env).require_auth();
        env.deployer()
            .update_current_contract(ContractExecutable::Wasm(new_wasm_hash));
    }
}

// ---------- validation ----------

/// Lowercase letters, digits or "-", 2-63 chars, not starting with "-".
/// Same rule as the gateway's listings.json, so ids work in /v1/call/<id>.
fn check_id(id: &String) -> Result<(), Error> {
    let len = id.len();
    if !(ID_MIN_LEN..=ID_MAX_LEN).contains(&len) {
        return Err(Error::InvalidId);
    }
    let mut buf = [0u8; ID_MAX_LEN as usize];
    let bytes = &mut buf[..len as usize];
    id.copy_into_slice(bytes);
    for (i, c) in bytes.iter().enumerate() {
        let ok = c.is_ascii_lowercase() || c.is_ascii_digit() || (*c == b'-' && i > 0);
        if !ok {
            return Err(Error::InvalidId);
        }
    }
    Ok(())
}

fn check_input(input: &ListingInput) -> Result<(), Error> {
    if input.price <= 0 {
        return Err(Error::InvalidPrice);
    }
    if !is_filled(&input.name, NAME_MAX_LEN) {
        return Err(Error::InvalidName);
    }
    if !is_filled(&input.description, DESCRIPTION_MAX_LEN) {
        return Err(Error::InvalidDescription);
    }
    if input.category.len() > CATEGORY_MAX_LEN {
        return Err(Error::InvalidCategory);
    }
    check_endpoint(&input.endpoint)
}

/// Non-empty, not only spaces, and at most `max` bytes.
fn is_filled(s: &String, max: u32) -> bool {
    let len = s.len();
    if len == 0 || len > max {
        return false;
    }
    let mut buf = [0u8; DESCRIPTION_MAX_LEN as usize];
    let bytes = &mut buf[..len as usize];
    s.copy_into_slice(bytes);
    bytes.iter().any(|c| !c.is_ascii_whitespace())
}

/// Must be an http(s) URL with something after the scheme, and no spaces.
fn check_endpoint(endpoint: &String) -> Result<(), Error> {
    let len = endpoint.len();
    if len > ENDPOINT_MAX_LEN {
        return Err(Error::InvalidEndpoint);
    }
    let mut buf = [0u8; ENDPOINT_MAX_LEN as usize];
    let bytes = &mut buf[..len as usize];
    endpoint.copy_into_slice(bytes);

    let rest = if let Some(rest) = bytes.strip_prefix(b"https://") {
        rest
    } else if let Some(rest) = bytes.strip_prefix(b"http://") {
        rest
    } else {
        return Err(Error::InvalidEndpoint);
    };
    if rest.is_empty() || rest[0] == b'/' || rest.iter().any(|c| c.is_ascii_whitespace()) {
        return Err(Error::InvalidEndpoint);
    }
    Ok(())
}

// ---------- storage helpers ----------

fn admin(env: &Env) -> Address {
    env.storage().instance().get(&DataKey::Admin).unwrap()
}

fn count(env: &Env) -> u32 {
    env.storage().instance().get(&DataKey::Count).unwrap_or(0)
}

fn bump_instance(env: &Env) {
    env.storage()
        .instance()
        .extend_ttl(TTL_THRESHOLD, TTL_EXTEND_TO);
}

fn set_persistent<V: soroban_sdk::IntoVal<Env, soroban_sdk::Val>>(
    env: &Env,
    key: &DataKey,
    value: &V,
) {
    let storage = env.storage().persistent();
    storage.set(key, value);
    storage.extend_ttl(key, TTL_THRESHOLD, TTL_EXTEND_TO);
}

fn get_persistent<V: soroban_sdk::TryFromVal<Env, soroban_sdk::Val>>(
    env: &Env,
    key: &DataKey,
) -> Option<V> {
    let storage = env.storage().persistent();
    let value = storage.get(key);
    if value.is_some() {
        storage.extend_ttl(key, TTL_THRESHOLD, TTL_EXTEND_TO);
    }
    value
}

fn load_listing(env: &Env, id: &String) -> Result<Listing, Error> {
    get_persistent(env, &DataKey::Listing(id.clone())).ok_or(Error::NotFound)
}

fn save_listing(env: &Env, listing: &Listing) {
    set_persistent(env, &DataKey::Listing(listing.id.clone()), listing);
}

fn seller_ids(env: &Env, seller: &Address) -> Vec<String> {
    get_persistent(env, &DataKey::Seller(seller.clone())).unwrap_or_else(|| Vec::new(env))
}

fn publish_updated(env: &Env, listing: &Listing) {
    Updated {
        id: listing.id.clone(),
        price: listing.price,
        pay_to: listing.pay_to.clone(),
        active: listing.active,
    }
    .publish(env);
}

/// Remove a listing and keep both indexes consistent.
fn delete_listing(env: &Env, listing: &Listing) {
    let storage = env.storage().persistent();
    let id = &listing.id;

    // Global index: move the last entry into the freed slot.
    let total = count(env);
    let pos_key = DataKey::Position(id.clone());
    if let Some(pos) = storage.get::<_, u32>(&pos_key) {
        let last = total - 1;
        if pos != last {
            let last_id: String = storage.get(&DataKey::IdAt(last)).unwrap();
            set_persistent(env, &DataKey::IdAt(pos), &last_id);
            set_persistent(env, &DataKey::Position(last_id), &pos);
        }
        storage.remove(&DataKey::IdAt(last));
        storage.remove(&pos_key);
        env.storage().instance().set(&DataKey::Count, &last);
    }

    // Seller index.
    let seller_key = DataKey::Seller(listing.seller.clone());
    let mut ids = seller_ids(env, &listing.seller);
    if let Some(i) = ids.first_index_of(id) {
        ids.remove(i);
    }
    if ids.is_empty() {
        storage.remove(&seller_key);
    } else {
        set_persistent(env, &seller_key, &ids);
    }

    storage.remove(&DataKey::Listing(id.clone()));
    bump_instance(env);
}
