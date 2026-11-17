#![cfg(test)]
extern crate std;

use super::*;
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::{Env, String};

struct Setup<'a> {
    env: Env,
    client: RegistryClient<'a>,
    admin: Address,
}

fn setup() -> Setup<'static> {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let contract_id = env.register(Registry, (&admin,));
    let client = RegistryClient::new(&env, &contract_id);
    Setup { env, client, admin }
}

fn s(env: &Env, v: &str) -> String {
    String::from_str(env, v)
}

fn input(env: &Env, price: i128) -> ListingInput {
    ListingInput {
        name: s(env, "Market prices"),
        description: s(env, "Staple-food prices for Nigerian cities."),
        endpoint: s(env, "https://gw.example.com/v1/call/market-prices"),
        price,
        pay_to: Address::generate(env),
        category: s(env, "data"),
    }
}

#[test]
fn register_and_read() {
    let t = setup();
    let seller = Address::generate(&t.env);
    let id = s(&t.env, "market-prices");
    let inp = input(&t.env, 50_000);

    let listing = t.client.register(&seller, &id, &inp);
    assert_eq!(listing.id, id);
    assert_eq!(listing.seller, seller);
    assert_eq!(listing.pay_to, inp.pay_to);
    assert_eq!(listing.price, 50_000);
    assert!(listing.active);

    // The seller signed the call.
    let auths = t.env.auths();
    assert_eq!(auths.len(), 1);
    assert_eq!(auths[0].0, seller);

    assert_eq!(t.client.get(&id), listing);
    assert_eq!(t.client.count(), 1);
    assert_eq!(t.client.list(&0, &10), soroban_sdk::vec![&t.env, listing]);
    assert_eq!(
        t.client.listings_of(&seller),
        soroban_sdk::vec![&t.env, id]
    );
    assert_eq!(t.client.admin(), t.admin);
}

#[test]
fn duplicate_id_is_rejected() {
    let t = setup();
    let id = s(&t.env, "fx");
    t.client
        .register(&Address::generate(&t.env), &id, &input(&t.env, 10_000));
    let res = t
        .client
        .try_register(&Address::generate(&t.env), &id, &input(&t.env, 10_000));
    assert_eq!(res, Err(Ok(Error::AlreadyExists)));
}

#[test]
fn invalid_ids_are_rejected() {
    let t = setup();
    let seller = Address::generate(&t.env);
    let too_long = "a".repeat(64);
    for bad in ["a", "-fx", "FX", "fx rates", "fx_rates", "fx/rates", too_long.as_str()] {
        let res = t
            .client
            .try_register(&seller, &s(&t.env, bad), &input(&t.env, 1));
        assert_eq!(res, Err(Ok(Error::InvalidId)), "id {bad:?}");
    }
    // Longest allowed id works.
    let longest = "a".repeat(63);
    t.client
        .register(&seller, &s(&t.env, &longest), &input(&t.env, 1));
}

#[test]
fn invalid_fields_are_rejected() {
    let t = setup();
    let seller = Address::generate(&t.env);
    let id = s(&t.env, "api");

    let check = |inp: ListingInput, err: Error| {
        assert_eq!(t.client.try_register(&seller, &id, &inp), Err(Ok(err)));
    };

    check(input(&t.env, 0), Error::InvalidPrice);
    check(input(&t.env, -5), Error::InvalidPrice);

    let mut inp = input(&t.env, 1);
    inp.name = s(&t.env, "   ");
    check(inp, Error::InvalidName);

    let mut inp = input(&t.env, 1);
    inp.name = s(&t.env, &"n".repeat(81));
    check(inp, Error::InvalidName);

    let mut inp = input(&t.env, 1);
    inp.description = s(&t.env, "");
    check(inp, Error::InvalidDescription);

    let mut inp = input(&t.env, 1);
    inp.description = s(&t.env, &"d".repeat(501));
    check(inp, Error::InvalidDescription);

    let mut inp = input(&t.env, 1);
    inp.category = s(&t.env, &"c".repeat(33));
    check(inp, Error::InvalidCategory);

    let long_url = std::format!("https://{}", "x".repeat(250));
    for bad in [
        "",
        "ftp://example.com",
        "example.com",
        "https://",
        "https:///path",
        "https://exa mple.com",
        long_url.as_str(),
    ] {
        let mut inp = input(&t.env, 1);
        inp.endpoint = s(&t.env, bad);
        check(inp, Error::InvalidEndpoint);
    }

    // Empty category and plain http are fine.
    let mut inp = input(&t.env, 1);
    inp.category = s(&t.env, "");
    inp.endpoint = s(&t.env, "http://localhost:4021/v1/call/api");
    t.client.register(&seller, &id, &inp);
}

#[test]
fn seller_can_update_and_pause() {
    let t = setup();
    let seller = Address::generate(&t.env);
    let id = s(&t.env, "fx");
    let created = t.client.register(&seller, &id, &input(&t.env, 10_000));

    t.env.ledger().with_mut(|l| l.sequence_number += 100);

    let mut inp = input(&t.env, 20_000);
    inp.name = s(&t.env, "Naira FX feed");
    let updated = t.client.update(&id, &inp);
    assert_eq!(t.env.auths()[0].0, seller);
    assert_eq!(updated.price, 20_000);
    assert_eq!(updated.name, inp.name);
    assert_eq!(updated.pay_to, inp.pay_to);
    assert_eq!(updated.seller, seller);
    assert_eq!(updated.created_ledger, created.created_ledger);
    assert_eq!(updated.updated_ledger, created.created_ledger + 100);

    let paused = t.client.set_active(&id, &false);
    assert_eq!(t.env.auths()[0].0, seller);
    assert!(!paused.active);
    assert!(!t.client.get(&id).active);
    assert!(t.client.set_active(&id, &true).active);
}

#[test]
fn update_rejects_bad_input_and_unknown_id() {
    let t = setup();
    let id = s(&t.env, "fx");
    t.client
        .register(&Address::generate(&t.env), &id, &input(&t.env, 1));
    assert_eq!(
        t.client.try_update(&id, &input(&t.env, 0)),
        Err(Ok(Error::InvalidPrice))
    );
    let missing = s(&t.env, "missing");
    assert_eq!(
        t.client.try_update(&missing, &input(&t.env, 1)),
        Err(Ok(Error::NotFound))
    );
    assert_eq!(
        t.client.try_set_active(&missing, &false),
        Err(Ok(Error::NotFound))
    );
    assert_eq!(t.client.try_remove(&missing), Err(Ok(Error::NotFound)));
    assert_eq!(t.client.try_get(&missing), Err(Ok(Error::NotFound)));
}

#[test]
fn changes_need_the_sellers_signature() {
    let t = setup();
    let id = s(&t.env, "fx");
    t.client
        .register(&Address::generate(&t.env), &id, &input(&t.env, 1));

    // Stop mocking auth: nobody has signed now.
    t.env.set_auths(&[]);
    assert!(t.client.try_update(&id, &input(&t.env, 2)).is_err());
    assert!(t.client.try_set_active(&id, &false).is_err());
    assert!(t.client.try_remove(&id).is_err());
    assert!(t.client.try_admin_remove(&id).is_err());
    assert_eq!(t.client.get(&id).price, 1);
}

#[test]
fn remove_keeps_indexes_consistent() {
    let t = setup();
    let alice = Address::generate(&t.env);
    let bob = Address::generate(&t.env);
    let (a, b, c) = (s(&t.env, "aa"), s(&t.env, "bb"), s(&t.env, "cc"));
    t.client.register(&alice, &a, &input(&t.env, 1));
    t.client.register(&bob, &b, &input(&t.env, 2));
    t.client.register(&alice, &c, &input(&t.env, 3));

    t.client.remove(&a);
    assert_eq!(t.env.auths()[0].0, alice);
    assert_eq!(t.client.count(), 2);
    assert_eq!(t.client.try_get(&a), Err(Ok(Error::NotFound)));

    // "cc" moved into the freed first slot.
    let ids: std::vec::Vec<String> = t.client.list(&0, &10).iter().map(|l| l.id).collect();
    assert_eq!(ids, std::vec![c.clone(), b.clone()]);
    assert_eq!(t.client.listings_of(&alice), soroban_sdk::vec![&t.env, c.clone()]);

    // Removing the last entry, then the only remaining one of a seller.
    t.client.remove(&b);
    assert_eq!(t.client.listings_of(&bob).len(), 0);
    t.client.remove(&c);
    assert_eq!(t.client.count(), 0);
    assert_eq!(t.client.list(&0, &10).len(), 0);

    // The id is free again.
    t.client.register(&bob, &a, &input(&t.env, 4));
    assert_eq!(t.client.get(&a).seller, bob);
}

#[test]
fn list_pages_through_listings() {
    let t = setup();
    let seller = Address::generate(&t.env);
    for i in 0..5 {
        let id = std::format!("api-{i}");
        t.client.register(&seller, &s(&t.env, &id), &input(&t.env, 1));
    }
    assert_eq!(t.client.list(&0, &2).len(), 2);
    assert_eq!(t.client.list(&4, &2).len(), 1);
    assert_eq!(t.client.list(&5, &2).len(), 0);
    assert_eq!(t.client.list(&2, &u32::MAX).len(), 3);
    assert_eq!(t.client.list(&0, &0).len(), 0);
}

#[test]
fn list_is_capped_at_page_size() {
    let t = setup();
    for i in 0..(MAX_PAGE_SIZE + 5) {
        // Spread over sellers to stay under the per-seller cap.
        let seller = Address::generate(&t.env);
        let id = std::format!("api-{i}");
        t.client.register(&seller, &s(&t.env, &id), &input(&t.env, 1));
    }
    assert_eq!(t.client.list(&0, &100).len(), MAX_PAGE_SIZE);
}

#[test]
fn seller_listing_cap() {
    let t = setup();
    let seller = Address::generate(&t.env);
    for i in 0..MAX_LISTINGS_PER_SELLER {
        let id = std::format!("api-{i}");
        t.client.register(&seller, &s(&t.env, &id), &input(&t.env, 1));
    }
    let res = t
        .client
        .try_register(&seller, &s(&t.env, "one-more"), &input(&t.env, 1));
    assert_eq!(res, Err(Ok(Error::TooManyListings)));

    // Freeing a slot lets them register again.
    t.client.remove(&s(&t.env, "api-0"));
    t.client
        .register(&seller, &s(&t.env, "one-more"), &input(&t.env, 1));
}

#[test]
fn admin_can_moderate_and_hand_over() {
    let t = setup();
    let id = s(&t.env, "spam");
    t.client
        .register(&Address::generate(&t.env), &id, &input(&t.env, 1));

    t.client.admin_remove(&id);
    assert_eq!(t.env.auths()[0].0, t.admin);
    assert_eq!(t.client.count(), 0);

    let new_admin = Address::generate(&t.env);
    t.client.set_admin(&new_admin);
    assert_eq!(t.env.auths()[0].0, t.admin);
    assert_eq!(t.client.admin(), new_admin);
}
