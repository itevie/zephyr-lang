# Try
- Used for attempting expressions, so they don't throw errors.
- There are two types of errors: "thrown" and "value".
  - Thrown errors should only be thrown by the interpreter, or in extremly dangerous circumstances
  - Results should always be returned by functions if that function can produce an error

# Syntax
`try x`
- Where `x` is any expression. This works in any situation, returns `null` if the expression an error.

`try x catch y`
- Returns `x` if it's sucessful, returns `y` if it is a failure.

`try x catch return`
- Provides the value of `x` if it's sucessful, otherwise, returns from the surrounding function with Result.Err

`try x catch y return z`
- Provides the value of `x` if it's sucessful, otherwise, runs the catch block.

All of the above work with blocks:
```
try { x }
catch y { return z; }
```

# Examples
`let a = try object.b catch 3;`
- `a` is set to `object.b`, unless it throws an error, such as `InvalidProperty`, then it will provide `3`
