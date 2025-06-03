# Match

# Syntax
`match x { ... }`
- Where `x` is any expression
- `...` is comma-seperated cases
`x -> y`
- `x` is any expression to test equality
- `y` is any expression which is ran on success
`compOp x -> y`
- Where `compOp` is any comparison operator
- `x` is any expression to test
- `y` is any expression which is ran on success
`else -> y`
- Same as above, but it's the "default" case
