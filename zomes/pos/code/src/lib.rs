#![feature(try_from)]
use std::convert::TryFrom;

#[macro_use]
extern crate hdk;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate holochain_core_types_derive;

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
  pub price: String,
}

#[derive(Serialize, Deserialize, Debug, DefaultJson)]
pub struct Position {
  pub amount: i8,
}

#[derive(Serialize, Deserialize, Debug, DefaultJson)]
pub struct Basket {
  pub sum: f32,
}

#[derive(Serialize, Deserialize, Debug, DefaultJson)]
pub struct BasketResponse {
  pub sum: f32,
  pub positions: Vec<Position>,
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
            hdk::ValidationPackageDefinition::ChainFull
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
            hdk::ValidationPackageDefinition::ChainFull
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
        outputs: |result: ZomeApiResult<HashString>|,
        handler: handle_add_product
      }
      get_basket: {
        inputs: |basket_addr: HashString|,
        outputs: |result: ZomeApiResult<BasketResponse>|,
        handler: handle_get_basket
      }
      get_products: {
        inputs: |product: Product|,
        outputs: |result: Vec<Product>|,
        handler: handle_get_products
      }
    }
  }
}

fn handle_create_product(product: Product) -> ZomeApiResult<Address> {
  let product_entry = Entry::App("product".into(), product.into());
  hdk::commit_entry(&product_entry)
}

fn handle_create_basket(basket: Basket) -> ZomeApiResult<Address> {
  let basket_entry = Entry::App("basket".into(), basket.into());
  hdk::commit_entry(&basket_entry)
}

fn handle_add_product(product_addr: HashString, basket_addr: HashString, position: Position) -> ZomeApiResult<Address> {
  let position_entry = Entry::App("position".into(), position.into());
  let position_addr = hdk::commit_entry(&position_entry)?;
  // since update_entry seems to be not supported yet
  // update_basket(&basket_addr,&product_addr,&position_addr)?;
  hdk::link_entries(&basket_addr, &position_addr, "positions")?;
  hdk::link_entries(&position_addr, &product_addr, "product")?;
  Ok(position_addr)
}

fn handle_get_basket(basket_addr: HashString) -> ZomeApiResult<BasketResponse> {

  let basket = get_as_type::<Basket>(basket_addr.clone())?;

  let positions = hdk::get_links(&basket_addr, "positions")?.addresses()
    .iter()
    .map(|position_address| {
      get_as_type::<Position>(position_address.to_owned())
    })
    .filter_map(Result::ok)
    .collect::<Vec<Position>>();

  Ok(BasketResponse{
    sum: basket.sum,
    positions: positions
  })
}

pub fn handle_get_products(product: Product) -> Vec<Product> {
  hdk::query("product".into(), 0, 0).unwrap()
    .iter()
    .map(|product_address| {
      get_as_type::<Product>(product_address.to_owned())
    })
    .filter_map(Result::ok)
    .collect::<Vec<Product>>()
}

pub fn update_basket(basket_addr: &HashString, product_addr: &HashString, position_addr: &HashString) -> ZomeApiResult<Address>{
  let mut basket = get_as_type::<Basket>(basket_addr.clone())?;
  let product = get_as_type::<Product>(product_addr.clone())?;
  let position = get_as_type::<Position>(position_addr.clone())?;
  update_entry(Entry::App("basket".into(),basket.into()),&basket_addr)
}

pub fn get_as_type<R: TryFrom<AppEntryValue>>(address: HashString) -> ZomeApiResult<R> {
  let get_result = hdk::get_entry(&address)?;
  let entry = get_result.ok_or(ZomeApiError::Internal("No entry at this address".into()))?;
  match entry {
    Entry::App(_, entry_value) => {
      R::try_from(entry_value.to_owned())
        .map_err(|_| ZomeApiError::Internal(
          "Could not convert get)links result tot requested type".to_string()
        ))
    },
    _ => Err(ZomeApiError::Internal(
      "get_links did not return an app entry".to_string()
    ))
  }
}
