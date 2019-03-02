// This test file uses the tape testing framework.
// To learn more, go here: https://github.com/substack/tape
const test = require('tape');

const { Config, Container } = require('@holochain/holochain-nodejs')

const dnaPath = "dist/bundle.json"

// IIFE to keep config-only stuff out of test scope
const container = (() => {
  const agentAlice = Config.agent("alice")

  const dna = Config.dna(dnaPath)

  const instanceAlice = Config.instance(agentAlice, dna)

  const containerConfig = Config.container([instanceAlice])
  return new Container(containerConfig)
})()

// Initialize the Container
container.start()

const app = container.makeCaller('alice', dnaPath)

const mockProduct = {
  name: "test product",
  description: 'yummi',
  price: 5.31
}

const mockBasket = {
  name: "Test",
  sum: 0
}

const mockPosition = {
  amount: 5,
  timestamp: Date.now().toString()
}

test('Can create a product', (t) => {
  const create_result = app.call("pos", "main", "create_product", {product: mockProduct})
  console.log(create_result)
  t.notEqual(create_result.Ok, undefined)
  t.end()
})

test('Can create a basket', (t) => {
  const create_result = app.call("pos", "main", "create_basket", {basket: mockBasket})
  console.log(create_result)
  t.notEqual(create_result.Ok, undefined)
  t.end()
})

test('Can add some positions', (t) => {
  const create_product_result = app.call("pos", "main", "create_product", {product: mockProduct})
  const product_addr = create_product_result.Ok
  const create_basket_result = app.call("pos", "main", "create_basket", {basket: mockBasket})
  const basket_addr = create_basket_result.Ok

  const result1 = app.call("pos", "main", "add_product", {product_addr: product_addr, basket_addr: basket_addr, position: mockPosition})
  const result2 = app.call("pos", "main", "add_product", {product_addr: product_addr, basket_addr: basket_addr, position: {amount: 2, timestamp: Date.now().toString()}})
  console.log(result1)

  t.notEqual(result1.Ok, undefined)
  t.notEqual(result2.Ok, undefined)

  t.end()
})

test('Can get a basket with positions', (t) => {
  const create_product_result = app.call("pos", "main", "create_product", {product: mockProduct})
  const product_addr = create_product_result.Ok
  const create_basket_result = app.call("pos", "main", "create_basket", {basket: mockBasket})
  const basket_addr = create_basket_result.Ok

  const result1 = app.call("pos", "main", "add_product", {product_addr: product_addr, basket_addr: basket_addr, position: mockPosition})
  const result2 = app.call("pos", "main", "add_product", {product_addr: product_addr, basket_addr: basket_addr, position: {amount: 2, timestamp: Date.now().toString()}})

  const get_result = app.call("pos", "main", "get_basket", {basket_addr: basket_addr})
  console.log(get_result.Ok.positions)

  t.equal(get_result.Ok.positions.length, 2, "there should be 2 items in the list")
  t.end()
})