# `tinyfilter`

`tinyfilter` is a extremely tiny filtering language.

It's like [expr-lang](https://github.com/expr-lang/expr) but safe for untrusted human input.

It was built for [backbeat](https://github.com/zkldi/backbeat)'s table folder language.

Add it to your repo with:

```sh
cargo add tinyfilter
```

and for javascript

```sh
npm add tinyfilter
# or whatever package manager the kids use nowadays
```

## Why?

I need a tiny, easily embeddable filter language for [backbeat](https://github.com/zkldi/backbeat) that is safe against untrusted user input.

It also needs a tiny surface area, as the spec for this is technically part of the backbeat spec.

It's unsafe to evaluate untrusted `expr-lang` inputs. For example, evaluating `1..9007199254740992` will let you instantly crash the host, and `expr-lang` has far too much of an implementation surface for something I want to be portable.

## Usage

In Rust:

```rs
fn main() {
	let mut cx = tinyfilter::Context::default();

    cx.insert("age", 24.0);
    cx.insert("name", "zk");
    cx.insert("hasID", false);

	match tinyfilter::evaluate("age >= 18 and hasID", &cx) {
		Ok(true) => println!("filter matched"),
		Ok(false) => println!("filter didn't match"),
		Err(err) => println!("the user's filter was invalid: {err}"),
	}
}
```

In Typescript:

```ts
import { evaluate } from "tinyfilter";

let result = evaluate("age >= 18 and hasID", {
	age: 24,
	name: "zk",
	hasID: false,
});
```

# Full Reference

## Syntax

Filters generally look like this:

```tnf
hasID and age > 18
```

A filter _must_ resolve to either `true` or `false`. An expression that doesn't resolve to a boolean will error.

There is no `let` or any way to create intermediate bindings.

You can do conditionals with `if/else`, if you want to use some extra lines.

```tnf
if age < 18 {
	false
} else {
	hasID
}
```

## Types

There are:

- `str`, which is UTF8 text. You can write a string literal with `"asdf"`.
- `num`, which is a 64 bit floating point number. You can define integers e.g. `123` or decimals `123.456`. You can also put `_` to separate integers up e.g. `1_000_000`.
- `bool`, which is true or false. You can write `true` or `false` to specify a boolean.
- `map`, which are key value pairs. You can index into a map with `map.value` or `map["value with funny name"]`. There is no map literal syntax.
- `nil`, which is produced from indexing into a map that doesn't have a value for that key.

There are no arrays/lists, and there is no syntax for a map literal. (by design, use expr-lang if you need powerful stuff)

## Operators

### `a + b`

Add two numbers together. There is no string concatenation.

- `5 + 5 == 10`
- `1.7 + 5 == 6.7`
- `"a" + "b"` (error)
- `5 + true` (error)

### `a - b`

Subtract two numbers.

- `5 - 5 == 0`
- `1.7 - 5 == -3.3`

### `a * b`

Multiply two numbers.

- `2 * 1.5 == 3`

### `a / b`

Divide two numbers. Use `floor(a / b)` for integer division.

- `5 / 2 == 2.5`
- `5 / 0` (error)

### `a % b`

Return the remainder of dividing `a` by `b`.

- `10 % 3 == 1`
- `10 % 3.1 == 0.7`
- `-5.5 % 2 == -1.5`

### `a ** b`

Return `a` to the power of `b`.

- `5 ** 3 == 125`
- `5 ** 2.5 == 55.90169943749474`
- `5 ** -1 == 1/5`

### `a == b`

Return whether a is equal to b. Errors if either side of argument is a `map`.

- `(5 == 5) == true`
- `(5 == 4) == false`
- `(5 == "5") == false`
- `("foo" == "foo") == true`

### `a != b`

Return whether a is not equal to b. Errors if either side of argument is a `map`

- `(5 != 4) == true`
- `(5 != 5) == false`
- `(5 != "5") == true`

### `a < b`, `a <= b`, `a > b`, `a >= b`

Return whether a is less than (`<`), less than or equal to (`<=`), greater than (`>`), greater than or equal to (`>=`) b.

- `5 > 4 == true`
- `5 > 5 == false`
- `5 >= 5 == true`
- `5.5 < 6 == true`
- `5.5 < 1 == false`
- `5.5 <= 5.5 == true`

### `not a`

Return the boolean opposite of `a`.

- `not true == false`
- `not false == true`
- `not 12` (error)

### `a and b`

Return true if `a` and `b` are true.

- `true and true == true`
- `true and false == false`
- `1 and true` (error)

### `a or b`

Return true if `a` or `b` are true.

- `true or false == true`
- `false or false == false`
- `1 or true` (error)

### `a otherwise b`

Returns `b` if `a` is null.

- `tags.difficulty otherwise 0 == 0` (assume that `tags.difficulty` is nil)
- `tags.isAwesome otherwise false == true` (assume that `tags.isAwesome` is true)
- `"a" otherwise "b"` (error)

### `exists a`

Returns `true` if `a` is not nil.

- `exists tags.difficulty == false`
- `exists tags.isAwesome == true`
- `not exists tags.difficulty == true`

## Functions

### `num(arg)`

Takes a string and converts it into a number. Floats get truncated.

- `num("14") == 14`
- `num("14.123") == 14.123`
- `num("a")` (error)
- `num("1e10")` (error)

### `type(arg)`

Takes anything and returns a string indicating what type it is.

- `type(5) == "num"`
- `type(5.5) == "num"`
- `type("hi") == "str"`
- `type(nil) == "nil"`
- `type(true) == "bool"`

### `startsWith(str, str)`

Returns whether the first string starts with the second string.

- `startsWith("md5/70924d6fa4b2d745185fa4660703a5c0", "md5/") == true`
- `startsWith("md5/70924d6fa4b2d745185fa4660703a5c0", "sha256/") == false`

### `contains(str, str)`

Returns whether the first string contains the second string.

- `contains("Blue Rain", "Rain") == true`
- `contains("Blue Rain", "Black") == false`

### `endsWith(str, str)`

Returns whether the first string ends with the second string.

- `endsWith("epic0", "0") == true`
- `endsWith("epic0", "1") == false`

### `abs(num)`

Return the absolute value of a number.

- `abs(-5.1) == 5.1`

### `ceil(num)`

Returns the number rounded up.

- `ceil(5.1) == 6`

### `floor(num)`

Returns the number rounded down.

- `floor(5.9) == 5`

### `round(num)`

Round the number to the nearest whole number.

- `round(5.1) == 5`
- `round(5.9) == 6`
- `round(5.5) == 6`

### `min(num, num)`

Return the smallest of two numbers.

- `min(9, 5.5) == 5.5`

### `max(num, num)`

Return the largest of two numbers.

- `max(9, 5.5) == 9`
