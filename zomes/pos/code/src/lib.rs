#![feature(try_from)]
use std::convert::TryFrom;

#[macro_use]
extern crate hdk;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate holochain_core_types_derive;

use std::time::{SystemTime, UNIX_EPOCH};

use hdk::{
  error::{ZomeApiResult, ZomeApiError},
  holochain_core_types::{
    hash::HashString,
    error::HolochainError,
    entry::{AppEntryValue,Entry},
    dna::entry_types::Sharing,
    json::JsonString,
    cas::content::Address
  },
  api::update_entry
};

#[derive(Serialize, Deserialize, Debug, DefaultJson)]
pub struct Product {
  pub name: String,
  pub description: String,
  pub price: f32,
}

#[derive(Serialize, Deserialize, Debug, DefaultJson)]
pub struct Position {
  pub amount: i8,
  pub timestamp: String,
}

#[derive(Serialize, Deserialize, Debug, DefaultJson)]
pub struct Basket {
  pub name: String,
  pub sum: f32,
}

#[derive(Serialize, Deserialize, Debug, DefaultJson)]
pub struct BasketResponse {
  pub id: HashString,
  pub name: String,
  pub sum: f32,
  pub product_positions: Vec<PositionWithProduct>,
}

#[derive(Serialize, Deserialize, Debug, DefaultJson)]
pub struct BasketResponseAll {
  pub name: String,
  pub sum: f32,
  pub id: HashString
}

#[derive(Serialize, Deserialize, Debug, DefaultJson)]
pub struct ProductResponse {
  pub id: HashString,
  pub name: String,
  pub description: String,
  pub price: f32,
}

#[derive(Serialize, Deserialize, Debug, DefaultJson)]
pub struct PositionWithProduct {
  pub amount: i8,
  pub product: Product
}

#[derive(Serialize, Deserialize, Debug, DefaultJson)]
pub struct Table {
  pub name: String
}

define_zome! {
  entries: [
    entry!(
      name: "product",
      description: "products that are offered in a facility",
      sharing: Sharing::Public,
      native_type: Product,
      validation_package: || {
        hdk::ValidationPackageDefinition::Entry
      },
      validation: |_product: Product, _ctx: hdk::ValidationData| {
        Ok(())
      }
    ),
    entry!(
      name: "basket",
      description: "basket holds all items per costumer",
      sharing: Sharing::Public,
      native_type: Basket,
      validation_package: || {
        hdk::ValidationPackageDefinition::Entry
      },
      validation: |_basket: Basket, _ctx: hdk::ValidationData| {
        Ok(())
      },
      links: [
        to!(
          "position",
          tag: "positions",
          validation_package: || {
            hdk::ValidationPackageDefinition::Entry
          },
          validation: |base: Address, target: Address, _ctx: hdk::ValidationData| {
            Ok(())
          }
        )
      ]
    ),
    entry!(
      name: "position",
      description: "represents the product position in a basket",
      sharing: Sharing::Public,
      native_type: Position,
      validation_package: || {
        hdk::ValidationPackageDefinition::Entry
      },
      validation: |_position: Position, _ctx: hdk::ValidationData| {
        Ok(())
      },
      links: [
        to!(
          "product",
          tag: "product",
          validation_package: || {
            hdk::ValidationPackageDefinition::Entry
          },
          validation: |base: Address, target: Address, _ctx: hdk::ValidationData| {
            Ok(())
          }
        )
      ]
    )
  ]

  genesis: || { Ok(()) }

  functions: {
    main (Public) {
      create_product: {
        inputs: |product: Product|,
        outputs: |result: ZomeApiResult<Address>|,
        handler: handle_create_product
      }
      create_basket: {
        inputs: |basket: Basket|,
        outputs: |result: ZomeApiResult<Address>|,
        handler: handle_create_basket
      }
      add_product: {
        inputs: |product_addr: HashString, basket_addr: HashString, position: Position|,
        outputs: |result: ZomeApiResult<BasketResponse>|,
        handler: handle_add_product
      }
      get_basket: {
        inputs: |basket_addr: HashString|,
        outputs: |result: ZomeApiResult<BasketResponse>|,
        handler: handle_get_basket
      }
      get_products: {
        inputs: |  |,
        outputs: |result: Vec<ProductResponse>|,
        handler: handle_get_products
      }
      get_product: {
        inputs: |product_addr: HashString|,
        outputs: |result: ZomeApiResult<ProductResponse>|,
        handler: handle_get_product
      }
      get_baskets: {
        inputs: | |,
        outputs: |result: Vec<BasketResponseAll>|,
        handler: handle_get_baskets
      }
      init_mock_data: {
        inputs: |products: Vec<Product>, baskets: Vec<Basket>, positions: Vec<Position>|,
        outputs: |result: Vec<Address>|,
        handler: handle_init_mock_data
      }
    }
  }
}

fn handle_init_mock_data(products: Vec<Product>, baskets: Vec<Basket>, positions: Vec<Position>) -> Vec<Address>{
  let product_addresses = products
    .iter()
    .map(|product| {
      let product_entry = Entry::App("product".into(), product.into());
      hdk::commit_entry(&product_entry)
    })
    .filter_map(Result::ok)
    .collect::<Vec<Address>>();

  let basket_addresses = baskets
    .iter()
    .map(|basket| {
      let basket_entry = Entry::App("basket".into(), basket.into());
      hdk::commit_entry(&basket_entry)
    })
    .filter_map(Result::ok)
    .collect::<Vec<Address>>();

  basket_addresses
}

fn handle_create_product(product: Product) -> ZomeApiResult<Address> {
  let product_entry = Entry::App("product".into(), product.into());
  hdk::commit_entry(&product_entry)
}

fn handle_create_basket(basket: Basket) -> ZomeApiResult<Address> {
  let basket_entry = Entry::App("basket".into(), basket.into());
  hdk::commit_entry(&basket_entry)
}

fn handle_add_product(product_addr: HashString, basket_addr: HashString, position: Position) -> ZomeApiResult<BasketResponse> {
  let position_entry = Entry::App("position".into(), position.into());
  let position_addr = hdk::commit_entry(&position_entry)?;
  hdk::link_entries(&basket_addr, &position_addr, "positions")?;
  hdk::link_entries(&position_addr, &product_addr, "product")?;
  update_basket(&basket_addr, &product_addr, &position_addr);
  handle_get_basket(basket_addr)
}

fn handle_get_basket(basket_addr: HashString) -> ZomeApiResult<BasketResponse> {

  let basket = get_as_type::<Basket>(basket_addr.clone())?;

  let positions_with_products = hdk::get_links(&basket_addr, "positions")?.addresses()
    .iter()
    .map(|position_address| {
      let position = get_as_type::<Position>(position_address.to_owned()).unwrap();
      let product_addresses_result = hdk::get_links(&position_address, "product").unwrap();
      let product_addresses = product_addresses_result.addresses();
      let product_address = product_addresses.first().unwrap();
      let product = get_as_type::<Product>(product_address.to_owned().clone()).unwrap();
      PositionWithProduct {
        amount: position.amount,
        product: product,
      }
    })
    .collect::<Vec<PositionWithProduct>>();

  Ok(BasketResponse{
    name: basket.name,
    id: basket_addr,
    sum: basket.sum,
    product_positions: positions_with_products
  })
}

fn handle_get_baskets() -> Vec<BasketResponseAll> {
  let mut results = hdk::query("basket".into(), 0, 0).unwrap();
  results.dedup();
  results.iter()
    .map(|basket_address| {
      let basket = get_as_type::<Basket>(basket_address.to_owned()).unwrap();
      BasketResponseAll {
        name: basket.name,
        sum: basket.sum,
        id: basket_address.to_owned()
      }
    })
    .collect::<Vec<BasketResponseAll>>()
}

pub fn handle_get_products() -> Vec<ProductResponse> {
  let mut results = hdk::query("product".into(), 0, 0).unwrap();
  results.dedup();
  results.iter()
    .map(|product_address| {
      let product = get_as_type::<Product>(product_address.to_owned()).unwrap();
      ProductResponse {
        name: product.name,
        description: product.description,
        price: product.price,
        id: product_address.to_owned()
      }
    })
    .collect::<Vec<ProductResponse>>()
}

fn handle_get_product(product_addr: HashString) -> ZomeApiResult<ProductResponse> {
  let product = get_as_type::<Product>(product_addr.clone())?;
  Ok(ProductResponse {
    id: product_addr,
    name: product.name,
    description: product.description,
    price: product.price
  })
}

pub fn update_basket(basket_addr: &HashString, product_addr: &HashString, position_addr: &HashString) -> ZomeApiResult<Address>{
  let mut basket = get_as_type::<Basket>(basket_addr.clone())?;
  let product = get_as_type::<Product>(product_addr.clone())?;
  let position = get_as_type::<Position>(position_addr.clone())?;
  basket.sum += product.price * position.amount as f32;
  update_entry(Entry::App("basket".into(),basket.into()),&basket_addr)
}

pub fn get_as_type<R: TryFrom<AppEntryValue>>(address: HashString) -> ZomeApiResult<R> {
  let get_result = hdk::get_entry(&address)?;
  let entry = get_result.ok_or(ZomeApiError::Internal("No entry at this address".into()))?;
  match entry {
    Entry::App(_, entry_value) => {
      R::try_from(entry_value.to_owned())
        .map_err(|_| ZomeApiError::Internal(
          "Could not convert getlinks result to requested type".to_string()
        ))
    },
    _ => Err(ZomeApiError::Internal(
      "get_links did not return an app entry".to_string()
    ))
  }
}
