# If
- Used for running code based on another input
- These are expressions, meaning they can be used like any other expression

## Syntax
```
if ... { }
else if ... { }
else { }
```
- Where `...` is the test expression, if it evaluates to `true` the block will be ran, if it is `false` it either runs the `else` or returns null
- The part after `else` is either a `Block` or a `If`.

## Examples
```
if 1 == 1 { "yes" }
else { "no" }
```
```
let value = if false { 2 } else { 3 };
value; // 3;
```
