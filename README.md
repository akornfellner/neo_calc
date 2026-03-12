# neo_calc — Neo Calculator

> **Note:** This is not a production project — it's my first experiment with [Leptos](https://github.com/leptos-rs/leptos) and is intended purely for learning purposes.
>
> ⚠️ **Math Logic Disclaimer:** The core math logic was implemented using "vibe coding" and has not been fully reviewed. Use at your own risk.
>
A Matrix-themed calculator web app built with [Leptos](https://github.com/leptos-rs/leptos) (Rust → WASM, client-side rendered).

## Features

### Expression Input

Type any mathematical expression and see the result update in real time. The
input field accepts a single expression using the operators, functions and
constants listed below.

### Operators

| Operator | Description                   | Example  | Result    |
| -------- | ----------------------------- | -------- | --------- |
| `+`      | Addition                      | `2 + 3`  | `5`       |
| `-`      | Subtraction / negation        | `10 - 3` | `7`       |
| `*`      | Multiplication                | `4 * 5`  | `20`      |
| `/`      | Division                      | `15 / 3` | `5`       |
| `^`      | Exponentiation (right-assoc.) | `2^10`   | `1024`    |
| `!`      | Factorial (postfix)           | `5!`     | `120`     |
| `°`      | Degree-to-radian (postfix)    | `90°`    | `1.5707…` |

Operator precedence (highest to lowest): `!` / `°` → `^` → `*` `/` → `+` `-`.

The `°` symbol converts a value from degrees to radians (`x° = x * π / 180`).
It can be applied to any value or sub-expression, including function arguments:

| Expression  | Equivalent         | Result    |
| ----------- | ------------------ | --------- |
| `90°`       | `90 * pi / 180`    | `1.5707…` |
| `sin(45°)`  | `sin(45 * pi/180)` | `0.7071…` |
| `cos(180°)` | `cos(pi)`          | `-1`      |

### Built-in Functions

| Function   | Description       | Example                  | Result    |
| ---------- | ----------------- | ------------------------ | --------- |
| `sin(x)`   | Sine (radians)    | `sin(pi/2)` / `sin(90°)` | `1`       |
| `cos(x)`   | Cosine (radians)  | `cos(0)` / `cos(0°)`     | `1`       |
| `tan(x)`   | Tangent (radians) | `tan(pi/4)` / `tan(45°)` | `1`       |
| `asin(x)`  | Inverse sine      | `asin(1)`                | `1.5707…` |
| `acos(x)`  | Inverse cosine    | `acos(1)`                | `0`       |
| `atan(x)`  | Inverse tangent   | `atan(1)`                | `0.7853…` |
| `sqrt(x)`  | Square root       | `sqrt(16)`               | `4`       |
| `abs(x)`   | Absolute value    | `abs(-7)`                | `7`       |
| `log(x)`   | Base-10 logarithm | `log(1000)`              | `3`       |
| `ln(x)`    | Natural logarithm | `ln(e)`                  | `1`       |
| `floor(x)` | Floor             | `floor(3.7)`             | `3`       |
| `ceil(x)`  | Ceiling           | `ceil(3.2)`              | `4`       |

### Built-in Constants

| Name | Value       |
| ---- | ----------- |
| `pi` | 3.14159265… |
| `e`  | 2.71828182… |

### Implicit Multiplication

Multiplication signs can be omitted where the intent is unambiguous:

| Expression | Equivalent   | Result    |
| ---------- | ------------ | --------- |
| `2pi`      | `2 * pi`     | `6.2831…` |
| `3(4+5)`   | `3 * (4+5)`  | `27`      |
| `(2)(3)`   | `(2) * (3)`  | `6`       |
| `2sin(0)`  | `2 * sin(0)` | `0`       |

### Variables

Store the current result into a named variable by clicking **Store** and typing
a name (letters, digits and `_`; must start with a letter). Variables can then
be referenced in later expressions:

```
> 100/3        → 33.3333…   [Store as "tax"]
> tax * 2      → 66.6666…
```

Variables are displayed in a table below the input. You can delete individual
variables or clear all of them at once.

### Plot Mode

Switch to the **Plot** tab to graph any expression that uses `x` as the free
variable.

- Type an expression such as `sin(x)`, `x^2 - 3x + 1`, or `abs(x)*cos(x)`.
- Adjust the **x min** / **x max** fields to control the viewing window
  (default: −10 to 10).
- The y-axis auto-scales to fit the computed values.
- Stored variables (other than `x`) are substituted into the expression, so
  `a*x^2` plots a parabola using the stored value of `a`.
- Points where the expression is undefined (e.g. division by zero) create gaps
  in the curve.
- The graph is rendered on an HTML `<canvas>` element using `web-sys`, with no
  external JS charting libraries.

## How It Works

The calculator uses a **recursive-descent parser** written from scratch in Rust.
The grammar is:

```
expr    = term (('+' | '-') term)*
term    = power (('*' | '/' | implicit) power)*
power   = postfix ('^' power)?          ← right-associative
postfix = factor ('!' | '°')*
factor  = NUMBER
        | IDENT '(' expr ')'            ← function call
        | IDENT                          ← constant / variable
        | '(' expr ')'
        | ('+' | '-') factor             ← unary
```

Implicit multiplication fires when a `power` is immediately followed by `(`,
a letter, `_`, or (after a closing paren) a digit — without an explicit
operator in between.

The Leptos framework provides **fine-grained reactivity**: signals and memos
ensure the expression is re-evaluated only when inputs or variables actually
change, keeping the UI snappy.

## Getting Started

### Prerequisites

- Rust toolchain (stable) with the `wasm32-unknown-unknown` target:

```bash
rustup target add wasm32-unknown-unknown
```

- [Trunk](https://trunkrs.dev/) for bundling and serving:

```bash
cargo install trunk
```

### Running the App

```bash
trunk serve --open
```

### Production Build

```bash
trunk build --release
```

### Running Tests

```bash
cargo test
```
